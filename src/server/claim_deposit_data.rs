use std::sync::Arc;

use crate::models::activity::BlockActivityResult;
use crate::models::claim::ClaimData;
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
use starknet::core::types::FieldElement;

use super::responses::{ApiResponse, Status};

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
                    format!("Amount is not specified: {:?}", body.tx_data.amount),
                )),
            )
        }
    };

    let claim_data = ClaimData {
        rune_id,
        amount,
        target_addr: body.starknet_addr,
        tx_id: body.tx_data.clone().location.tx_id,
    };

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, claim_data)),
    )
}
