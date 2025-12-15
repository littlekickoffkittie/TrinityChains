//! Core blockchain implementation for TrinityChain, including block structure,
//! chain validation, UTXO management, and mining difficulty adjustment.

use crate::error::ChainError;
use crate::geometry::{Coord, Point, Triangle, GEOMETRIC_TOLERANCE};
use crate::mempool::Mempool;
use crate::miner::mine_block;
use crate::crypto::Address;
use crate::transaction::{CoinbaseTx, Transaction, TransferTx};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ============================================================================
// Constants
// ============================================================================

/// The number of blocks after which to adjust the difficulty.
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10;
/// The desired time between blocks in seconds.
pub const TARGET_BLOCK_TIME: u64 = 30;

// ============================================================================
// Types
// ============================================================================

/// A 32-byte hash (SHA-256)
pub type Sha256Hash = [u8; 32];

/// A single block header
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub previous_hash: Sha256Hash,
    pub merkle_root: Sha256Hash,
    pub difficulty: u32,
    pub nonce: u64,
}

impl BlockHeader {
    /// Calculate the hash of the block header
    pub fn hash(&self) -> Sha256Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.previous_hash);
        hasher.update(self.merkle_root);
        hasher.update(self.difficulty.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.finalize().into()
    }
}

/// A complete block
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block candidate before mining
    pub fn new(
        height: u64,
        previous_hash: Sha256Hash,
        difficulty: u32,
        transactions: Vec<Transaction>,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let merkle_root = Block::calculate_merkle_root(&transactions);

        Block {
            header: BlockHeader {
                height,
                timestamp,
                previous_hash,
                merkle_root,
                difficulty,
                nonce: 0,
            },
            transactions,
        }
    }

    /// Calculate the SHA-256 hash of the block (header hash)
    pub fn hash(&self) -> Sha256Hash {
        self.header.hash()
    }

    /// Standard Merkle Root calculation (hashing all transaction hashes together)
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Sha256Hash {
        let mut hasher = Sha256::new();
        for tx in transactions {
            hasher.update(tx.hash());
        }
        hasher.finalize().into()
    }

    pub fn hash_as_u256(hash: &Sha256Hash) -> [u8; 32] {
        *hash
    }

    pub fn hash_to_target(difficulty: &u32) -> [u8; 32] {
        // Target: 2^(256 - difficulty)
        let mut target = [0xFF; 32];
        let leading_zeros = *difficulty / 8; // Number of leading zero bytes
        let partial_bits = *difficulty % 8; // Number of partial bits

        for item in target.iter_mut().take(leading_zeros as usize) {
            *item = 0;
        }

        if leading_zeros < 32 && partial_bits > 0 {
            // E.g., if partial_bits is 4, we need the first 4 bits to be 0,
            // so we set the byte to 0b00001111 = 15
            target[leading_zeros as usize] = (0xFF >> partial_bits) as u8;
        }

        target
    }
}

// ============================================================================
// State Management (UTXO Cache)
// ============================================================================

/// The global state of all unspent triangles (UTXOs) and derived address balances.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TriangleState {
    /// The actual UTXO set: Maps Triangle Hash -> Triangle
    pub utxo_set: HashMap<Sha256Hash, Triangle>,
    /// Derived balances: Maps Address -> Total Area (Coord)
    pub address_balances: HashMap<Address, Coord>,
}

impl TriangleState {
    /// Creates a new, empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the address_balances HashMap by iterating through the current utxo_set.
    /// This should be called after loading the utxo_set from persistence.
    pub fn rebuild_address_balances(&mut self) {
        self.address_balances.clear();
        for triangle in self.utxo_set.values() {
            *self
                .address_balances
                .entry(triangle.owner)
                .or_insert(Coord::from_num(0)) += triangle.effective_value();
        }
    }

    /// Gets the current total area owned by an address.
    pub fn get_balance(&self, address: &Address) -> Coord {
        *self
            .address_balances
            .get(address)
            .unwrap_or(&Coord::from_num(0))
    }

    /// Updates the UTXO set and derived balances based on a transaction.
    /// This is the core state transition logic for the blockchain. It is critical
    /// that this function is correct and deterministic.
    pub fn apply_transaction(
        &mut self,
        tx: &Transaction,
        _block_height: u64,
    ) -> Result<(), ChainError> {
        match tx {
            // ================== 1. Coinbase Transaction ==================
            // Creates new value (area) and assigns it to the miner's address (beneficiary).
            Transaction::Coinbase(tx) => {
                // a) Create the new Genesis Triangle for the block reward.
                // This triangle has a specific area but its geometric points are zero,
                // as it doesn't represent a real geometric shape but rather a value.
                let new_triangle = Triangle::new(
                    Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
                    Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
                    Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
                    None,
                    tx.beneficiary_address,
                )
                .with_effective_value(tx.reward_area);

                // b) Add the new triangle to the UTXO set, indexed by the transaction hash.
                let tx_hash = Transaction::Coinbase(tx.clone()).hash();
                self.utxo_set.insert(tx_hash, new_triangle);

                // c) Update the balance for the beneficiary address.
                *self
                    .address_balances
                    .entry(tx.beneficiary_address)
                    .or_insert(Coord::from_num(0)) += tx.reward_area;
            }

            // ================== 2. Transfer Transaction ==================
            // Consumes one UTXO and creates one or two new UTXOs (one for the recipient,
            // and optionally one for the sender's change).
            Transaction::Transfer(tx) => {
                // a) Find and remove the input UTXO being spent.
                let input_hash = tx.input_hash;
                let consumed_triangle = self.utxo_set.remove(&input_hash).ok_or_else(|| {
                    ChainError::TriangleNotFound(format!(
                        "Input UTXO not found for transfer: {}",
                        hex::encode(input_hash)
                    ))
                })?;

                // b) Verify that the sender owns the input UTXO.
                // This is a critical check to prevent theft.
                if consumed_triangle.owner != tx.sender {
                    // If ownership is invalid, revert the state change (put the UTXO back) and error out.
                    self.utxo_set.insert(input_hash, consumed_triangle.clone());
                    return Err(ChainError::InvalidTransaction(format!(
                        "Sender {} does not own input UTXO (owned by {})",
                        hex::encode(tx.sender), hex::encode(consumed_triangle.owner)
                    )));
                }

                let input_value = consumed_triangle.effective_value();
                let total_spent = tx.amount + tx.fee_area;
                let remaining_value = input_value - total_spent;

                // c) Decrease the sender's balance by the full value of the consumed UTXO.
                // The change amount will be added back later if applicable.
                let sender_balance = self
                    .address_balances
                    .entry(tx.sender)
                    .or_insert(Coord::from_num(0));
                *sender_balance -= input_value;
                if *sender_balance < Coord::from_num(0) {
                    *sender_balance = Coord::from_num(0);
                }

                // d) Create the new UTXO for the recipient.
                let new_owner_triangle = consumed_triangle
                    .clone()
                    .change_owner(tx.new_owner)
                    .with_effective_value(tx.amount); // The value is the amount being transferred.

                let tx_hash = Transaction::Transfer(tx.clone()).hash();
                self.utxo_set.insert(tx_hash, new_owner_triangle);

                // e) Update the recipient's balance.
                *self
                    .address_balances
                    .entry(tx.new_owner)
                    .or_insert(Coord::from_num(0)) += tx.amount;

                // f) Handle the change. If there's remaining value, create a new UTXO for the sender.
                if remaining_value > GEOMETRIC_TOLERANCE {
                    // Create a pseudo-transaction for the change to get a unique hash.
                    let change_tx = Transaction::Transfer(TransferTx {
                        input_hash: tx_hash, // The "input" for the change is the hash of the main transfer output.
                        new_owner: tx.sender,
                        sender: tx.sender,
                        amount: remaining_value,
                        fee_area: Coord::from_num(0), // No fee on change.
                        nonce: tx.nonce + 1,          // Different nonce to ensure different hash.
                        signature: None,
                        public_key: None,
                        memo: Some("Change".to_string()),
                    });

                    let change_hash = change_tx.hash();
                    let change_triangle = consumed_triangle
                        .change_owner(tx.sender)
                        .with_effective_value(remaining_value);

                    self.utxo_set.insert(change_hash, change_triangle);

                    // Add the change value back to the sender's balance.
                    *self
                        .address_balances
                        .entry(tx.sender)
                        .or_insert(Coord::from_num(0)) += remaining_value;
                }
            }

            // ================== 3. Subdivision Transaction ==================
            // Consumes one parent UTXO and creates multiple new children UTXOs from it.
            Transaction::Subdivision(tx) => {
                // a) Find and remove the parent UTXO being subdivided.
                let input_hash = tx.parent_hash;
                let consumed_triangle = self.utxo_set.remove(&input_hash).ok_or_else(|| {
                    ChainError::TriangleNotFound(format!(
                        "Parent UTXO for subdivision not found: {}",
                        hex::encode(input_hash)
                    ))
                })?;

                // b) Verify ownership of the parent triangle.
                if consumed_triangle.owner != tx.owner_address {
                    self.utxo_set.insert(input_hash, consumed_triangle.clone()); // Revert state change.
                    return Err(ChainError::InvalidTransaction(format!(
                        "Subdivision owner {} does not match parent triangle owner {}",
                        hex::encode(tx.owner_address), hex::encode(consumed_triangle.owner)
                    )));
                }

                // c) Decrease the owner's balance by the value of the consumed parent.
                let parent_value = consumed_triangle.effective_value();
                let owner_balance = self
                    .address_balances
                    .entry(tx.owner_address)
                    .or_insert(Coord::from_num(0));
                *owner_balance -= parent_value;
                if *owner_balance < Coord::from_num(0) {
                    *owner_balance = Coord::from_num(0);
                }

                // d) Validate that the children's total value equals the parent's value minus the fee.
                let total_child_value: Coord =
                    tx.children.iter().map(|c| c.effective_value()).sum();
                let expected_value = parent_value - tx.fee_area;

                if (total_child_value - expected_value).abs() > GEOMETRIC_TOLERANCE {
                    // Revert state changes before returning error
                    self.utxo_set.insert(input_hash, consumed_triangle);
                    *self
                        .address_balances
                        .entry(tx.owner_address)
                        .or_insert(Coord::from_num(0)) += parent_value;
                    return Err(ChainError::InvalidTransaction(format!(
                        "Value mismatch in subdivision: parent ({}) - fee ({}) != children total ({}).",
                        parent_value, tx.fee_area, total_child_value
                    )));
                }

                // e) Add new child UTXOs to the state and update balance.
                for child in &tx.children {
                    self.utxo_set.insert(child.hash(), child.clone());
                    *self
                        .address_balances
                        .entry(tx.owner_address)
                        .or_insert(Coord::from_num(0)) += child.effective_value();
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// Blockchain
// ============================================================================

/// The main structure holding the chain, UTXO state, and pending transactions.
#[derive(Debug)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub difficulty: u32,
    pub mempool: Mempool,
    pub state: TriangleState, // UTXO Cache (TriangleState)
}

impl Clone for Blockchain {
    fn clone(&self) -> Self {
        Self {
            blocks: self.blocks.clone(),
            difficulty: self.difficulty,
            mempool: self.mempool.clone(),
            state: self.state.clone(),
        }
    }
}

impl Blockchain {
    /// Initializer for a new blockchain with the genesis block.
    pub fn new(
        genesis_miner_address: Address,
        initial_difficulty: u32,
    ) -> Result<Self, ChainError> {
        let genesis_block = Self::create_genesis_block(genesis_miner_address, initial_difficulty)?;

        let mut blockchain = Blockchain {
            blocks: vec![],
            difficulty: initial_difficulty,
            mempool: Mempool::new(),
            state: TriangleState::new(),
        };

        // Apply the genesis block to initialize the state
        blockchain.apply_block(genesis_block)?;
        Ok(blockchain)
    }

    /// Creates the immutable genesis block.
    fn create_genesis_block(
        miner_address: Address,
        initial_difficulty: u32,
    ) -> Result<Block, ChainError> {
        // Create a special genesis transaction (Coinbase)
        let coinbase_tx = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(1_000_000.0), // Initial fixed supply
            beneficiary_address: miner_address,
            nonce: 0,
        });

        let transactions = vec![coinbase_tx];
        let merkle_root = Block::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            height: 0,
            timestamp: 1672531200000, // Jan 1, 2023
            previous_hash: [0u8; 32],
            merkle_root,
            difficulty: initial_difficulty,
            nonce: 0,
        };

        let genesis_block = Block {
            header,
            transactions,
        };

        mine_block(genesis_block)
    }

    /// Calculates the block reward based on height (halving model)
    pub fn calculate_block_reward(height: u64) -> f64 {
        const INITIAL_REWARD: f64 = 50.0; // In area units
        const HALVING_INTERVAL: u64 = 210000;

        let halving_count = height / HALVING_INTERVAL;
        if halving_count >= 64 {
            // Prevent overflow and reward going to 0
            0.0
        } else {
            INITIAL_REWARD / (2u64.pow(halving_count as u32) as f64)
        }
    }

    // ============================================================================
    // Core Chain and State Logic
    // ============================================================================

    /// Validates that no UTXOs are spent more than once within the same block.
    fn validate_no_double_spend(block: &Block) -> Result<(), ChainError> {
        let mut seen_inputs = HashMap::new();
        for tx in &block.transactions {
            let input_hash = match tx {
                Transaction::Transfer(t) => Some(t.input_hash),
                Transaction::Subdivision(s) => Some(s.parent_hash),
                _ => None, // Coinbase has no input
            };

            if let Some(hash) = input_hash {
                if let Some(conflicting_tx_hash) = seen_inputs.get(&hash) {
                    return Err(ChainError::InvalidTransaction(format!(
                        "Double spend detected in block. UTXO {} is spent by both {} and {}",
                        hex::encode(hash),
                        hex::encode(conflicting_tx_hash),
                        hex::encode(tx.hash())
                    )));
                }
                seen_inputs.insert(hash, tx.hash());
            }
        }
        Ok(())
    }

    /// Attempts to apply a block to the chain, performing all necessary validations.
    /// This is the heart of the consensus logic, ensuring that only valid blocks
    /// are added to the chain.
    pub fn apply_block(&mut self, block: Block) -> Result<(), ChainError> {
        let is_genesis = block.header.height == 0;

        // 1. ==================== Basic Header Validation ====================
        if !is_genesis {
            let last_block = self.blocks.last().ok_or_else(|| {
                ChainError::InvalidBlock(
                    "Cannot apply non-genesis block; the chain is empty.".to_string(),
                )
            })?;

            // a) Check for sequential height
            if block.header.height != last_block.header.height + 1 {
                return Err(ChainError::InvalidBlock(format!(
                    "Invalid block height. Expected {}, but got {}.",
                    last_block.header.height + 1,
                    block.header.height
                )));
            }

            // b) Check for correct previous hash link
            if block.header.previous_hash != last_block.hash() {
                return Err(ChainError::InvalidBlock(format!(
                    "Invalid previous block hash. Expected {}, but got {}.",
                    hex::encode(last_block.hash()),
                    hex::encode(block.header.previous_hash)
                )));
            }
        } else if !self.blocks.is_empty() {
            // Genesis block can only be applied to an empty chain
            return Err(ChainError::InvalidBlock(
                "Genesis block can only be applied to an empty chain.".to_string(),
            ));
        }

        // 2. ==================== Proof-of-Work (PoW) Validation ====================
        if !self.verify_pow(&block) {
            return Err(ChainError::InvalidBlock(
                "Invalid Proof-of-Work: Block hash does not meet difficulty target.".to_string(),
            ));
        }

        // 3. ================= Transaction and State Validation =================
        // Create a temporary state to simulate the application of transactions.
        // If any transaction fails, we discard this state, leaving the main state untouched.
        let mut temp_state = self.state.clone();

        // a) Check for double spending within the block itself
        Self::validate_no_double_spend(&block)?;

        // b) Validate and apply each transaction sequentially
        for (i, tx) in block.transactions.iter().enumerate() {
            // All transactions must adhere to size limits.
            tx.validate_size()?;

            // The first transaction MUST be a Coinbase transaction.
            if i == 0 {
                if !matches!(tx, Transaction::Coinbase(_)) {
                    return Err(ChainError::InvalidBlock(
                        "First transaction in a block must be a Coinbase transaction.".to_string(),
                    ));
                }
            } else {
                // All other transactions must be standard and pass signature/state checks.
                tx.validate(&temp_state)?;
            }

            // Apply the transaction to the temporary state, updating the UTXO set.
            temp_state.apply_transaction(tx, block.header.height)?;
        }

        // 4. ==================== Final Block Validation ====================
        // a) Verify the Merkle root matches the transactions in the block.
        let expected_merkle_root = Block::calculate_merkle_root(&block.transactions);
        if expected_merkle_root != block.header.merkle_root {
            return Err(ChainError::InvalidBlock(format!(
                "Merkle root mismatch. Expected {}, but got {}.",
                hex::encode(expected_merkle_root),
                hex::encode(block.header.merkle_root)
            )));
        }

        // 5. ==================== Commit to Chain State ====================
        // All checks passed. The block is valid.
        // a) Add the block to the blockchain.
        self.blocks.push(block.clone());
        // b) Commit the temporary state as the new official state.
        self.state = temp_state;

        // c) Remove the newly confirmed transactions from the mempool.
        for tx in &block.transactions {
            self.mempool.remove_transaction(&tx.hash());
        }

        // d) Adjust difficulty.
        self.adjust_difficulty();

        Ok(())
    }

    /// Adjusts the blockchain difficulty based on the time it took to mine the last
    /// `DIFFICULTY_ADJUSTMENT_INTERVAL` blocks.
    fn adjust_difficulty(&mut self) {
        let current_height = self.blocks.last().map_or(0, |b| b.header.height);
        if current_height > 0 && current_height.is_multiple_of(DIFFICULTY_ADJUSTMENT_INTERVAL) {
            let last_adjustment_block = self
                .blocks
                .get((current_height - DIFFICULTY_ADJUSTMENT_INTERVAL) as usize);

            if let Some(last_adjustment_block) = last_adjustment_block {
                let last_block = self.blocks.last().unwrap();
                let actual_time =
                    last_block.header.timestamp - last_adjustment_block.header.timestamp;
                let expected_time = (DIFFICULTY_ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME) * 1000; // in milliseconds

                let ratio = actual_time as f64 / expected_time as f64;

                // Clamp the ratio to prevent drastic changes
                let ratio = ratio.clamp(0.25, 4.0);

                let new_difficulty = (self.difficulty as f64 * ratio) as u32;
                // Ensure difficulty is at least 1
                self.difficulty = new_difficulty.max(1);
            }
        }
    }

    /// Verifies the Proof-of-Work constraint of a block.
    fn verify_pow(&self, block: &Block) -> bool {
        let hash_target = Block::hash_to_target(&block.header.difficulty);
        let block_hash_int = Block::hash_as_u256(&block.hash());

        // Block hash must be less than or equal to the target
        block_hash_int <= hash_target
    }
}

// ============================================================================
// Blockchain
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Coord, Point};
    use crate::transaction::{SubdivisionTx, TransferTx};
    fn create_test_address(id: &str) -> Address {
        let mut address = [0u8; 32];
        let bytes = id.as_bytes();
        address[..bytes.len()].copy_from_slice(bytes);
        address
    }

    fn create_test_blockchain() -> Blockchain {
        Blockchain::new(create_test_address("miner1"), 1).unwrap()
    }

    fn create_test_transaction(i: u8) -> Transaction {
        Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(i),
            beneficiary_address: create_test_address("test"),
        })
    }

    #[test]
    fn test_new_blockchain_creates_genesis_block() {
        let blockchain = create_test_blockchain();
        assert_eq!(blockchain.blocks.len(), 1);
        let genesis_block = &blockchain.blocks[0];
        assert_eq!(genesis_block.header.height, 0);
        assert_eq!(genesis_block.header.previous_hash, [0u8; 32]);
        assert_eq!(
            blockchain.state.get_balance(&create_test_address("miner1")),
            Coord::from_num(1_000_000.0)
        );
    }

    #[test]
    fn test_block_header_hash() {
        let header = BlockHeader {
            height: 1,
            timestamp: 12345,
            previous_hash: [1; 32],
            merkle_root: [2; 32],
            difficulty: 10,
            nonce: 42,
        };
        let hash = header.hash();
        assert_ne!(hash, [0; 32]);
    }

    #[test]
    fn test_calculate_merkle_root() {
        let tx1 = create_test_transaction(1);
        let tx2 = create_test_transaction(2);

        // Test with no transactions
        let root_empty = Block::calculate_merkle_root(&[]);
        assert_ne!(root_empty, [0u8; 32]); // Should be hash of empty data, not zeros

        // Test with one transaction
        let root_one = Block::calculate_merkle_root(&[tx1.clone()]);
        let mut hasher = Sha256::new();
        hasher.update(tx1.hash());
        let expected_one: Sha256Hash = hasher.finalize().into();
        assert_eq!(root_one, expected_one);

        // Test with multiple transactions
        let root_multiple = Block::calculate_merkle_root(&[tx1.clone(), tx2.clone()]);
        let mut hasher_multiple = Sha256::new();
        hasher_multiple.update(tx1.hash());
        hasher_multiple.update(tx2.hash());
        let expected_multiple: Sha256Hash = hasher_multiple.finalize().into();
        assert_eq!(root_multiple, expected_multiple);
    }

    #[test]
    fn test_hash_to_target() {
        let target1 = Block::hash_to_target(&8);
        assert_eq!(target1[0], 0);
        assert_eq!(target1[1], 0xFF);

        let target2 = Block::hash_to_target(&16);
        assert_eq!(target2[0], 0);
        assert_eq!(target2[1], 0);
        assert_eq!(target2[2], 0xFF);

        let target3 = Block::hash_to_target(&10);
        assert_eq!(target3[0], 0);
        assert_eq!(target3[1], 0b0011_1111);
    }

    #[test]
    fn test_calculate_block_reward() {
        assert_eq!(Blockchain::calculate_block_reward(0), 50.0);
        assert_eq!(Blockchain::calculate_block_reward(210000 - 1), 50.0);
        assert_eq!(Blockchain::calculate_block_reward(210000), 25.0);
        assert_eq!(Blockchain::calculate_block_reward(420000), 12.5);
        // Test far in the future
        assert_eq!(Blockchain::calculate_block_reward(210000 * 64), 0.0);
    }

    #[test]
    fn test_apply_block_valid() {
        let mut blockchain = create_test_blockchain();
        let last_block = blockchain.blocks.last().unwrap().clone();

        let tx = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(50.0),
            beneficiary_address: create_test_address("miner2"),
        });

        let block = Block::new(1, last_block.hash(), 1, vec![tx]);
        let mined_block = mine_block(block).unwrap();

        let result = blockchain.apply_block(mined_block);
        assert!(result.is_ok());
        assert_eq!(blockchain.blocks.len(), 2);
        assert_eq!(
            blockchain.state.get_balance(&create_test_address("miner2")),
            Coord::from_num(50.0)
        );
    }

    #[test]
    fn test_apply_block_invalid_height() {
        let mut blockchain = create_test_blockchain();
        let last_block = blockchain.blocks.last().unwrap().clone();

        let block = Block::new(2, last_block.hash(), 1, vec![]); // Invalid height

        let result = blockchain.apply_block(block);
        assert!(matches!(result, Err(ChainError::InvalidBlock(_))));
    }

    #[test]
    fn test_apply_block_invalid_prev_hash() {
        let mut blockchain = create_test_blockchain();

        let block = Block::new(1, [0; 32], 1, vec![]); // Invalid prev hash

        let result = blockchain.apply_block(block);
        assert!(matches!(result, Err(ChainError::InvalidBlock(_))));
    }

    #[test]
    fn test_apply_block_invalid_pow() {
        let mut blockchain = create_test_blockchain();
        let last_block = blockchain.blocks.last().unwrap().clone();

        // Block without mining
        let block = Block::new(1, last_block.hash(), 20, vec![]); // High difficulty

        let result = blockchain.apply_block(block);
        assert!(
            matches!(result, Err(ChainError::InvalidBlock(msg)) if msg.contains("Invalid Proof-of-Work"))
        );
    }

    #[test]
    fn test_apply_block_double_spend_in_block() {
        let mut blockchain = create_test_blockchain();

        // Create a UTXO to be spent
        let initial_tx = blockchain.blocks[0].transactions[0].clone();
        let input_hash = initial_tx.hash();

        let transfer_tx = TransferTx {
            input_hash,
            new_owner: create_test_address("recipient"),
            sender: create_test_address("miner1"),
            amount: Coord::from_num(100.0),
            fee_area: Coord::from_num(1.0),
            nonce: 0,
            signature: None, // Simplified for test
            public_key: None,
            memo: None,
        };

        let tx1 = Transaction::Transfer(transfer_tx.clone());
        let tx2 = Transaction::Transfer(transfer_tx); // Same input hash

        let last_block = blockchain.blocks.last().unwrap().clone();
        let coinbase = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(50.0),
            beneficiary_address: create_test_address("miner1"),
        });

        let block = Block::new(1, last_block.hash(), 1, vec![coinbase, tx1, tx2]);
        let mined_block = mine_block(block).unwrap();

        let result = blockchain.apply_block(mined_block);
        assert!(
            matches!(result, Err(ChainError::InvalidTransaction(msg)) if msg.contains("Double spend detected"))
        );
    }

    #[test]
    fn test_state_apply_coinbase_tx() {
        let mut state = TriangleState::new();
        let address = create_test_address("miner");
        let tx = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(100.0),
            beneficiary_address: address.clone(),
        });

        let result = state.apply_transaction(&tx, 0);
        assert!(result.is_ok());
        assert_eq!(state.get_balance(&address), Coord::from_num(100.0));
        assert_eq!(state.utxo_set.len(), 1);
    }

    #[test]
    fn test_state_apply_transfer_tx() {
        let mut state = TriangleState::new();
        let sender = create_test_address("sender");
        let recipient = create_test_address("recipient");

        // 1. Create a UTXO for the sender
        let initial_triangle = Triangle::new(
            Point::new(Coord::from_num(0), Coord::from_num(0)),
            Point::new(Coord::from_num(10), Coord::from_num(0)),
            Point::new(Coord::from_num(5), Coord::from_num(10)),
            None,
            sender.clone(),
        )
        .with_effective_value(Coord::from_num(1000.0));
        let input_hash = initial_triangle.hash();
        state.utxo_set.insert(input_hash, initial_triangle);
        state
            .address_balances
            .insert(sender.clone(), Coord::from_num(1000.0));

        // 2. Create and apply the transfer
        let tx = Transaction::Transfer(TransferTx {
            input_hash,
            new_owner: recipient.clone(),
            sender: sender.clone(),
            amount: Coord::from_num(700.0),
            fee_area: Coord::from_num(50.0),
            nonce: 0,
            signature: None,
            public_key: None,
            memo: None,
        });

        let result = state.apply_transaction(&tx, 1);
        assert!(result.is_ok());

        // 3. Verify state changes
        let change_value = Coord::from_num(250.0);
        assert_eq!(state.get_balance(&sender), change_value);
        assert_eq!(state.get_balance(&recipient), Coord::from_num(700.0));

        // Should be 2 UTXOs: one for recipient, one for change
        assert_eq!(state.utxo_set.len(), 2);

        let recipient_utxo_found = state
            .utxo_set
            .values()
            .any(|t| t.owner == recipient && t.effective_value() == Coord::from_num(700.0));
        let change_utxo_found = state
            .utxo_set
            .values()
            .any(|t| t.owner == sender && t.effective_value() == change_value);

        assert!(
            recipient_utxo_found,
            "Recipient's UTXO not found or incorrect value"
        );
        assert!(
            change_utxo_found,
            "Sender's change UTXO not found or incorrect value"
        );
    }

    #[test]
    fn test_state_apply_subdivision_tx() {
        let mut state = TriangleState::new();
        let owner = create_test_address("owner");

        // 1. Create a parent UTXO with an effective value of 100.
        let parent_triangle = Triangle::new(
            Point::new(Coord::from_num(0), Coord::from_num(0)),
            Point::new(Coord::from_num(20), Coord::from_num(0)),
            Point::new(Coord::from_num(10), Coord::from_num(10)),
            None,
            owner.clone(),
        ); // Area = 100
        let parent_hash = parent_triangle.hash();
        let parent_value = parent_triangle.effective_value();
        state.utxo_set.insert(parent_hash, parent_triangle.clone());
        state.address_balances.insert(owner.clone(), parent_value);

        // 2. Define fee and calculate the total value for the children.
        let fee = Coord::from_num(10.0);
        let total_child_value = parent_value - fee; // Should be 90.0

        // 3. Create child triangles whose values sum to the required total.
        // This simulates a client correctly constructing a subdivision transaction.
        let base_children = parent_triangle.subdivide();
        let child1_value = total_child_value / 3;
        let child2_value = total_child_value / 3;
        let child3_value = total_child_value - child1_value - child2_value; // Handle rounding

        let children = vec![
            base_children[0].clone().with_effective_value(child1_value),
            base_children[1].clone().with_effective_value(child2_value),
            base_children[2].clone().with_effective_value(child3_value),
        ];

        // 4. Create and apply the subdivision transaction.
        let subdivision_tx = SubdivisionTx {
            parent_hash,
            owner_address: owner.clone(),
            children,
            fee_area: fee,
            nonce: 0,
            signature: None,
            public_key: None,
        };
        let tx = Transaction::Subdivision(subdivision_tx);

        let result = state.apply_transaction(&tx, 1);
        assert!(
            result.is_ok(),
            "Transaction should be valid, but failed with: {:?}",
            result.err()
        );

        // 5. Verify state changes.
        let expected_balance = total_child_value;
        assert!(
            (state.get_balance(&owner) - expected_balance).abs() < GEOMETRIC_TOLERANCE,
            "Balance mismatch: expected {}, got {}",
            expected_balance,
            state.get_balance(&owner)
        );
        assert_eq!(state.utxo_set.len(), 3, "There should be 3 new UTXOs");

        // Verify that the child UTXOs are in the set with the correct values.
        if let Transaction::Subdivision(sub_tx) = tx {
            for child in &sub_tx.children {
                assert!(
                    state.utxo_set.contains_key(&child.hash()),
                    "Child UTXO not found in state"
                );
                let utxo = state.utxo_set.get(&child.hash()).unwrap();
                assert_eq!(
                    utxo.effective_value(),
                    child.effective_value(),
                    "Child UTXO value is incorrect in state"
                );
            }
        }
    }
}
