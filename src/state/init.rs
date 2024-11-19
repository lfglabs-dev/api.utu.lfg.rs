use std::{env, sync::Arc};

use bitcoincore_rpc::Auth;
use mongodb::options::ClientOptions;
use reqwest::Url;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};

use crate::{logger::Logger, utils::starknet_tx::get_account};

use super::AppState;

pub trait AppStateTraitInitializer {
    async fn load() -> Arc<Self>;
}

impl AppStateTraitInitializer for AppState {
    async fn load() -> Arc<Self> {
        let logger = Logger::new();
        let db = mongodb::Client::with_options(
            ClientOptions::parse(
                env::var("MONGODB_CONNECTION_STRING")
                    .expect("MONGODB_CONNECTION_STRING must be set"),
            )
            .await
            .unwrap(),
        )
        .unwrap()
        .database(&env::var("MONGODB_NAME").expect("MONGODB_NAME must be set"));

        let bitcoin_rpc_user = env::var("BITCOIN_RPC_USER").expect("BITCOIN_RPC_USER must be set");
        let bitcoin_rpc_password =
            env::var("BITCOIN_RPC_PASSWORD").expect("BITCOIN_RPC_PASSWORD must be set");
        let bitcoin_auth = if bitcoin_rpc_user.is_empty() || bitcoin_rpc_password.is_empty() {
            Auth::None
        } else {
            Auth::UserPass(bitcoin_rpc_user, bitcoin_rpc_password)
        };
        println!("auth {:?}", bitcoin_auth);
        let bitcoin_provider = bitcoincore_rpc::Client::new(
            &env::var("BITCOIN_RPC_URL").expect("BITCOIN_RPC_URL must be set"),
            bitcoin_auth,
        )
        .unwrap();

        let starknet_provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&env::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL must be set"))
                .unwrap(),
        ));
        let starknet_account = get_account().await;

        Arc::new_cyclic(|_| AppState {
            logger,
            db,
            bitcoin_provider,
            starknet_provider,
            starknet_account,
        })
    }
}
