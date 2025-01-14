use std::env;
use std::sync::Arc;

use crate::models::activity::BlockActivity;
use crate::models::claim::{ClaimData, Signature};
use crate::models::deposit::DepositDocument;
use crate::models::runes::Operation;
use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use crate::utils::deposit::get_bitcoin_addr_from_starknet_addr;
use crate::utils::runes::symbol_as_felt;
use crate::utils::starknet::{convert_to_bigint, hex_to_uint256, to_uint256};
use crate::utils::Address;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use starknet::core::crypto::{ecdsa_sign, pedersen_hash, ExtendedSignature};
use starknet::core::types::FieldElement;

use super::responses::{ApiResponse, Status};

lazy_static::lazy_static! {
    static ref RUNES_BRIDGE_STARKNET_PRIV_KEY: FieldElement =   FieldElement::from_hex_be(&env::var("RUNES_BRIDGE_STARKNET_PRIV_KEY")
    .expect("RUNES_BRIDGE_STARKNET_PRIV_KEY must be set")).expect("Invalid RUNES_BRIDGE_STARKNET_PRIV_KEY");
    static ref HIRO_API_URL: String = env::var("HIRO_API_URL").expect("HIRO_API_URL must be set");
    static ref HIRO_API_KEY: String = env::var("HIRO_API_KEY").expect("HIRO_API_KEY must be set");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimDepositDataQuery {
    starknet_addr: Address,
    tx_id: String,
    tx_vout: Option<u64>,
}

#[route(post, "/claim_deposit_data")]
pub async fn claim_deposit_data(
    State(state): State<Arc<AppState>>,
    body: Json<ClaimDepositDataQuery>,
) -> impl IntoResponse {
    let mut session = try_start_session!(state);
    if let Err(err) = session.start_transaction().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            )),
        );
    };

    // Fetch BlockActivityResult
    let url = format!(
        "{}/runes/v1/transactions/{}/activity",
        *HIRO_API_URL, body.tx_id
    );
    let client = Client::new();
    let tx_data = match client
        .get(url)
        .header("x-api-key", HIRO_API_KEY.clone())
        .send()
        .await
    {
        Ok(res) => {
            if res.status().is_success() {
                let res: BlockActivity = if let Ok(tx) = res.json::<BlockActivity>().await {
                    tx
                } else {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::new(
                            Status::InternalServerError,
                            format!("Failed to retrieve tx activity for tx_id {}", body.tx_id),
                        )),
                    );
                };
                let tx = res.results.into_iter().find(|tx| {
                    tx.location.tx_id == body.tx_id
                        && tx.operation == Operation::Receive
                        && tx.location.vout == body.tx_vout
                });

                if let Some(tx) = tx {
                    tx
                } else {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::new(
                            Status::InternalServerError,
                            format!(
                                "Failed to retrieve a send activity for tx_id {}",
                                body.tx_id
                            ),
                        )),
                    );
                }
            } else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::new(
                        Status::InternalServerError,
                        format!(
                            "Failed to fetch transaction activity for tx_id {}",
                            body.tx_id
                        ),
                    )),
                );
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!(
                        "Failed to fetch block activity for tx_id {}: {}",
                        body.tx_id, e
                    ),
                )),
            );
        }
    };

    if tx_data.address.is_none() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!(
                    "Failed to retrieve a address of Receive operation for tx_id {}",
                    body.tx_id
                ),
            )),
        );
    }

    // Recompute bitcoin_deposit_addr from starknet_addr and ensure it matches the one in tx_data
    let bitcoin_deposit_addr = get_bitcoin_addr_from_starknet_addr(body.starknet_addr);
    if bitcoin_deposit_addr != tx_data.clone().address.unwrap() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!(
                    "Bitcoin deposit address does not match for starknet address: {:?}",
                    body.starknet_addr
                ),
            )),
        );
    }

    // For now we return the arguments for the claim_rune tx : rune_id: u8, rune_amount: u256, target_addr: ContractAddress
    let rune = match state
        .db
        .get_rune(&mut session, tx_data.clone().rune.id)
        .await
    {
        Ok(rune) => rune,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Rune not supported: {:?}", err),
                )),
            )
        }
    };

    let rune_id: FieldElement = symbol_as_felt(rune.symbol);

    let amount = if let Some(amount) = tx_data.clone().amount {
        amount
    } else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Amount is not specified: {:?}", tx_data.clone().amount),
            )),
        );
    };
    let amount_bigint = match convert_to_bigint(&amount, rune.divisibility) {
        Ok(amount_bigint) => amount_bigint,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Amount is not a valid number: {:?}", err),
                )),
            )
        }
    };
    let amount_felt = to_uint256(amount_bigint);

    let tx_id_felt = if let Ok(tx_id) = hex_to_uint256(&body.tx_id) {
        tx_id
    } else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Invalid tx_id: {:?}", body.tx_id),
            )),
        );
    };

    // Compute signature of (rune_id, rune_amount, target_addr, deposit_tx_id)
    let hashed = pedersen_hash(
        &pedersen_hash(
            &pedersen_hash(&rune_id, &amount_felt.0),
            &body.starknet_addr.felt,
        ),
        &tx_id_felt.0,
    );
    let signature: ExtendedSignature = match ecdsa_sign(&RUNES_BRIDGE_STARKNET_PRIV_KEY, &hashed) {
        Ok(signature) => signature,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Error while generating signature: {}", e),
                )),
            )
        }
    };

    let vout = if let Some(vout) = tx_data.location.vout {
        vout as u32
    } else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Vout is not specified: {:?}", tx_data.location.vout),
            )),
        );
    };

    // store deposit into database
    if let Err(err) = state
        .db
        .store_deposit(
            &mut session,
            DepositDocument {
                identifier: format!("{}:{}", body.tx_id.clone(), vout),
                tx_id: body.tx_id.clone(),
                vout,
                rune: tx_data.rune,
                amount,
                bitcoin_deposit_addr,
            },
        )
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            )),
        );
    }

    if let Err(err) = session.commit_transaction().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            )),
        );
    };

    let claim_data = ClaimData {
        rune_id,
        amount: amount_felt,
        target_addr: body.starknet_addr,
        tx_id: body.tx_id.clone(),
        tx_vout: vout,
        sig: Signature {
            r: signature.r,
            s: signature.s,
        },
    };

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, claim_data)),
    )
}

#[cfg(test)]
mod tests {
    use bigdecimal::{num_bigint::BigInt, Num};
    use starknet::core::{
        crypto::{ecdsa_sign, pedersen_hash},
        types::FieldElement,
    };

    use crate::utils::starknet::to_uint256;

    #[test]
    fn test_generate_signature() {
        let priv_key = FieldElement::from_hex_be("0x123").unwrap();

        let rune_id: FieldElement = FieldElement::from_hex_be("0x95909ff0").unwrap();
        let amount = (
            FieldElement::from_hex_be("0x7a120").unwrap(),
            FieldElement::ZERO,
        );
        let addr = FieldElement::from_hex_be(
            "0x403c80a49f16ed8ecf751f4b3ad62cc8f85ebeb2d40dc3b4377a089b438995d",
        )
        .unwrap();

        let tx_deposit_id = "bd51cd6d88a59456e2585c2dd61e51f91645dd071d33484d0015328f460057fc";
        // Digest = [0xfc570046, 0x8f321500, 0x4d48331d, 0x7dd4516, 0xf9511ed6, 0x2d5c58e2, 0x5694a588, 0x6dcd51bd]
        let tx_u256 = to_uint256(BigInt::from_str_radix(tx_deposit_id, 16).unwrap());

        assert_eq!(
            tx_u256,
            (
                FieldElement::from_dec_str("29605767366663658861677795006692218876").unwrap(),
                FieldElement::from_dec_str("251648833821019018272888897087823827449").unwrap()
            )
        );

        let hashed = pedersen_hash(
            &pedersen_hash(&pedersen_hash(&rune_id, &amount.0), &addr),
            &tx_u256.0,
        );

        assert_eq!(
            hashed,
            FieldElement::from_dec_str(
                "1356414087408964637607039731483801640134638362415144195139607419676485543996"
            )
            .unwrap()
        );

        match ecdsa_sign(&priv_key, &hashed) {
            Ok(signature) => {
                assert_eq!(
                    signature.r,
                    FieldElement::from_hex_be(
                        "0x035517e49e7a1337428401645f05ee58f3be3d612a09732262875bf0a9c20a53"
                    )
                    .unwrap()
                );
                assert_eq!(
                    signature.s,
                    FieldElement::from_hex_be(
                        "0x05d541de65ec78c7b2c667830a79f233f5c3aeef1fde771adaa21ad2a73a0251"
                    )
                    .unwrap()
                );
            }
            Err(e) => {
                println!("Error while generating signature: {}", e);
            }
        };
    }
}
