use std::collections::HashMap;
use std::sync::Arc;

use crate::state::AppState;
use crate::try_start_session;
use crate::{state::database::DatabaseExt, utils::Address};
use axum::extract::State;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use mongodb::bson::doc;
use reqwest::StatusCode;
use starknet_crypto::Felt;

use super::responses::{ApiResponse, Status};

#[route(get, "/bitcoin_deposits")]
pub async fn bitcoin_deposits<B>(
    State(state): State<Arc<AppState>>,
    request: Request<B>,
) -> impl IntoResponse {
    let mut session = try_start_session!(state);

    let query_string = request.uri().query().unwrap_or_default();
    let params: HashMap<String, Vec<String>> = url::form_urlencoded::parse(query_string.as_bytes())
        .into_owned()
        .fold(HashMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_default().push(value);
            acc
        });
    let starknet_addresses: Vec<String> = match params.get("starknet_receiving_addresses") {
        Some(addresses) => addresses
            .iter()
            .map(|address| {
                // Try parsing as hex; fall back to decimal if hex parsing fails
                Felt::from_hex(address)
                    .or_else(|_| Felt::from_dec_str(address))
                    .map(|addr_felt| (Address { felt: addr_felt }).to_string())
                    .unwrap_or_else(|_| String::new())
            })
            .filter(|parsed_address| !parsed_address.is_empty())
            .collect(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::new(Status::BadRequest, "bad_request")),
            )
        }
    };

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
