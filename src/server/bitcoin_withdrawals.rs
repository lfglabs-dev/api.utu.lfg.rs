use std::sync::Arc;

use crate::models::withdrawal::{BitcoinWithdrawalResponse, BitcoinWithdrawalStatus};
use crate::state::database::DatabaseExt;
use crate::state::{AppState, DatabaseError};
use crate::try_start_session;
use crate::utils::withdrawals::retrieve_submission_status;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use utu_bridge_types::bitcoin::BitcoinAddress;
use utu_bridge_types::starknet::StarknetAddress;

use super::responses::{ApiResponse, Status};

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinWithdrawalQuery {
    bitcoin_receiving_address: Option<BitcoinAddress>,
    starknet_sending_address: Option<StarknetAddress>,
}

#[route(get, "/bitcoin_withdrawals")]
pub async fn bitcoin_withdrawals(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BitcoinWithdrawalQuery>,
) -> impl IntoResponse {
    let mut session = try_start_session!(state);

    let withdrawals = match state
        .db
        .get_bitcoin_withdrawals(
            &mut session,
            query.bitcoin_receiving_address,
            query.starknet_sending_address,
        )
        .await
    {
        Ok(withdrawals) => withdrawals,
        Err(err) => match err {
            DatabaseError::NotFound => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::new(Status::NotFound, "not_found")),
                )
            }
            _ => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::new(
                        Status::InternalServerError,
                        format!("Database error: {:?}", err),
                    )),
                )
            }
        },
    };

    let mut result: Vec<BitcoinWithdrawalResponse> = Vec::new();
    for withdrawal in withdrawals {
        if withdrawal.matched_submissions.is_some() {
            let matched_submissions = withdrawal.matched_submissions.unwrap();

            if matched_submissions.rejected_status.is_some() {
                result.push(BitcoinWithdrawalResponse {
                    status: BitcoinWithdrawalStatus::Rejected,
                    sn_txhash: withdrawal.transaction_hash,
                    reason: matched_submissions.rejected_status,
                    btc_txid: None,
                });
            } else if matched_submissions.request_id.is_some() {
                let request_id = matched_submissions.request_id.unwrap();
                let submission =
                    retrieve_submission_status(&state, withdrawal.transaction_hash, request_id);
                result.push(submission);
            } else {
                result.push(BitcoinWithdrawalResponse {
                    status: BitcoinWithdrawalStatus::InReview,
                    sn_txhash: withdrawal.transaction_hash,
                    reason: None,
                    btc_txid: None,
                });
            }
        } else {
            result.push(BitcoinWithdrawalResponse {
                status: BitcoinWithdrawalStatus::InReview,
                sn_txhash: withdrawal.transaction_hash,
                reason: None,
                btc_txid: None,
            });
        }
    }

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, result)),
    )
}
