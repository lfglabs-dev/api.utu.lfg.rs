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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DepositStatus {
    Pending,
    Confirmed,
    Claimed,
}
