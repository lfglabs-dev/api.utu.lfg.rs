use std::{env, str::FromStr, sync::Arc};

use anyhow::Result;
use reqwest::Url;
use starknet::{
    accounts::{Account, Call, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, FieldElement},
        utils::{get_udc_deployed_address, UdcUniqueness},
    },
    macros::selector,
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
    signers::{LocalWallet, SigningKey},
};

use crate::state::AppState;

lazy_static::lazy_static! {
    static ref SAG_CLASS_HASH: FieldElement = FieldElement::from_hex_be(&env::var("SAG_CLASS_HASH").expect("SAG_CLASS_HASH must be set")).unwrap();
    static ref RUNE_BRIDGE_CONTRACT: FieldElement = FieldElement::from_hex_be(&env::var("RUNE_BRIDGE_CONTRACT").expect("RUNE_BRIDGE_CONTRACT must be set")).unwrap();
}

pub async fn get_account() -> SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet> {
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(&env::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL must be set")).unwrap(),
    ));
    let chainid = provider.chain_id().await.unwrap();
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(
            &env::var("ACCOUNT_PRIV_KEY").expect("ACCOUNT_PRIV_KEY must be set"),
        )
        .unwrap(),
    ));
    SingleOwnerAccount::new(
        provider,
        signer,
        FieldElement::from_hex_be(
            &env::var("ACCOUNT_ADDRESS").expect("ACCOUNT_ADDRESS must be set"),
        )
        .unwrap(),
        chainid,
        starknet::accounts::ExecutionEncoding::New,
    )
}

pub fn compute_rune_contract(rune_id: String) -> FieldElement {
    let calldata: Vec<FieldElement> = vec![FieldElement::from_str(&rune_id).unwrap()];

    get_udc_deployed_address(
        FieldElement::ZERO,
        *SAG_CLASS_HASH,
        &UdcUniqueness::NotUnique,
        &calldata,
    )
}

pub async fn is_deployed_on_starknet(
    state: &Arc<AppState>,
    contract_address: FieldElement,
) -> Result<()> {
    let _ = state
        .starknet_provider
        .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
        .await?;
    Ok(())
}

pub async fn deploy_rune_contract(
    state: &Arc<AppState>,
    rune_id: String,
    rune_name: String,
    rune_symbol: String,
) -> Result<FieldElement> {
    let mut calldata: Vec<FieldElement> = vec![FieldElement::from_str(&rune_id).unwrap()];
    calldata.push(FieldElement::from(rune_name.len()));
    calldata.extend(rune_name.chars().map(|ch| FieldElement::from(ch as u8)));
    calldata.push(FieldElement::from(rune_symbol.len()));
    calldata.extend(rune_symbol.chars().map(|ch| FieldElement::from(ch as u8)));

    let call = Call {
        to: *RUNE_BRIDGE_CONTRACT,
        selector: selector!("create_rune"),
        calldata,
    };

    let execution = state
        .starknet_account
        .execute(vec![call])
        .fee_estimate_multiplier(5.0f64);
    match execution.estimate_fee().await {
        Ok(_) => match execution
            // harcode max fee to 0.0040 ETH
            .max_fee(FieldElement::from(4000000000000000_u64))
            .send()
            .await
        {
            Ok(tx_result) => Ok(tx_result.transaction_hash),
            Err(e) => {
                let error_message = format!("An error occurred while creating rune: {}", e);
                Err(anyhow::anyhow!(error_message))
            }
        },
        Err(e) => {
            let error_message =
                format!("An error occurred while simulating create_rune call: {}", e);

            Err(anyhow::anyhow!(error_message))
        }
    }
}
