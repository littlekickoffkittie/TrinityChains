//! Mempool for TrinityChain

use crate::blockchain::Sha256Hash;
use crate::crypto::Address;
use crate::error::ChainError;
use crate::transaction::Transaction;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_MEMPOOL_SIZE: usize = 10000; // Max transactions in mempool
const MAX_TX_PER_ADDRESS: usize = 100; // Max transactions per sender address

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolTransaction {
    pub tx: Transaction,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mempool {
    transactions: HashMap<Sha256Hash, MempoolTransaction>,
    #[serde(skip)]
    by_sender: HashMap<Address, Vec<Sha256Hash>>,
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

impl Mempool {
    pub fn new() -> Self {
        Mempool {
            transactions: HashMap::new(),
            by_sender: HashMap::new(),
        }
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), ChainError> {
        if self.transactions.len() >= MAX_MEMPOOL_SIZE {
            self.evict_lowest_fee()?;
        }

        let tx_hash = tx.hash();
        if self.transactions.contains_key(&tx_hash) {
            return Err(ChainError::InvalidTransaction(
                "Transaction already in mempool".to_string(),
            ));
        }

        let sender = match &tx {
            Transaction::Transfer(tx) => tx.sender,
            Transaction::Subdivision(tx) => tx.owner_address,
            Transaction::Coinbase(_) => {
                return Err(ChainError::InvalidTransaction(
                    "Coinbase transactions cannot be in mempool".to_string(),
                ))
            }
        };

        let sender_txs = self.by_sender.entry(sender).or_default();
        if sender_txs.len() >= MAX_TX_PER_ADDRESS {
            return Err(ChainError::InvalidTransaction(
                "Exceeded maximum transactions per address".to_string(),
            ));
        }

        let mempool_tx = MempoolTransaction {
            tx,
            timestamp: Utc::now().timestamp(),
        };

        self.transactions.insert(tx_hash, mempool_tx);
        sender_txs.push(tx_hash);

        Ok(())
    }

    fn evict_lowest_fee(&mut self) -> Result<(), ChainError> {
        if let Some(eviction_candidate) = self
            .transactions
            .values()
            .min_by_key(|tx| tx.tx.fee_area())
            .map(|tx| tx.tx.hash())
        {
            self.remove_transaction(&eviction_candidate);
            Ok(())
        } else {
            Err(ChainError::MempoolFull)
        }
    }

    pub fn get_transactions_by_fee(&self, limit: usize) -> Vec<Transaction> {
        let mut txs: Vec<Transaction> = self
            .transactions
            .values()
            .map(|mtx| mtx.tx.clone())
            .collect();
        txs.sort_by_key(|b| std::cmp::Reverse(b.fee_area()));
        txs.truncate(limit);
        txs
    }

    pub fn remove_transaction(&mut self, tx_hash: &Sha256Hash) {
        if let Some(mempool_tx) = self.transactions.remove(tx_hash) {
            let sender = match &mempool_tx.tx {
                Transaction::Transfer(tx) => tx.sender,
                Transaction::Subdivision(tx) => tx.owner_address,
                Transaction::Coinbase(_) => return,
            };

            if let Some(sender_txs) = self.by_sender.get_mut(&sender) {
                sender_txs.retain(|h| h != tx_hash);
                if sender_txs.is_empty() {
                    self.by_sender.remove(&sender);
                }
            }
        }
    }

    pub fn get_transaction(&self, tx_hash: &Sha256Hash) -> Option<&Transaction> {
        self.transactions.get(tx_hash).map(|mtx| &mtx.tx)
    }

    pub fn get_all_transactions(&self) -> Vec<Transaction> {
        self.transactions
            .values()
            .map(|mtx| mtx.tx.clone())
            .collect()
    }

    pub fn remove_transactions(&mut self, tx_hashes: &[Sha256Hash]) {
        for hash in tx_hashes {
            self.remove_transaction(hash);
        }
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn prune(&mut self, state: &crate::blockchain::TriangleState) {
        let mut invalid_hashes = Vec::new();
        for (hash, mempool_tx) in self.transactions.iter() {
            if mempool_tx.tx.validate(state).is_err() {
                invalid_hashes.push(*hash);
            }
        }

        for hash in invalid_hashes {
            self.remove_transaction(&hash);
        }
    }
}
