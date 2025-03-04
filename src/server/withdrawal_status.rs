use std::str::FromStr;
use std::sync::Arc;

use crate::server::responses::{ApiResponse, Status};
use crate::state::database::DatabaseExt;
use crate::state::{AppState, DatabaseError};
use crate::try_start_session;
use crate::utils::Address;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use bitcoin::Txid;
use bitcoincore_rpc::RpcApi;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawalStatusQuery {
    sn_txhash: Address,
}

#[route(get, "/withdrawal_status")]
pub async fn withdrawal_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WithdrawalStatusQuery>,
) -> impl IntoResponse {
    let mut session = try_start_session!(state);

    let withdrawal_status = match state
        .db
        .get_withdrawal_status(&mut session, query.sn_txhash)
        .await
    {
        Ok(status) => status,
        Err(err) => match err {
            DatabaseError::NotFound => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::new(Status::NotFound, "not_found".to_string())),
                )
            }
            _ => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::new(
                        Status::InternalServerError,
                        format!("Error while retrieving withdrawal status : {:?}", err),
                    )),
                )
            }
        },
    };

    if withdrawal_status.matched_submissions.is_none()
        && withdrawal_status.rejected_status.is_none()
    {
        return (
            StatusCode::ACCEPTED,
            Json(ApiResponse::new(
                Status::Success,
                json!({ "status": "in_review" }),
            )),
        );
    }

    if withdrawal_status.rejected_status.is_some() {
        return (
            StatusCode::ACCEPTED,
            Json(ApiResponse::new(
                Status::Success,
                json!({ "status": "rejected", "reason": withdrawal_status.rejected_status.unwrap() }),
            )),
        );
    }

    if withdrawal_status.matched_submissions.is_some() {
        let matched_submissions = withdrawal_status.matched_submissions.unwrap();

        let txid = match Txid::from_str(&matched_submissions.request_id) {
            Ok(txid) => txid,
            Err(_) => {
                return (
                    StatusCode::ACCEPTED,
                    Json(ApiResponse::new(
                        Status::Success,
                        json!({ "status": "in_review" }),
                    )),
                );
            }
        };

        // Check the transaction on the network
        if let Ok(tx) = state.bitcoin_provider.get_raw_transaction_info(&txid, None) {
            return (
                StatusCode::ACCEPTED,
                Json(ApiResponse::new(
                    Status::Success,
                    json!({ "status": "submitted", "btc_txid": tx.txid }),
                )),
            );
        } else {
            return (
                StatusCode::ACCEPTED,
                Json(ApiResponse::new(
                    Status::Success,
                    json!({ "status": "in_review" }),
                )),
            );
        }
    }

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(
            Status::Success,
            json!({ "status": "in_review" }),
        )),
    )
}
