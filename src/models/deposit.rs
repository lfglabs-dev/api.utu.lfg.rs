use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddressDocument {
    pub starknet_address: String,
    pub bitcoin_deposit_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositDocument {
    pub tx_hash: String,
    pub starknet_address: String,
    pub claimed_tx : Option<String>,
    pub runes_data : Vec<RuneData>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneData {
    pub rune_id: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinAddresses {
    pub addresses: Vec<String>,
    pub amount: f64,
}
