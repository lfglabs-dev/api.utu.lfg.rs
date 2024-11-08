use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddressDocument {
    pub starknet_address: String,
    pub bitcoin_deposit_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositDocument {
    pub tx_hash: String,
    pub bitcoin_sending_address: String,
    pub starknet_receiving_address: String,
    pub claimed: bool,
    pub verification_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinAddresses {
    pub addresses: Vec<String>,
    pub amount: f64,
}
