use std::sync::Arc;

use crate::server::responses::{ApiResponse, Status};
use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDepositBitcoin {
    bitcoin_addr: String,
}

#[route(get, "/get_deposits/bitcoin")]
pub async fn get_deposit_bitcoin(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetDepositBitcoin>,
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

    match state
        .db
        .get_deposits_bitcoin(&mut session, query.bitcoin_addr)
        .await
    {
        Ok(deposits) => (
            StatusCode::ACCEPTED,
            Json(ApiResponse::new(Status::Success, deposits)),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            )),
        ),
    }
}
