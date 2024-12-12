use std::env;
use std::str::FromStr;
use std::sync::Arc;

use crate::models::activity::BlockActivityResult;
use crate::models::claim::{ClaimData, Signature};
use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use crate::utils::starknet::{convert_to_bigint, to_uint256};
use crate::utils::Address;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use starknet::core::crypto::{ecdsa_sign, pedersen_hash, ExtendedSignature};
use starknet::core::types::FieldElement;

use super::responses::{ApiResponse, Status};

lazy_static::lazy_static! {
    static ref RUNES_BRIDGE_PRIV_KEY: FieldElement =   FieldElement::from_hex_be(&env::var("RUNES_BRIDGE_PRIV_KEY")
    .expect("RUNES_BRIDGE_PRIV_KEY must be set")).expect("Invalid RUNES_BRIDGE_PRIV_KEY");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimDepositDataQuery {
    starknet_addr: Address,
    bitcoin_deposit_addr: String,
    tx_data: BlockActivityResult,
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

    // For now we return the arguments for the claim_rune tx : rune_id: u8, rune_amount: u256, target_addr: ContractAddress
    let rune = match state
        .db
        .get_rune(&mut session, body.tx_data.clone().rune.id)
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

    let amount = match body.tx_data.clone().amount {
        Some(amount) => {
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
            to_uint256(amount_bigint)
        }
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Amount is not specified: {:?}", body.tx_data.clone().amount),
                )),
            )
        }
    };

    // Compute signature of (rune_id, rune_amount, target_addr, deposit_tx_id)
    let hashed = pedersen_hash(
        &rune_id,
        &pedersen_hash(
            &amount.0,
            &pedersen_hash(
                &body.starknet_addr.felt,
                &FieldElement::from_str(&body.tx_data.location.tx_id).unwrap(),
            ),
        ),
    );
    let signature: ExtendedSignature = match ecdsa_sign(&RUNES_BRIDGE_PRIV_KEY, &hashed) {
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

    let claim_data = ClaimData {
        rune_id,
        amount,
        target_addr: body.starknet_addr,
        tx_id: body.tx_data.clone().location.tx_id,
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
    use crypto_bigint::{Encoding, U256};
    use starknet::core::{
        crypto::{ecdsa_sign, pedersen_hash},
        types::FieldElement,
    };
    use starknet_crypto::get_public_key;

    fn to_u256(tx_id: &str) -> (u128, u128) {
        // Convert the hex string to U256
        let tx_u256 = U256::from(U256::from_be_hex(tx_id));

        // Extract high and low as u128
        let high_u128 = (tx_u256 >> 128).to_be_bytes()[..16].try_into().unwrap();
        let low_u128 = tx_u256.to_be_bytes()[16..].try_into().unwrap();

        // Convert the bytes directly to u128
        let high_u128 = u128::from_be_bytes(high_u128);
        let low_u128 = u128::from_be_bytes(low_u128);

        (low_u128, high_u128)
    }

    #[test]
    fn test_generate_signature() {
        let priv_key = FieldElement::from_hex_be("0x123").unwrap();
        let pub_key = get_public_key(&priv_key);
        println!("Public key: {:?}", pub_key);

        let rune_id: FieldElement = FieldElement::from_dec_str("97").unwrap();
        let amount = (FieldElement::from(2500_u128), FieldElement::ZERO);
        let addr = FieldElement::from(504447201841_u128);

        let tx_deposit_id = "a795ede3bec4b9095eb207bff4abacdbcdd1de065788d4ffb53b1ea3fe5d67fb";
        let tx_u256 = to_u256(tx_deposit_id);

        let hashed = pedersen_hash(
            &pedersen_hash(&pedersen_hash(&rune_id, &amount.0), &addr),
            &FieldElement::from(tx_u256.0),
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
