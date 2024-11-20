use mongodb::{
    bson::{doc, DateTime},
    ClientSession, Database,
};

use crate::{
    models::{deposit::DepositAddressDocument, runes::SupportedRuneDocument},
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
    async fn is_blacklisted(
        &self,
        session: &mut ClientSession,
        tx_id: String,
    ) -> Result<(), DatabaseError>;
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

        while let Ok(doc) = cursor.next(session).await.expect("Failed to read cursor") {
            res.push(doc);
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
            .collection::<SupportedRuneDocument>("blacklisted_deposits")
            .find_one(doc! {"tx_id": tx_id})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(_) => Ok(()),
            None => Err(DatabaseError::NotFound),
        }
    }
}
