use std::{collections::HashMap, env, str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use bitcoin::BlockHash;
use bitcoincore_rpc::RpcApi;
use mongodb::ClientSession;
use reqwest::Client;

use crate::{
    models::{
        deposit::{DepositActivityDetails, DepositStatus},
        runes::{Operation, RuneActivityForAddress},
    },
    state::{database::DatabaseExt, rate_limit::RateLimitStateTrait, AppState},
};

lazy_static::lazy_static! {
    static ref HIRO_API_URL: String = env::var("HIRO_API_URL").expect("HIRO_API_URL must be set");
    static ref HIRO_API_KEY: String = env::var("HIRO_API_KEY").expect("HIRO_API_KEY must be set");
    static ref MIN_CONFIRMATIONS: i32 = env::var("MIN_CONFIRMATIONS").expect("MIN_CONFIRMATIONS must be set").parse::<i32>().expect("unable to parse MIN_CONFIRMATIONS as i32");
    static ref HTTP_CLIENT: Client = Client::builder()
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to create HTTP client");
}

// Function that retrieves the activity for a given bitcoin address from the hiro api
pub async fn get_activity_bitcoin_addr(
    state: &Arc<AppState>,
    session: &mut ClientSession,
    bitcoin_sending_addr: String,
    bitcoin_deposit_addr: Option<String>,
) -> Result<Vec<DepositActivityDetails>> {
    let mut response: Vec<DepositActivityDetails> = Vec::new();

    // retrieve available runes
    let runes = state.db.get_supported_runes(session).await?;
    for rune in runes {
        let mut offset = 0;
        let mut total = 0;
        loop {
            state.rate_limit.add_entry().await;

            let url = format!(
                "{}/runes/v1/etchings/{}/activity/{}?offset={}&limit=60",
                *HIRO_API_URL, rune.id, bitcoin_sending_addr, offset
            );

            let res = HTTP_CLIENT
                .get(url)
                .header("x-api-key", HIRO_API_KEY.clone())
                .send()
                .await?;

            if !res.status().is_success() {
                state.logger.warning(format!(
                    "Failed to fetch block activity for rune {} and bitcoin_addr {}",
                    rune.clone().name,
                    bitcoin_sending_addr
                ));
                break;
            }

            let account_activity = res.json::<RuneActivityForAddress>().await?;
            total = account_activity.total;

            for tx in account_activity.results {
                if tx.operation == Operation::Send
                    && tx.address.is_some()
                    && tx.receiver_address.is_some()
                {
                    // we ensure the receiver is one of our addresses or the bitcoin_deposit_addr specified
                    let receiver_addr = tx.clone().receiver_address.unwrap();
                    if bitcoin_deposit_addr.is_some()
                        && receiver_addr == bitcoin_deposit_addr.clone().unwrap()
                    {
                        response.push(DepositActivityDetails {
                            rune: rune.clone(),
                            tx,
                        });
                    } else {
                        if state
                            .db
                            .is_deposit_addr(session, receiver_addr)
                            .await
                            .is_ok()
                        {
                            response.push(DepositActivityDetails {
                                rune: rune.clone(),
                                tx,
                            });
                        }
                    }
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

pub async fn filter_deposits(
    state: &Arc<AppState>,
    session: &mut ClientSession,
    deposits: Vec<DepositActivityDetails>,
) -> Result<HashMap<DepositStatus, Vec<DepositActivityDetails>>> {
    let mut filtered_deposits: HashMap<DepositStatus, Vec<DepositActivityDetails>> = HashMap::new();
    for deposit in deposits {
        let block_hash = BlockHash::from_str(&deposit.tx.location.block_hash)?;
        let res = state.bitcoin_provider.get_block_header_info(&block_hash)?;
        if res.confirmations >= *MIN_CONFIRMATIONS {
            // update deposit status to confirmed or claimed
            if state
                .db
                .is_blacklisted(session, deposit.tx.location.tx_id.clone())
                .await
                .is_ok()
            {
                filtered_deposits
                    .entry(DepositStatus::Claimed)
                    .or_default()
                    .push(deposit);
            } else {
                filtered_deposits
                    .entry(DepositStatus::Confirmed)
                    .or_default()
                    .push(deposit);
            }
        } else {
            // update deposit status to pending
            filtered_deposits
                .entry(DepositStatus::Pending)
                .or_default()
                .push(deposit);
        }
    }
    Ok(filtered_deposits)
}

pub async fn get_activity_bitcoin_addr_from_starknet(
    state: &Arc<AppState>,
    session: &mut ClientSession,
    bitcoin_deposit_addr: String,
) -> Result<Vec<DepositActivityDetails>> {
    let mut bitcoin_sending_addresses: Vec<String> = Vec::new();
    let mut response: Vec<DepositActivityDetails> = Vec::new();

    // retrieve available runes
    let runes = state.db.get_supported_runes(session).await?;
    for rune in runes {
        let mut offset = 0;
        let mut total = 0;
        loop {
            state.rate_limit.add_entry().await;

            let url = format!(
                "{}/runes/v1/etchings/{}/activity/{}?offset={}&limit=60",
                *HIRO_API_URL, rune.id, bitcoin_deposit_addr, offset
            );

            let res = HTTP_CLIENT
                .get(url)
                .header("x-api-key", HIRO_API_KEY.clone())
                .send()
                .await?;

            if !res.status().is_success() {
                state.logger.warning(format!(
                    "Failed to fetch block activity for rune {} and bitcoin_deposit_addr {}",
                    rune.clone().name,
                    bitcoin_deposit_addr
                ));
                break;
            }

            let account_activity = res.json::<RuneActivityForAddress>().await?;
            total = account_activity.total;

            for tx in account_activity.results {
                if tx.operation == Operation::Receive && tx.address.is_some() {
                    let txid = tx.location.tx_id.clone();

                    // We query api to get sending address
                    let url = format!(
                        "{}/runes/v1/transactions/{}/activity?offset=1&limit=60",
                        *HIRO_API_URL, txid
                    );
                    let res = HTTP_CLIENT
                        .get(url)
                        .header("x-api-key", HIRO_API_KEY.clone())
                        .send()
                        .await?;

                    if res.status().is_success() {
                        for tx in res.json::<RuneActivityForAddress>().await?.results {
                            if tx.operation == Operation::Send && tx.address.is_some() {
                                // We add this address into the list of addresses to check
                                let sending_addr = tx.address.unwrap();
                                if !bitcoin_sending_addresses.contains(&sending_addr) {
                                    bitcoin_sending_addresses.push(sending_addr);
                                }
                            }
                        }
                    }
                }
            }

            // we fetch 60 results at a time but total activity could be more
            offset += 1;
            if total <= offset * 60 {
                break;
            }
        }

        // Once we have all the sending addresses, we call get_activity_bitcoin_addr for each of them
        // We specify bitcoin_deposit_addr and we retrieve only deposits made to that bitcoin_deposit_addr
        for sending_addr in &bitcoin_sending_addresses {
            let deposits = get_activity_bitcoin_addr(
                state,
                session,
                sending_addr.to_string(),
                Some(bitcoin_deposit_addr.clone()),
            )
            .await?;
            response.extend(deposits);
        }
    }

    Ok(response)
}
