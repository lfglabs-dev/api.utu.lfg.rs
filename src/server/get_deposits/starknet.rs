use std::sync::Arc;

use crate::models::runes::Operation;
use crate::server::responses::{ApiResponse, Status};
use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use crate::utils::deposit_activity::{filter_deposits, get_activity_bitcoin_addr};
use crate::utils::Address;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDepositStarknet {
    starknet_addr: Address,
}

#[route(get, "/get_deposits/starknet")]
pub async fn get_deposit_starknet(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetDepositStarknet>,
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

    let bitcoin_deposit_addr = match state
        .db
        .get_bitcoin_deposit_addr(&mut session, query.starknet_addr)
        .await
    {
        Ok(addr) => addr,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Database error: {:?}", err),
                )),
            )
        }
    };

    // We retrieve deposits from hiro api, we're looking for deposits that have a type Operation::Receive
    // and matches the runes we support
    let deposits = match get_activity_bitcoin_addr(
        &state,
        &mut session,
        bitcoin_deposit_addr,
        Operation::Receive,
    )
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
