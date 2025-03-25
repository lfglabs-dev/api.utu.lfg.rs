use std::sync::Arc;

use bitcoincore_rpc::RpcApi;
use utu_bridge_types::{bitcoin::BitcoinTxId, starknet::StarknetTxHash};

use crate::{
    models::withdrawal::{BitcoinWithdrawalResponse, BitcoinWithdrawalStatus},
    state::AppState,
};

pub fn retrieve_submission_status(
    state: &Arc<AppState>,
    sn_txhash: StarknetTxHash,
    txid: BitcoinTxId,
) -> BitcoinWithdrawalResponse {
    // Check the transaction on the network
    if state
        .bitcoin_provider
        .get_raw_transaction_info(&txid.to_txid(), None)
        .is_ok()
    {
        BitcoinWithdrawalResponse {
            status: BitcoinWithdrawalStatus::Submitted,
            sn_txhash,
            reason: None,
            btc_txid: Some(txid),
        }
    } else {
        BitcoinWithdrawalResponse {
            status: BitcoinWithdrawalStatus::InReview,
            sn_txhash,
            reason: None,
            btc_txid: None,
        }
    }
}
