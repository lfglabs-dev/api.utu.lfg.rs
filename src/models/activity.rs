use serde::{Deserialize, Serialize};

use super::{
    deposit::Rune,
    runes::{Location, Operation},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockActivity {
    pub limit: u64,
    pub offset: u64,
    pub total: u64,
    pub results: Vec<BlockActivityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockActivityResult {
    pub rune: Rune,
    pub address: Option<String>,
    pub receiver_address: Option<String>,
    pub amount: Option<String>,
    pub operation: Operation,
    pub location: Location,
}
