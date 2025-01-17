use std::collections::HashMap;

use mongodb::{
    bson::{doc, from_document, DateTime},
    ClientSession, Database,
};

use crate::{
    models::{
        deposit::{
            BitcoinDepositEntry, BitcoinDepositQuery, BlacklistedDeposit, DepositAddressDocument,
            DepositClaimTxDocument, DepositClaimTxHashDocument, DepositDocument,
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
    async fn get_bitcoin_deposits(
        &self,
        session: &mut ClientSession,
        starknet_receiving_addresses: Vec<String>,
    ) -> Result<HashMap<String, Vec<BitcoinDepositEntry>>, DatabaseError>;
    async fn get_deposit_claim_txhash(
        &self,
        session: &mut ClientSession,
        btc_txid: String,
    ) -> Result<String, DatabaseError>;
    async fn get_starknet_addrs(
        &self,
        session: &mut ClientSession,
        bitcoin_addresses: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>, DatabaseError>;
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
        let pipeline = vec![
            doc! {
                "$match": {
                    "tx_id": btc_txid
                }
            },
            doc! {
                "$lookup": {
                    "from": "deposit_claim_txs",
                    "let": { "vout_value": "$vout", "tx_id": "$tx_id" },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$and": [
                                        { "$eq": ["$identifier", { "$concat": ["$$tx_id", ":", { "$toString": "$$vout_value" }] }] },
                                        { "$eq": ["$_cursor.to", null] }
                                    ]
                                }
                            }
                        }
                    ],
                    "as": "matched_txs"
                }
            },
            doc! {
                "$unwind": {
                    "path": "$matched_txs",
                    "preserveNullAndEmptyArrays": true
                }
            },
            doc! {
                "$project": {
                    "_id": 0,
                    "tx_id": 1,
                    "vout": 1,
                    "matched_tx": "$matched_txs"
                }
            },
        ];

        let mut cursor = self
            .collection::<DepositDocument>("claimed_runes_deposits")
            .aggregate(pipeline)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        if let Some(doc) = cursor.next(&mut *session).await {
            let data: DepositClaimTxHashDocument =
                from_document(doc.map_err(DatabaseError::QueryFailed)?)
                    .map_err(DatabaseError::DeserializationFailed)?;
            Ok(data.matched_tx.transaction_hash)
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    async fn get_bitcoin_deposits(
        &self,
        session: &mut ClientSession,
        starknet_receiving_addresses: Vec<String>,
    ) -> Result<HashMap<String, Vec<BitcoinDepositEntry>>, DatabaseError> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "starknet_address": { "$in": starknet_receiving_addresses }
                }
            },
            doc! {
                "$lookup": {
                    "from": "claimed_runes_deposits",
                    "localField": "bitcoin_deposit_address",
                    "foreignField": "bitcoin_deposit_addr",
                    "as": "claimed_deposits"
                }
            },
            doc! {
                "$unwind": {
                    "path": "$claimed_deposits",
                    "preserveNullAndEmptyArrays": true
                }
            },
            // Add identifier as tx_id:vout for the next lookup
            doc! {
                "$addFields": {
                    "claimed_deposits": {
                        "$cond": {
                            "if": { "$not": ["$claimed_deposits"] },
                            "then": null,
                            "else": {
                                "$mergeObjects": [
                                    "$claimed_deposits", // Retain all original fields
                                    {
                                        "identifier": {
                                            "$concat": [
                                                "$claimed_deposits.tx_id",
                                                ":",
                                                { "$toString": "$claimed_deposits.vout" }
                                            ]
                                        }
                                    }
                                ]
                            },
                        }
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "deposit_claim_txs",
                    "let": { "identifier": "$claimed_deposits.identifier" },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$and": [
                                        { "$eq": ["$identifier", "$$identifier"] },
                                        { "$eq": ["$_cursor.to", null] }
                                    ]
                                }
                            }
                        }
                    ],
                    "as": "deposit_claim_txs"
                }
            },
            doc! {
                "$project": {
                    "_id": 0,
                    "starknet_address": 1,
                    "bitcoin_deposit_address": 1,
                    "claimed_deposits": 1,
                    "deposit_claim_txs": 1
                }
            },
        ];
        let mut cursor = self
            .collection::<DepositDocument>("deposit_addresses")
            .aggregate(pipeline)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut results: HashMap<String, Vec<BitcoinDepositEntry>> = HashMap::new();
        while let Some(doc) = cursor.next(&mut *session).await {
            let data: BitcoinDepositQuery = from_document(doc.map_err(DatabaseError::QueryFailed)?)
                .map_err(DatabaseError::DeserializationFailed)?;

            results.entry(data.starknet_address.clone()).or_default();

            if data.claimed_deposits.is_some() {
                let btc_txid = data.claimed_deposits.unwrap().tx_id;
                let sn_txhash = data
                    .deposit_claim_txs
                    .first()
                    .map(|claim_tx| claim_tx.transaction_hash.clone());
                results
                    .entry(data.starknet_address)
                    .or_default()
                    .push(BitcoinDepositEntry {
                        btc_txid,
                        sn_txhash,
                    });
            }
        }

        Ok(results)
    }

    async fn get_starknet_addrs(
        &self,
        session: &mut ClientSession,
        bitcoin_addresses: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>, DatabaseError> {
        let mut cursor = self
            .collection::<DepositAddressDocument>("deposit_addresses")
            .find(doc! {"bitcoin_deposit_address": { "$in": bitcoin_addresses.clone() }})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut results: HashMap<String, Option<String>> = HashMap::new();

        // Initialize results with nulls for all bitcoin_addresses
        for addr in &bitcoin_addresses {
            results.insert(addr.clone(), None);
        }
        while let Some(doc) = cursor.next(&mut *session).await {
            let data = doc.map_err(DatabaseError::QueryFailed)?;
            results.insert(data.bitcoin_deposit_address, Some(data.starknet_address));
        }

        Ok(results)
    }
}
