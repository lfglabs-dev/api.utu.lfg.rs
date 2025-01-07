use std::env;
use std::str::FromStr;
use std::sync::Arc;

use crate::models::activity::BlockActivity;
use crate::models::claim::{ClaimData, Signature};
use crate::models::deposit::DepositDocument;
use crate::models::runes::Operation;
use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use crate::utils::deposit::get_bitcoin_addr_from_starknet_addr;
use crate::utils::starknet::{convert_to_bigint, to_uint256};
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

    if  tx_data.address.is_none() {
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

    let rune_id: FieldElement = if rune.symbol.chars().count() == 1 {
        let single_char = rune.symbol.chars().next().unwrap();
        FieldElement::from(single_char as u32)
    } else {
        state.logger.warning(format!(
            "Rune symbol is not a valid character: {:?}",
            rune.symbol
        ));
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Rune symbol is not a single character: {:?}", rune.symbol),
            )),
        );
    };

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

    // Compute signature of (rune_id, rune_amount, target_addr, deposit_tx_id)
    let hashed = pedersen_hash(
        &rune_id,
        &pedersen_hash(
            &amount_felt.0,
            &pedersen_hash(
                &body.starknet_addr.felt,
                &FieldElement::from_str(&body.tx_id).unwrap(),
            ),
        ),
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
    use starknet_crypto::get_public_key;

    use crate::utils::starknet::to_uint256;

    #[test]
    fn test_generate_signature() {
        let priv_key = FieldElement::from_hex_be("0x123").unwrap();
        let pub_key = get_public_key(&priv_key);
        println!("Public key: {:?}", pub_key);

        let rune_id: FieldElement = FieldElement::from_dec_str("97").unwrap();
        let amount = (FieldElement::from(2500_u128), FieldElement::ZERO);
        let addr = FieldElement::from(504447201841_u128);

        let tx_deposit_id = "a795ede3bec4b9095eb207bff4abacdbcdd1de065788d4ffb53b1ea3fe5d67fb";
        let tx_u256 = to_uint256(BigInt::from_str_radix(tx_deposit_id, 16).unwrap());

        let hashed = pedersen_hash(
            &pedersen_hash(&pedersen_hash(&rune_id, &amount.0), &addr),
            &tx_u256.0,
        );

        assert_eq!(
            hashed,
            FieldElement::from_hex_be(
                "0x05b83a0441dd5eb9409a4f4bb3775fc7d420aed6abb307d11e59668acc192c3d"
            )
            .unwrap()
        );

        match ecdsa_sign(&priv_key, &hashed) {
            Ok(signature) => {
                assert_eq!(
                    signature.r,
                    FieldElement::from_hex_be(
                        "0x00823dd95547161bb6612384e59b9c041b97fe2c0c02bf521a0b8d8b449a05eb"
                    )
                    .unwrap()
                );
                assert_eq!(
                    signature.s,
                    FieldElement::from_hex_be(
                        "0x06367b23138364e35314840bd0fd826626d1e9283e303d263655edee80e27487"
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
