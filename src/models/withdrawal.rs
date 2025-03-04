use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalRequest {
    pub identifier: String,
    pub rune_id: String,
    pub amount: String,
    pub target_bitcoin_address: String,
    pub caller_address: String,
    pub transaction_hash: String,
    pub _cursor: Cursor,
    pub rejected_status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cursor {
    pub to: Option<i64>,
    pub from: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalSubmission {
    pub identifier: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalStatusResponse {
    pub identifier: String,
    pub transaction_hash: String,
    pub rejected_status: Option<String>,
    pub matched_submissions: Option<WithdrawalSubmission>,
}

#[derive(Debug, Serialize)]
pub struct BitcoinWithdrawalResponse {
    pub status: BitcoinWithdrawalStatus,
    pub sn_txhash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btc_txid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BitcoinWithdrawalStatus {
    InReview,
    Submitted,
    Rejected,
}
