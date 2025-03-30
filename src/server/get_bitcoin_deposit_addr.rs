use std::sync::Arc;

use crate::state::database::DatabaseExt;
use crate::state::AppState;
use crate::try_start_session;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use axum_auto_routes::route;
use bitcoin::Network;
use mongodb::bson::doc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use utu_bridge_deposit_address::get_deposit_address;
use utu_bridge_types::bitcoin::BitcoinAddress;
use utu_bridge_types::starknet::StarknetAddress;

use super::responses::{ApiResponse, Status};

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinDepositAddrQuery {
    starknet_addr: StarknetAddress,
}

#[route(get, "/get_bitcoin_deposit_addr")]
pub async fn get_bitcoin_deposit_addr(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BitcoinDepositAddrQuery>,
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

    let deposit_addr = get_deposit_address(query.starknet_addr.felt);

    // store deposit address into database
    if let Err(err) = state
        .db
        .set_user_bitcoin_deposit_addr(
            &mut session,
            query.starknet_addr,
            BitcoinAddress::new(deposit_addr.to_string().as_str(), Network::Bitcoin).unwrap(),
        )
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            )),
        );
    }

    if let Err(err) = session.commit_transaction().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::new(
                Status::InternalServerError,
                format!("Database error: {:?}", err),
            )),
        );
    };

    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::new(Status::Success, deposit_addr)),
    )
}
