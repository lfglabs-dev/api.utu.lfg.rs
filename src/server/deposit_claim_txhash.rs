use std::sync::Arc;

use crate::state::database::DatabaseExt;
use crate::state::{AppState, DatabaseError};
use crate::try_start_session;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::responses::{ApiResponse, Status};

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositClaimTxhashQuery {
    btc_txid: String,
}

#[route(get, "/deposit_claim_txhash")]
pub async fn deposit_claim_txhash(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DepositClaimTxhashQuery>,
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

    let sn_txhash = match state
        .db
        .get_deposit_claim_txhash(&mut session, query.btc_txid)
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

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, sn_txhash)),
    )
}
