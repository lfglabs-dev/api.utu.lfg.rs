use std::{env, sync::Arc};

use anyhow::Result;
use mongodb::ClientSession;
use reqwest::Client;

use crate::{
    models::runes::{Operation, RuneActivity, RuneActivityForAddress},
    state::{database::DatabaseExt, AppState},
};

lazy_static::lazy_static! {
    static ref HIRO_API_URL: String = env::var("HIRO_API_URL").expect("HIRO_API_URL must be set");
    static ref HIRO_API_KEY: String = env::var("HIRO_API_KEY").expect("HIRO_API_KEY must be set");
}

pub async fn get_activity_bitcoin_addr(
    state: &Arc<AppState>,
    session: &mut ClientSession,
    bitcoin_addr: String,
    operation: Operation,
) -> Result<Vec<RuneActivity>> {
    let mut response: Vec<RuneActivity> = Vec::new();

    // retrieve available runes
    let runes = state.db.get_supported_runes(session).await?;
    for rune in runes {
        let mut offset = 0;
        let mut total = 0;
        loop {
            let url = format!(
                "{}/runes/v1/etchings/{}/activity/{}?offset={}&limit=60",
                *HIRO_API_URL, rune.id, bitcoin_addr, offset
            );

            let client = Client::new();
            match client
                .get(url)
                .header("x-api-key", HIRO_API_KEY.clone())
                .send()
                .await
            {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.json::<RuneActivityForAddress>().await {
                            Ok(account_activity) => {
                                // println!("Block Activity: {:?}", block_activity);
                                total = account_activity.total;

                                for tx in account_activity.results {
                                    if tx.operation == operation {
                                        if operation == Operation::Receive
                                            && tx.receiver_address.is_some()
                                        {
                                            response.push(tx.clone());
                                        }

                                        if operation == Operation::Send
                                            && tx.address.is_some()
                                            && tx.receiver_address.is_some()
                                        {
                                            // we ensure the receiver is one of our addresses 
                                            let receiver_addr = tx.clone().receiver_address.unwrap();
                                            if state
                                                .db
                                                .is_deposit_addr(session, receiver_addr)
                                                .await
                                                .is_ok()
                                            {
                                                response.push(tx);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                println!("Failed to deserialize response: {:?}", err);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch block activity: {}", e)
                }
            }

            // we fetch 60 results at a time but total activity could be more
            offset += 1;
            if total <= offset * 60 {
                break;
            }
        }
    }

    Ok(response)
}
