use mongodb::{
    bson::{doc, DateTime},
    ClientSession, Database,
};

use crate::{
    models::{
        deposit::{
            BlacklistedDeposit, DepositAddressDocument, DepositClaimTxDocument, DepositDocument,
        },
        runes::SupportedRuneDocument,
    },
    utils::Address,
};

use super::DatabaseError;

pub trait DatabaseExt {
    async fn set_user_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        starknet_addr: Address,
        bitcoin_addr: String,
    ) -> Result<(), DatabaseError>;
    async fn get_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        starknet_addr: Address,
    ) -> Result<String, DatabaseError>;
    async fn get_supported_runes(
        &self,
        session: &mut ClientSession,
    ) -> Result<Vec<SupportedRuneDocument>, DatabaseError>;
    async fn is_deposit_addr(
        &self,
        session: &mut ClientSession,
        bitcoin_addr: String,
    ) -> Result<(), DatabaseError>;
    async fn get_rune(
        &self,
        session: &mut ClientSession,
        rune_id: String,
    ) -> Result<SupportedRuneDocument, DatabaseError>;
    #[allow(dead_code)]
    async fn is_blacklisted(
        &self,
        session: &mut ClientSession,
        tx_id: String,
    ) -> Result<(), DatabaseError>;
    async fn was_claimed(
        &self,
        session: &mut ClientSession,
        tx_id: String,
        vout: Option<u64>,
    ) -> Result<String, DatabaseError>;
    async fn store_deposit(
        &self,
        session: &mut ClientSession,
        deposit: DepositDocument,
    ) -> Result<(), DatabaseError>;
    async fn get_deposit_claim_txhash(
        &self,
        session: &mut ClientSession,
        btc_txid: String,
    ) -> Result<String, DatabaseError>;
}

impl DatabaseExt for Database {
    async fn set_user_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        starknet_addr: Address,
        bitcoin_addr: String,
    ) -> Result<(), DatabaseError> {
        self.collection::<DepositAddressDocument>("deposit_addresses")
            .update_one(
                doc! {"starknet_address": starknet_addr.to_string() },
                doc! {
                    "$set":
                    {
                        "bitcoin_deposit_address": bitcoin_addr,
                        "created_at": DateTime::now(),
                    }
                },
            )
            .upsert(true)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;
        Ok(())
    }

    async fn get_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        starknet_addr: Address,
    ) -> Result<String, DatabaseError> {
        let result = self
            .collection::<DepositAddressDocument>("deposit_addresses")
            .find_one(doc! {"starknet_address": starknet_addr.to_string()})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(doc) => Ok(doc.bitcoin_deposit_address),
            None => Err(DatabaseError::NotFound),
        }
    }
    async fn get_supported_runes(
        &self,
        session: &mut ClientSession,
    ) -> Result<Vec<SupportedRuneDocument>, DatabaseError> {
        let mut cursor = self
            .collection::<SupportedRuneDocument>("runes")
            .find(doc! {})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut res: Vec<SupportedRuneDocument> = Vec::new();

        while let Some(doc_result) = cursor.next(session).await {
            match doc_result {
                Ok(doc) => res.push(doc),
                Err(err) => return Err(DatabaseError::QueryFailed(err)),
            }
        }

        Ok(res)
    }

    async fn is_deposit_addr(
        &self,
        session: &mut ClientSession,
        bitcoin_addr: String,
    ) -> Result<(), DatabaseError> {
        let result = self
            .collection::<DepositAddressDocument>("deposit_addresses")
            .find_one(doc! {"bitcoin_deposit_address": bitcoin_addr})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(_) => Ok(()),
            None => Err(DatabaseError::NotFound),
        }
    }
    async fn get_rune(
        &self,
        session: &mut ClientSession,
        rune_id: String,
    ) -> Result<SupportedRuneDocument, DatabaseError> {
        let result = self
            .collection::<SupportedRuneDocument>("runes")
            .find_one(doc! {"id": rune_id})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(doc) => Ok(doc),
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn is_blacklisted(
        &self,
        session: &mut ClientSession,
        tx_id: String,
    ) -> Result<(), DatabaseError> {
        let result = self
            .collection::<BlacklistedDeposit>("blacklisted_deposits")
            .find_one(doc! {"tx_id": tx_id})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(_) => Ok(()),
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn was_claimed(
        &self,
        session: &mut ClientSession,
        tx_id: String,
        vout: Option<u64>,
    ) -> Result<String, DatabaseError> {
        let identifier = match vout {
            Some(vout) => format!("{}:{}", tx_id, vout),
            None => return Err(DatabaseError::Other("vout is None".to_string())),
        };
        let result = self
            .collection::<DepositClaimTxDocument>("deposit_claim_txs")
            .find_one(doc! {"identifier": identifier, "_cursor.to": null })
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(claim_tx) => Ok(claim_tx.transaction_hash),
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn store_deposit(
        &self,
        session: &mut ClientSession,
        deposit: DepositDocument,
    ) -> Result<(), DatabaseError> {
        self.collection::<DepositDocument>("claimed_runes_deposits")
            .update_one(
                doc! {"identifier": deposit.identifier },
                doc! {
                    "$set":
                    {
                        "tx_id": &deposit.tx_id,
                        "vout": deposit.vout,
                        "rune": {
                            "id": &deposit.rune.id,
                            "name": &deposit.rune.name,
                            "spaced_name": &deposit.rune.spaced_name,
                        },
                        "amount": &deposit.amount,
                        "bitcoin_deposit_addr": &deposit.bitcoin_deposit_addr,
                    }
                },
            )
            .upsert(true)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        Ok(())
    }

    async fn get_deposit_claim_txhash(
        &self,
        session: &mut ClientSession,
        btc_txid: String,
    ) -> Result<String, DatabaseError> {
        let result = self
            .collection::<DepositClaimTxDocument>("deposit_claim_txs")
            .find_one(doc! {"identifier": {
                "$regex": format!(r"^{}:[0-9]+$", btc_txid),
                "$options": "i"
            }, "_cursor.to": null })
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(claim_tx) => Ok(claim_tx.transaction_hash),
            None => Err(DatabaseError::NotFound),
        }
    }
}
