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
    async fn get_deposits_bitcoin(
        &self,
        session: &mut ClientSession,
        bitcoin_deposit_address: String,
    ) -> Result<Vec<DepositAddressDocument>, DatabaseError>;
    async fn get_deposits_starknet(
        &self,
        session: &mut ClientSession,
        starknet_deposit_addr: Address,
    ) -> Result<Vec<DepositAddressDocument>, DatabaseError>;
    async fn get_starknet_addr_from_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        bitcoin_deposit_address: String,
    ) -> Result<Address, DatabaseError>;
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
        bitcoin_addr: String
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

    async fn get_deposits_bitcoin(
        &self,
        session: &mut ClientSession,
        bitcoin_sender_address: String,
    ) -> Result<Vec<DepositAddressDocument>, DatabaseError> {
        let mut cursor = self
            .collection::<DepositAddressDocument>("deposits")
            .find(doc! {"bitcoin_sender_address": bitcoin_sender_address })
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut res: Vec<DepositAddressDocument> = Vec::new();

        while let Ok(doc) = cursor.next(session).await.expect("Failed to read cursor") {
            res.push(doc);
        }

        Ok(res)
    }

    async fn get_deposits_starknet(
        &self,
        session: &mut ClientSession,
        starknet_deposit_addr: Address,
    ) -> Result<Vec<DepositAddressDocument>, DatabaseError> {
        let mut cursor = self
            .collection::<DepositAddressDocument>("deposits")
            .find(doc! {"starknet_address": starknet_deposit_addr.to_string() })
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut res: Vec<DepositAddressDocument> = Vec::new();

        while let Ok(doc) = cursor.next(session).await.expect("Failed to read cursor") {
            res.push(doc);
        }

        Ok(res)
    }

    async fn get_starknet_addr_from_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        bitcoin_deposit_address: String,
    ) -> Result<Address, DatabaseError> {
        let result = self
            .collection::<DepositAddressDocument>("deposit_addresses")
            .find_one(doc! {"bitcoin_deposit_address": bitcoin_deposit_address})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(doc) => Address::from_str(&doc.starknet_address).map_err(DatabaseError::Other),
            None => Err(DatabaseError::NotFound),
        }
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
        bitcoin_addr: String
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
}
