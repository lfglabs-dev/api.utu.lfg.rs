use mongodb::{bson::{doc, DateTime}, ClientSession, Database};

use crate::{
    models::deposit::{DepositAddressDocument, DepositDocument},
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
    async fn set_deposit(
        &self,
        session: &mut ClientSession,
        tx_hash: String,
        bitcoin_sending_address: String,
        starknet_receiving_address: Address,
        claimed: bool,
        verification_status: String,
    ) -> Result<(), DatabaseError>;
    async fn get_starknet_addr_from_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        bitcoin_deposit_address: String,
    ) -> Result<Address, DatabaseError>;
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

    async fn set_deposit(
        &self,
        session: &mut ClientSession,
        tx_hash: String,
        bitcoin_sending_address: String,
        starknet_receiving_address: Address,
        claimed: bool,
        verification_status: String,
    ) -> Result<(), DatabaseError> {
        self.collection::<DepositDocument>("deposits")
            .update_one(
                doc! {
                    "tx_hash": tx_hash,
                    "starknet_receiving_address": starknet_receiving_address.to_string(),
                    "bitcoin_sending_address": bitcoin_sending_address
                },
                doc! {
                    "$set":
                    {
                        "claimed": claimed,
                        "verification_status": verification_status,
                    }
                },
            )
            .upsert(true)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;
        Ok(())
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
}
