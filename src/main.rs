#[macro_use]
mod utils;
mod logger;
mod models;
mod server;
mod state;
use axum::Extension;
use axum::{http::StatusCode, Router};
use axum_auto_routes::route;
use mongodb::bson::doc;
use state::init::AppStateTraitInitializer;
use state::AppState;
use state::WithState;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use tower_http::cors::{self, CorsLayer};

lazy_static::lazy_static! {
    pub static ref ROUTE_REGISTRY: Mutex<Vec<Box<dyn WithState>>> = Mutex::new(Vec::new());
}

#[tokio::main]
async fn main() {
    let shared_state: Arc<AppState> = AppState::load().await;
    shared_state
        .logger
        .async_info("starting utu rest_server")
        .await;

    // setup http server
    let cors = CorsLayer::new()
        .allow_headers(cors::Any)
        .allow_origin(cors::Any);
    let app = ROUTE_REGISTRY
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .fold(Router::new(), |acc, r| {
            acc.merge(r.to_router(shared_state.clone()))
        })
        .layer(cors)
        .layer(Extension(shared_state.clone()));

    let server_port = env::var("SERVER_PORT")
        .expect("SERVER_PORT must be set")
        .parse::<u16>()
        .expect("invalid SERVER_PORT format");
    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));

    // spawn the server task
    let server_task = tokio::spawn(async move {
        axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    });
    shared_state
        .logger
        .async_info(format!(
            "server: listening on http://0.0.0.0:{}",
            server_port
        ))
        .await;

    // wait for server to stop the program
    tokio::select! {
        _ = server_task => {},
    }
}

#[route(get, "/")]
async fn root() -> (StatusCode, String) {
    (
        StatusCode::OK,
        format!("server v{}", env!("CARGO_PKG_VERSION")),
    )
}
