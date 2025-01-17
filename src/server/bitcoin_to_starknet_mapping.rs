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
pub struct BitcoinStarknetMappingQuery {
    bitcoin_addresses: Vec<String>,
}

#[route(get, "/bitcoin_to_starknet_mapping")]
pub async fn bitcoin_to_starknet_mapping<B>(
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
    let bitcoin_addresses = match params.get("bitcoin_addresses") {
        Some(addresses) => addresses.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::new(Status::BadRequest, "bad_request")),
            )
        }
    };

    let bitcoin_starknet_mapping = match state
        .db
        .get_starknet_addrs(&mut session, bitcoin_addresses)
        .await
    {
        Ok(addr) => addr,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::new(
                    Status::InternalServerError,
                    format!(
                    "Error while fetching bitcoin_starknet_mappingbitcoin_starknet_mapping: {:?}",
                    err
                ),
                )),
            )
        }
    };

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, bitcoin_starknet_mapping)),
    )
}
