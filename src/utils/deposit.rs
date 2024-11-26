use std::env;
use std::str::FromStr;

use super::Address;
use bitcoin::{bip32::Xpub, secp256k1::Secp256k1};
use bitcoin::{Address as BitcoinAddress, Network};
use utu_bridge_deposit_address::{
    compute_tweak, derive_internal_public_key, generate_taproot_output_key,
};

lazy_static::lazy_static! {
    static ref BITCOIN_PUB_KEY: Xpub =   Xpub::from_str(&env::var("BITCOIN_PUB_KEY")
    .expect("BITCOIN_PUB_KEY must be set")).expect("Invalid Bitcoin public key");
}

pub fn get_bitcoin_addr_from_starknet_addr(starknet_deposit_address: Address) -> String {
    let secp = Secp256k1::new();
    let internal_key = derive_internal_public_key(&BITCOIN_PUB_KEY).unwrap();

    // Derive tweak for starknet_deposit_address
    let nonce: [u8; 32] = starknet_deposit_address.felt.to_bytes_be();
    let tweak = compute_tweak(&internal_key, &nonce);

    // Generate taproot output key and address
    let (taproot_key, _parity) =
        generate_taproot_output_key(&internal_key.public_key, &tweak).unwrap();
    let deposit_address = BitcoinAddress::p2tr(&secp, taproot_key, None, Network::Bitcoin);
    deposit_address.to_string()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bitcoin::{
        bip32::{Xpriv, Xpub},
        key::Secp256k1,
        Network,
    };

    #[test]
    fn test_generate_pub_key() {
        // Generate a new random keypair
        let secp = Secp256k1::new();
        let master_private_key = Xpriv::new_master(Network::Bitcoin, &[1u8; 32]).unwrap();
        let master_public_key = Xpub::from_priv(&secp, &master_private_key);

        // Convert into string
        let serialized_pub_key = master_public_key.to_string();
        println!("serialized_pub_key: {:?}", serialized_pub_key);

        // Convert back into Xpub and assert equality
        let recovered_key = Xpub::from_str(&serialized_pub_key).unwrap();
        assert!(recovered_key == master_public_key);
    }
}
