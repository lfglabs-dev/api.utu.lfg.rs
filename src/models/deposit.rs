use serde::{Deserialize, Serialize};
use utu_bridge_types::{
    bitcoin::{BitcoinAddress, BitcoinTxId},
    starknet::{StarknetAddress, StarknetTxHash},
    ClaimedRunesDepositsDocument, DepositClaimTxsDocument,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rune {
    pub id: String,
    pub name: String,
    pub spaced_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DepositStatus {
    Pending,
    Confirmed,
    Claimed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinDepositQuery {
    pub starknet_address: StarknetAddress,
    pub bitcoin_deposit_address: BitcoinAddress,
    pub claimed_deposits: Option<ClaimedRunesDepositsDocument>,
    pub deposit_claim_txs: Vec<DepositClaimTxsDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinDepositEntry {
    pub btc_txid: BitcoinTxId,
    pub btc_txvout: u32,
    pub sn_txhash: Option<StarknetTxHash>,
}
