use starknet::{
    core::types::FieldElement,
    macros::{felt, selector},
};
use starknet_crypto::poseidon_hash_many;

lazy_static::lazy_static! {
    // 'StarkNet Message'
    static ref STARKNET_MESSAGE: FieldElement = felt!("110930206544689809660069706067448260453");
    // 'Utu Runes Bridge'
    static ref UTU_RUNES_BRIDGE_STR: FieldElement = felt!("113589061680052453361606720447114930021");
    static ref STARKNET_DOMAIN_TYPE_SELECTOR: FieldElement = selector!("\"StarknetDomain\"(\"name\":\"shortstring\",\"version\":\"shortstring\",\"chainId\":\"shortstring\",\"revision\":\"shortstring\")");
    static ref VERSION: FieldElement = FieldElement::ONE;
    static ref REVISION: FieldElement = FieldElement::ONE;
    // 'UtuRunesBridge: Claim'
    static ref UTU_RUNES_BRIDGE_CLAIM_STR: FieldElement = felt!("124892498472897766688382465010089205919870131202413");
    static ref CLAIM_RUNES_TYPE_SELECTOR: FieldElement = selector!("\"ClaimStruct\"(\"Operation\":\"shortstring\",\"Hashed value\":\"felt\")");
}

fn build_starknet_domain_hash(chain_id: FieldElement) -> FieldElement {
    poseidon_hash_many(&[
        *STARKNET_DOMAIN_TYPE_SELECTOR,
        *UTU_RUNES_BRIDGE_STR, // "Utu Runes Bridge"
        *VERSION,
        chain_id,
        *REVISION,
    ])
}

pub fn build_claim_data_hash(
    chain_id: FieldElement,
    rune_id: FieldElement,
    amount: FieldElement,
    starknet_addr: FieldElement,
    tx_id: FieldElement,
) -> FieldElement {
    let hashed = poseidon_hash_many(&[rune_id, amount, starknet_addr, tx_id]);
    let claim_data_hash = poseidon_hash_many(&[
        *CLAIM_RUNES_TYPE_SELECTOR,
        *UTU_RUNES_BRIDGE_CLAIM_STR,
        hashed,
    ]);
    let elements = &[
        *STARKNET_MESSAGE,
        build_starknet_domain_hash(chain_id),
        claim_data_hash,
    ];
    poseidon_hash_many(elements)
}

#[cfg(test)]
mod tests {
    use crate::utils::starknet::to_uint256;

    use super::*;
    use bigdecimal::{num_bigint::BigInt, Num};
    use starknet::core::crypto::ecdsa_sign;

    #[test]
    fn test_compute_hash() {
        let chain_id = felt!("393402133025997798000961");
        println!("chain_id: {:?}", chain_id);
        let priv_key = felt!("0x123");

        let rune_id: FieldElement = felt!("0x95909ff0");
        let amount = (felt!("0x7a120"), FieldElement::ZERO);
        let addr = felt!("0x403c80a49f16ed8ecf751f4b3ad62cc8f85ebeb2d40dc3b4377a089b438995d");

        let tx_deposit_id = "bd51cd6d88a59456e2585c2dd61e51f91645dd071d33484d0015328f460057fc";
        // Digest = [0xfc570046, 0x8f321500, 0x4d48331d, 0x7dd4516, 0xf9511ed6, 0x2d5c58e2, 0x5694a588, 0x6dcd51bd]
        let tx_u256 = to_uint256(BigInt::from_str_radix(tx_deposit_id, 16).unwrap());

        assert_eq!(
            tx_u256,
            (
                felt!("29605767366663658861677795006692218876"),
                felt!("251648833821019018272888897087823827449")
            )
        );

        let hashed = poseidon_hash_many(&[rune_id, amount.0, addr, tx_u256.0]);
        assert_eq!(
            hashed,
            felt!("0x07a6d66b689fda331b65dba000b887cc17796ded88da0c9c3147c7cc3654a6b2")
        );

        let msg_hash = build_claim_data_hash(chain_id, rune_id, amount.0, addr, tx_u256.0);
        assert_eq!(
            msg_hash,
            felt!("0x06d2fdb6e5c22b0f893bb3550bf28219a505a97d843f8cce7cc22023bf8acd5f")
        );

        match ecdsa_sign(&priv_key, &msg_hash) {
            Ok(signature) => {
                println!("Signature: {:?}", signature);
                assert_eq!(
                    signature.r,
                    felt!("0x04854dfe178876b436ab086c05892050d4d00fe0287388512b098b8313ce0c46")
                );
                assert_eq!(
                    signature.s,
                    felt!("0x00e810eb6a6309b2b9175e12d9d9063a224d5489f75696fbdb185e2e172abb26")
                );
            }
            Err(e) => {
                println!("Error while generating signature: {}", e);
            }
        };
    }

    #[test]
    fn test_get_sig_contract() {
        let chain_id = felt!("393402133025997798000961");
        println!("chain_id: {:?}", chain_id);
        let priv_key = felt!("0x123");

        let rune_id: FieldElement = felt!("97");
        let amount = (felt!("25000"), FieldElement::ZERO);
        let addr = felt!("504447201841");
        let tx_u256 = (
            felt!("273581432376733134300661245883050715131"),
            FieldElement::ZERO,
        );

        let hashed = poseidon_hash_many(&[rune_id, amount.0, addr, tx_u256.0]);
        assert_eq!(
            hashed,
            felt!("0x02e3b9c42da5734902894f646ee4b33311e1bbd506787f24bf5918f99164daac")
        );

        let msg_hash = build_claim_data_hash(chain_id, rune_id, amount.0, addr, tx_u256.0);
        assert_eq!(
            msg_hash,
            felt!("0x07241f8c2d1ededa866f604ac8f73845517687c960a309ab087459407e3a53a9")
        );

        match ecdsa_sign(&priv_key, &msg_hash) {
            Ok(signature) => {
                println!("Signature: {:?}", signature);
            }
            Err(e) => {
                println!("Error while generating signature: {}", e);
            }
        };
    }
}
