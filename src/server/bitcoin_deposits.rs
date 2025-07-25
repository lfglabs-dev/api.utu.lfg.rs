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
use utu_bridge_types::starknet::StarknetAddress;

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
    let starknet_addresses: Vec<StarknetAddress> = match params.get("starknet_receiving_addresses")
    {
        Some(addresses) => {
            match addresses
                .iter()
                .map(|address| StarknetAddress::new(address))
                .collect::<Result<Vec<StarknetAddress>, _>>()
            {
                Ok(addresses) => addresses,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::new(
                            Status::BadRequest,
                            "invalid_starknet_address",
                        )),
                    );
                }
            }
        }
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
        StatusCode::OK,
        Json(ApiResponse::new(Status::Success, deposits)),
    )
}
