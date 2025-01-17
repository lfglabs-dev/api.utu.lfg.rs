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

use super::responses::{ApiResponse, Status};

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinWithdrawalQuery {
    bitcoin_receiving_address: Option<String>,
    starknet_sending_address: Option<String>,
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
        Ok(sn_txhash) => sn_txhash,
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
        if withdrawal.matched_submissions.is_none() && withdrawal.rejected_status.is_none() {
            result.push(BitcoinWithdrawalResponse {
                status: BitcoinWithdrawalStatus::InReview,
                sn_txhash: withdrawal.transaction_hash,
                rejected: None,
                btc_txid: None,
            });
        } else if withdrawal.rejected_status.is_some() {
            result.push(BitcoinWithdrawalResponse {
                status: BitcoinWithdrawalStatus::Rejected,
                sn_txhash: withdrawal.transaction_hash,
                rejected: withdrawal.rejected_status,
                btc_txid: None,
            });
        } else if withdrawal.matched_submissions.is_some() {
            let matched_submissions = withdrawal.matched_submissions.unwrap();
            let submission = retrieve_submission_status(
                &state,
                withdrawal.transaction_hash,
                matched_submissions.request_id,
            );
            result.push(submission);
        }
    }

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, result)),
    )
}
