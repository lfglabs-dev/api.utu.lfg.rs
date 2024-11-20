use std::{env, sync::Arc};

use mongodb::options::ClientOptions;

use crate::logger::Logger;

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

        Arc::new_cyclic(|_| AppState { logger, db })
    }
}
