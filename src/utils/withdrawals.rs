use std::{str::FromStr, sync::Arc};

use bitcoin::Txid;
use bitcoincore_rpc::RpcApi;

use crate::{
    models::withdrawal::{BitcoinWithdrawalResponse, BitcoinWithdrawalStatus},
    state::AppState,
};

pub fn retrieve_submission_status(
    state: &Arc<AppState>,
    sn_txhash: String,
    tx_hex: String,
) -> BitcoinWithdrawalResponse {
    let txid = match Txid::from_str(&tx_hex) {
        Ok(txid) => txid,
        Err(_) => {
            return BitcoinWithdrawalResponse {
                status: BitcoinWithdrawalStatus::InReview,
                sn_txhash,
                rejected: None,
                btc_txid: None,
            }
        }
    };
    // Check the transaction on the network
    if let Ok(tx) = state.bitcoin_provider.get_raw_transaction_info(&txid, None) {
        BitcoinWithdrawalResponse {
            status: BitcoinWithdrawalStatus::Submitted,
            sn_txhash,
            rejected: None,
            btc_txid: Some(tx.txid.to_string()),
        }
    } else {
        BitcoinWithdrawalResponse {
            status: BitcoinWithdrawalStatus::InReview,
            sn_txhash,
            rejected: None,
            btc_txid: None,
        }
    }
}
