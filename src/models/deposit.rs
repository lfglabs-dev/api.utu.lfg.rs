use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

use super::runes::{RuneActivity, SupportedRuneDocument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddressDocument {
    pub starknet_address: String,
    pub bitcoin_deposit_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositDocument {
    pub starknet_address: String,
    pub bitcoin_deposit_address: String,
    pub tx_id: String,
    pub rune: Rune,
    pub amount: String,
    pub claimed: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
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
