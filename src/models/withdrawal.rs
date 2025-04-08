use serde::{Deserialize, Serialize};
use utu_bridge_types::{
    bitcoin::{BitcoinOutpoint, BitcoinTxId},
    starknet::StarknetTxHash,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalSubmission {
    pub identifier: BitcoinOutpoint,
    pub request_id: Option<BitcoinTxId>,
    pub rejected_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalStatusResponse {
    pub identifier: BitcoinOutpoint,
    pub transaction_hash: StarknetTxHash,
    pub matched_submissions: Option<WithdrawalSubmission>,
}

#[derive(Debug, Serialize)]
pub struct BitcoinWithdrawalResponse {
    pub status: BitcoinWithdrawalStatus,
    pub sn_txhash: StarknetTxHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btc_txid: Option<BitcoinTxId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BitcoinWithdrawalStatus {
    InReview,
    Submitted,
    Rejected,
}
