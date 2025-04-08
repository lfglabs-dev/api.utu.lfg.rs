use std::collections::HashMap;

use mongodb::{
    bson::{doc, from_document, DateTime},
    ClientSession, Database,
};
use utu_bridge_types::{
    bitcoin::{BitcoinAddress, BitcoinOutpoint},
    starknet::{StarknetAddress, StarknetTxHash},
    DepositAddressesDocument, DepositClaimTxsDocument, WithdrawalRequestsDocument,
};

use crate::models::{
    deposit::{BitcoinDepositEntry, BitcoinDepositQuery},
    withdrawal::WithdrawalStatusResponse,
};

use super::DatabaseError;

pub trait DatabaseExt {
    async fn set_user_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        starknet_addr: StarknetAddress,
        bitcoin_addr: BitcoinAddress,
    ) -> Result<(), DatabaseError>;
    async fn get_bitcoin_deposits(
        &self,
        session: &mut ClientSession,
        starknet_receiving_addresses: Vec<StarknetAddress>,
    ) -> Result<HashMap<StarknetAddress, Vec<BitcoinDepositEntry>>, DatabaseError>;
    async fn get_deposit_claim_txhash(
        &self,
        session: &mut ClientSession,
        btc_utxo_id: BitcoinOutpoint,
    ) -> Result<StarknetTxHash, DatabaseError>;
    async fn get_starknet_addrs(
        &self,
        session: &mut ClientSession,
        bitcoin_addresses: Vec<BitcoinAddress>,
    ) -> Result<HashMap<BitcoinAddress, Option<StarknetAddress>>, DatabaseError>;
    async fn get_withdrawal_status(
        &self,
        session: &mut ClientSession,
        sn_txhash: StarknetTxHash,
    ) -> Result<WithdrawalStatusResponse, DatabaseError>;
    async fn get_bitcoin_withdrawals(
        &self,
        session: &mut ClientSession,
        bitcoin_receiving_address: Option<BitcoinAddress>,
        starknet_sending_address: Option<StarknetAddress>,
    ) -> Result<Vec<WithdrawalStatusResponse>, DatabaseError>;
}

impl DatabaseExt for Database {
    async fn set_user_bitcoin_deposit_addr(
        &self,
        session: &mut ClientSession,
        starknet_addr: StarknetAddress,
        bitcoin_addr: BitcoinAddress,
    ) -> Result<(), DatabaseError> {
        self.collection::<DepositAddressesDocument>("deposit_addresses")
            .update_one(
                doc! {"starknet_address": starknet_addr.as_str() },
                doc! {
                    "$set":
                    {
                        "bitcoin_deposit_address": bitcoin_addr.as_str(),
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

    async fn get_deposit_claim_txhash(
        &self,
        session: &mut ClientSession,
        btc_utxo_id: BitcoinOutpoint,
    ) -> Result<StarknetTxHash, DatabaseError> {
        let result = self
            .collection::<DepositClaimTxsDocument>("deposit_claim_txs")
            .find_one(doc! {"identifier": btc_utxo_id.to_string(), "_cursor.to": null  })
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        match result {
            Some(doc) => Ok(doc.transaction_hash),
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn get_bitcoin_deposits(
        &self,
        session: &mut ClientSession,
        starknet_receiving_addresses: Vec<StarknetAddress>,
    ) -> Result<HashMap<StarknetAddress, Vec<BitcoinDepositEntry>>, DatabaseError> {
        let starknet_addresses_arr: Vec<String> = starknet_receiving_addresses
            .iter()
            .map(|addr| addr.as_str().to_string())
            .collect();
        let pipeline = vec![
            doc! {
                "$match": {
                    "starknet_address": { "$in": starknet_addresses_arr }
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
            .collection::<DepositAddressesDocument>("deposit_addresses")
            .aggregate(pipeline)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut results: HashMap<StarknetAddress, Vec<BitcoinDepositEntry>> = HashMap::new();
        while let Some(doc) = cursor.next(&mut *session).await {
            let data: BitcoinDepositQuery = from_document(doc.map_err(DatabaseError::QueryFailed)?)
                .map_err(DatabaseError::DeserializationFailed)?;

            results.entry(data.starknet_address).or_default();

            if data.claimed_deposits.is_some() {
                let claimed_deposits = data.claimed_deposits.unwrap();
                let btc_txid = claimed_deposits.tx_id;
                let btc_txvout = claimed_deposits.vout;
                let sn_txhash = data
                    .deposit_claim_txs
                    .first()
                    .map(|claim_tx| claim_tx.transaction_hash);
                results
                    .entry(data.starknet_address)
                    .or_default()
                    .push(BitcoinDepositEntry {
                        btc_txid,
                        btc_txvout,
                        sn_txhash,
                    });
            }
        }

        Ok(results)
    }

    async fn get_starknet_addrs(
        &self,
        session: &mut ClientSession,
        bitcoin_addresses: Vec<BitcoinAddress>,
    ) -> Result<HashMap<BitcoinAddress, Option<StarknetAddress>>, DatabaseError> {
        let bitcoin_addresses_arr: Vec<String> = bitcoin_addresses
            .iter()
            .map(|addr| addr.as_str().to_string())
            .collect();
        let mut cursor = self
            .collection::<DepositAddressesDocument>("deposit_addresses")
            .find(doc! {"bitcoin_deposit_address": { "$in": bitcoin_addresses_arr.clone() }})
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut results: HashMap<BitcoinAddress, Option<StarknetAddress>> = HashMap::new();

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

    async fn get_withdrawal_status(
        &self,
        session: &mut ClientSession,
        sn_txhash: StarknetTxHash,
    ) -> Result<WithdrawalStatusResponse, DatabaseError> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "transaction_hash": sn_txhash.to_string(),
                    "_cursor.to": { "$eq": null }
                }
            },
            doc! {
                "$lookup": {
                    "from": "withdrawal_submissions",
                    "localField": "identifier",
                    "foreignField": "identifier",
                    "as": "matched_submissions"
                }
            },
            doc! {
                "$unwind": {
                    "path": "$matched_submissions",
                    "preserveNullAndEmptyArrays": true
                }
            },
            doc! {
                "$project": {
                    "_id": 0,
                    "identifier": 1,
                    "transaction_hash": 1,
                    "matched_submissions": 1
                }
            },
        ];
        let mut cursor = self
            .collection::<WithdrawalRequestsDocument>("withdrawal_requests")
            .aggregate(pipeline)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        if let Some(doc) = cursor.next(&mut *session).await {
            let data: WithdrawalStatusResponse =
                from_document(doc.map_err(DatabaseError::QueryFailed)?)
                    .map_err(DatabaseError::DeserializationFailed)?;
            return Ok(data);
        }
        Err(DatabaseError::NotFound)
    }

    async fn get_bitcoin_withdrawals(
        &self,
        session: &mut ClientSession,
        bitcoin_receiving_address: Option<BitcoinAddress>,
        starknet_sending_address: Option<StarknetAddress>,
    ) -> Result<Vec<WithdrawalStatusResponse>, DatabaseError> {
        let mut match_stage = doc! {
            "_cursor.to": { "$eq": null }
        };

        if let Some(address) = bitcoin_receiving_address {
            match_stage.insert("target_bitcoin_address", address.as_str());
        } else if let Some(address) = starknet_sending_address {
            match_stage.insert("caller_address", address.as_str());
        }

        let pipeline = vec![
            doc! { "$match": match_stage },
            doc! {
                "$lookup": {
                    "from": "withdrawal_submissions",
                    "localField": "identifier",
                    "foreignField": "identifier",
                    "as": "matched_submissions"
                }
            },
            doc! {
                "$unwind": {
                    "path": "$matched_submissions",
                    "preserveNullAndEmptyArrays": true
                }
            },
            doc! {
                "$project": {
                    "_id": 0,
                    "identifier": 1,
                    "transaction_hash": 1,
                    "matched_submissions": 1
                }
            },
        ];
        let mut cursor = self
            .collection::<WithdrawalRequestsDocument>("withdrawal_requests")
            .aggregate(pipeline)
            .session(&mut *session)
            .await
            .map_err(DatabaseError::QueryFailed)?;

        let mut res: Vec<WithdrawalStatusResponse> = Vec::new();
        while let Some(doc) = cursor.next(&mut *session).await {
            let data: WithdrawalStatusResponse =
                from_document(doc.map_err(DatabaseError::QueryFailed)?)
                    .map_err(DatabaseError::DeserializationFailed)?;
            res.push(data);
        }
        Ok(res)
    }
}
