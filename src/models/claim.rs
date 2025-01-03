use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

use crate::utils::Address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub r: FieldElement,
    pub s: FieldElement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimData {
    pub rune_id: FieldElement,
    pub amount: (FieldElement, FieldElement),
    pub target_addr: Address,
    pub tx_id: String,
    pub sig: Signature,
}
