use serde::{Deserialize, Serialize};

use super::runes::{RuneActivity, SupportedRuneDocument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddressDocument {
    pub starknet_address: String,
    pub bitcoin_deposit_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositDocument {
    pub identifier: String,
    pub tx_id: String,
    pub vout: u32,
    pub rune: Rune,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rune {
    pub id: String,
    pub name: String,
    pub spaced_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinAddresses {
    pub addresses: Vec<String>,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositActivityDetails {
    pub rune: SupportedRuneDocument,
    pub tx: RuneActivity,
    pub claim_tx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DepositStatus {
    Pending,
    Confirmed,
    Claimed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistedDeposit {
    pub tx_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositClaimTxDocument {
    pub identifier: String,
    pub rune_id: String,
    pub amount: String,
    pub caller_address: String,
    pub target_address: String,
    pub transaction_hash: String,
    pub _cursor: Cursor,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cursor {
    pub to: Option<i64>,
    pub from: Option<i64>,
}
