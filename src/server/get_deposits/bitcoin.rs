use std::sync::Arc;

use crate::models::runes::Operation;
use crate::server::auth::middleware::rate_limit_middleware;
use crate::server::responses::{ApiResponse, Status};
use crate::state::AppState;
use crate::try_start_session;
use crate::utils::deposit_activity::{filter_deposits, get_activity_bitcoin_addr};
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

#[route(get, "/get_deposits/bitcoin", rate_limit_middleware)]
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

    // We retrieve deposits from hiro api, we're looking for deposits that have a type Operation::Send
    // and where the receiver_address will be one of our deposit addresses
    // and matches the runes we support
    let deposits =
        match get_activity_bitcoin_addr(&state, &mut session, query.bitcoin_addr, Operation::Send)
            .await
        {
            Ok(deposits) => deposits,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::new(
                        Status::InternalServerError,
                        format!("Hiro API error: {:?}", err),
                    )),
                )
            }
        };

    let filtered_deposits = match filter_deposits(&state, &mut session, deposits).await {
        Ok(deposits) => deposits,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Error while filtering deposits error: {:?}", err),
                )),
            )
        }
    };

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, filtered_deposits)),
    )
}
