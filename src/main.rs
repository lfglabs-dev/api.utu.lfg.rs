#[macro_use]
mod utils;
mod logger;
mod models;
mod server;
mod state;
use axum::Extension;
use axum::{http::StatusCode, Router};
use axum_auto_routes::route;
use bitcoincore_rpc::RpcApi;
use models::block::BlockWithTransactions;
use mongodb::bson::doc;
use state::init::AppStateTraitInitializer;
use state::AppState;
use state::WithState;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::{env, thread};
use tower_http::cors::{self, CorsLayer};
use utils::indexer::get_block_hash;

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

    // Spawn a task to listen for ZMQ messages
    let context = zmq::Context::new();
    let subscriber = context.socket(zmq::SUB).expect("Failed to create socket");
    subscriber
        .connect(&format!(
            "tcp://localhost:{}",
            env::var("ZMQ_PORT").expect("ZMQ_PORT must be set")
        ))
        .expect("Failed to connect to socket");

    // Subscribe to topics
    subscriber
        .set_subscribe(b"hashblock")
        .expect("Failed to subscribe to hashblock");

    let zmq_state = shared_state.clone();
    let zmq_task = tokio::spawn(async move {
        loop {
            // Wait for a message from the socket
            match subscriber.recv_msg(0) {
                Ok(topic) => {
                    if topic.as_str() == Some("hashblock") {
                        let block_hash_msg = subscriber
                            .recv_msg(0)
                            .expect("Failed to receive block hash");

                        let block_hash = match get_block_hash(block_hash_msg) {
                            Ok(block_hash) => block_hash,
                            Err(e) => {
                                zmq_state
                                    .logger
                                    .info(format!("Failed to get block hash: {}", e));
                                continue;
                            }
                        };
                        println!("block hash: {:?}", block_hash);

                        match zmq_state.bitcoin_provider.call::<BlockWithTransactions>(
                            "getblock",
                            &[serde_json::to_value(block_hash).unwrap(), 2.into()],
                        ) {
                            Ok(block) => {
                                println!("block: {:?}", block);
                                // todo: once we're able to send rune on regtest check tx results
                                // let mut address_amounts: Vec<BitcoinAddresses> = Vec::new();
                                // for tx in &block.tx {
                                //     for vout in &tx.vout {
                                //         // Check if there are addresses in the scriptPubKey
                                //         if let Some(addresses) = &vout.script_pub_key.addresses {
                                //             // Create an AddressAmount for each address in this vout
                                //             address_amounts.push(BitcoinAddresses {
                                //                 addresses: addresses.to_vec(),
                                //                 amount: vout.value,
                                //             });
                                //         }
                                //     }
                                // }
                                // println!("address_amounts: {:?}", address_amounts);
                            }
                            Err(e) => eprintln!("Failed to get block: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Failed to receive message: {}", e),
            }

            thread::sleep(Duration::from_millis(1));
        }
    });

    // wait for both the zqm task and server to stop the program
    tokio::select! {
        _ = server_task => {},
        _ = zmq_task => {},
    }
}

#[route(get, "/")]
async fn root() -> (StatusCode, String) {
    (
        StatusCode::ACCEPTED,
        format!("server v{}", env!("CARGO_PKG_VERSION")),
    )
}
