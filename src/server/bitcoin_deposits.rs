use std::collections::HashMap;
use std::sync::Arc;

use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use axum::extract::State;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::responses::{ApiResponse, Status};

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinDepositsQuery {
    starknet_receiving_addresses: Vec<String>,
}

#[route(get, "/bitcoin_deposits")]
pub async fn bitcoin_deposits<B>(
    State(state): State<Arc<AppState>>,
    request: Request<B>,
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

    let query_string = request.uri().query().unwrap_or_default();
    let params: HashMap<String, Vec<String>> = url::form_urlencoded::parse(query_string.as_bytes())
        .into_owned()
        .fold(HashMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_default().push(value);
            acc
        });
    let starknet_addresses = match params.get("starknet_receiving_addresses") {
        Some(addresses) => addresses.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::new(Status::BadRequest, "bad_request")),
            )
        }
    };

    // For now we return the arguments for the claim_rune tx : rune_id: u8, rune_amount: u256, target_addr: ContractAddress
    let deposits = match state
        .db
        .get_bitcoin_deposits(&mut session, starknet_addresses)
        .await
    {
        Ok(deposits) => deposits,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!("Error while retrieving bitcoin deposits : {:?}", err),
                )),
            )
        }
    };

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, deposits)),
    )
}
