//! Core blockchain implementation for TrinityChain

use crate::error::ChainError;
use crate::geometry::{Coord, Point, Triangle};
use crate::mempool::Mempool;
use crate::transaction::{CoinbaseTx, SubdivisionTx, Transaction};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub type Sha256Hash = [u8; 32];
pub type BlockHeight = u64;

/// The genesis triangle - the root of all triangles
pub fn genesis_triangle() -> Triangle {
    Triangle::new(
        Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
        Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
        Point::new(Coord::from_num(0.5), Coord::from_num(0.8660254)),
        None,
        "genesis_owner".to_string(),
    )
}

/// Manages the canonical set of all currently valid (unspent) triangles (UTXO set).
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriangleState {
    pub utxo_set: HashMap<Sha256Hash, Triangle>,
    /// Address index: maps owner address to list of triangle hashes they own
    /// This makes balance queries O(1) instead of O(n)
    #[serde(skip)]
    pub address_index: HashMap<String, Vec<Sha256Hash>>,
}

impl TriangleState {
    pub fn new() -> Self {
        TriangleState {
            utxo_set: HashMap::new(),
            address_index: HashMap::new(),
        }
    }

    /// Rebuild the address index from the UTXO set
    /// Should be called after loading from database
    pub fn rebuild_address_index(&mut self) {
        self.address_index.clear();
        for (hash, triangle) in &self.utxo_set {
            self.address_index
                .entry(triangle.owner.clone())
                .or_default()
                .push(*hash);
        }
    }

    /// Get all triangles owned by an address (O(1) lookup)
    pub fn get_triangles_by_owner(&self, owner: &str) -> Vec<&Triangle> {
        self.address_index
            .get(owner)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|hash| self.utxo_set.get(hash))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Calculate total area owned by an address (O(1) lookup + O(k) sum where k = # triangles owned)
    pub fn get_balance(&self, owner: &str) -> Coord {
        self.get_triangles_by_owner(owner)
            .iter()
            .map(|t| t.effective_value())
            .sum()
    }

    pub fn count(&self) -> usize {
        self.utxo_set.len()
    }

    /// Apply a subdivision transaction to the state
    /// Optimized to minimize hash calculations and clones
    pub fn apply_subdivision(&mut self, tx: &SubdivisionTx) -> Result<(), ChainError> {
        // Remove parent from UTXO set and address index
        let parent = self.utxo_set.remove(&tx.parent_hash).ok_or_else(|| {
            ChainError::TriangleNotFound(format!(
                "Parent triangle {} not found",
                hex::encode(tx.parent_hash)
            ))
        })?;

        // Update address index: remove parent hash
        if let Some(hashes) = self.address_index.get_mut(&parent.owner) {
            hashes.retain(|h| h != &tx.parent_hash);
            if hashes.is_empty() {
                self.address_index.remove(&parent.owner);
            }
        }

        // Add children to UTXO set and address index
        for child in &tx.children {
            let child_hash = child.hash();
            self.utxo_set.insert(child_hash, child.clone());

            // Update address index: add child hash
            self.address_index
                .entry(child.owner.clone())
                .or_default()
                .push(child_hash);
        }

        Ok(())
    }

    /// Apply a coinbase transaction to the state, creating a new triangle as a reward.
    pub fn apply_coinbase(
        &mut self,
        tx: &CoinbaseTx,
        block_height: BlockHeight,
    ) -> Result<(), ChainError> {
        // Create a new triangle with a canonical shape based on the reward area
        // The position is offset by the block height to ensure uniqueness
        let side = tx.reward_area.sqrt();
        if side <= Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Invalid reward area for coinbase transaction".to_string(),
            ));
        }

        // We'll create a right isosceles triangle at a location based on block height
        // This ensures that reward triangles don't collide with each other
        let offset = Coord::from_num(block_height * 1000); // Use a large offset
        let new_triangle = Triangle::new(
            Point::new(offset, Coord::from_num(0)),
            Point::new(offset + side, Coord::from_num(0)),
            Point::new(offset, side),
            None,
            tx.beneficiary_address.clone(),
        );

        let hash = new_triangle.hash();
        self.utxo_set.insert(hash, new_triangle.clone());

        // Update address index
        self.address_index
            .entry(tx.beneficiary_address.clone())
            .or_default()
            .push(hash);

        Ok(())
    }
}

/// Represents a block header with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub previous_hash: Sha256Hash,
    pub timestamp: i64,
    pub difficulty: u64,
    pub nonce: u64,
    pub merkle_root: Sha256Hash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
}

impl BlockHeader {
    #[inline]
    pub fn calculate_hash(&self) -> Sha256Hash {
        let mut hasher = Sha256::new();
        // Use as_slice() and direct byte operations for better performance
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.previous_hash);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.difficulty.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.update(self.merkle_root);
        hasher.finalize().into()
    }
}

/// A block in the blockchain
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub hash: Sha256Hash,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        height: BlockHeight,
        previous_hash: Sha256Hash,
        difficulty: u64,
        transactions: Vec<Transaction>,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let merkle_root = Self::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            height,
            previous_hash,
            timestamp,
            difficulty,
            nonce: 0,
            merkle_root,
            headline: None, // Only genesis block has a headline
        };

        Block {
            header,
            hash: [0; 32], // Will be calculated by the miner
            transactions,
        }
    }

    /// Create a new block ensuring timestamp is greater than parent timestamp
    pub fn new_with_parent_time(
        height: BlockHeight,
        previous_hash: Sha256Hash,
        parent_timestamp: i64,
        difficulty: u64,
        transactions: Vec<Transaction>,
    ) -> Self {
        let mut timestamp = Utc::now().timestamp();

        // Ensure timestamp is strictly greater than parent
        if timestamp <= parent_timestamp {
            timestamp = parent_timestamp + 1;
        }

        let merkle_root = Self::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            height,
            previous_hash,
            timestamp,
            difficulty,
            nonce: 0,
            merkle_root,
            headline: None,
        };

        Block {
            header,
            hash: [0; 32],
            transactions,
        }
    }

    #[inline]
    pub fn calculate_hash(&self) -> Sha256Hash {
        // Delegate to header's hash calculation for consistency
        self.header.calculate_hash()
    }

    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Sha256Hash {
        if transactions.is_empty() {
            return [0; 32];
        }

        // Pre-allocate with exact capacity to avoid reallocations
        let mut hashes: Vec<[u8; 32]> = Vec::with_capacity(transactions.len());
        for tx in transactions {
            hashes.push(tx.hash());
        }

        while hashes.len() > 1 {
            if !hashes.len().is_multiple_of(2) {
                // Duplicate last hash for odd-length trees
                hashes.push(hashes[hashes.len() - 1]);
            }

            // Reuse the same vec for parent hashes to reduce allocations
            let mut new_hashes = Vec::with_capacity(hashes.len().div_ceil(2));
            for i in (0..hashes.len()).step_by(2) {
                let mut hasher = Sha256::new();
                hasher.update(hashes[i]);
                hasher.update(hashes[i + 1]);
                new_hashes.push(hasher.finalize().into());
            }
            hashes = new_hashes;
        }

        hashes[0]
    }

    #[inline]
    pub fn verify_proof_of_work(&self) -> bool {
        // Use the optimized is_hash_valid from miner module
        crate::miner::is_hash_valid(&self.hash, self.header.difficulty)
    }
}

/// The blockchain itself
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub block_index: HashMap<Sha256Hash, Block>,
    pub forks: HashMap<Sha256Hash, Block>,
    pub state: TriangleState,
    pub difficulty: u64,
    pub mempool: Mempool,
}

// Bitcoin-like parameters for Sierpinski Triangle Blockchain
// Target: 1 block every 60 seconds = 1,440 blocks/day = ~525,600 blocks/year

/// Difficulty adjusts every 2,016 blocks (like Bitcoin) ~1.4 days at 1 minute blocks
const DIFFICULTY_ADJUSTMENT_WINDOW: BlockHeight = 2016;

/// Target block time: 60 seconds (1 minute)
const TARGET_BLOCK_TIME_SECONDS: i64 = 60;

/// Initial mining reward (in area units) - represents triangle area
const INITIAL_MINING_REWARD: u64 = 1000;

/// Halving interval - reward halves every 210,000 blocks (~4 years at 1 minute blocks)
/// This matches Bitcoin's ~4 year halving cycle
pub const REWARD_HALVING_INTERVAL: BlockHeight = 210_000;

/// Maximum number of halvings before reward becomes 0 (64 halvings)
const MAX_HALVINGS: u64 = 64;

/// Maximum depth for a blockchain reorganization to prevent instability (e.g. 100 blocks)
const MAX_REORG_DEPTH: BlockHeight = 100;

/// Calculate maximum supply: sum of geometric series
/// Max supply = INITIAL_REWARD * HALVING_INTERVAL * (1 + 1/2 + 1/4 + ... ≈ 2)
/// = 1000 * 210,000 * 2 = 420,000,000 area units
pub const MAX_SUPPLY: u64 = INITIAL_MINING_REWARD * REWARD_HALVING_INTERVAL * 2;

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl Blockchain {
    /// Calculate the block reward for a given block height (with halving)
    pub fn calculate_block_reward(height: BlockHeight) -> u64 {
        let halvings = height / REWARD_HALVING_INTERVAL;
        if halvings >= MAX_HALVINGS {
            // After 64 halvings, reward is 0
            return 0;
        }
        INITIAL_MINING_REWARD >> halvings
    }

    pub fn new() -> Self {
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash, genesis);

        // Use a fixed genesis timestamp (January 1, 2024, 00:00:00 UTC)
        // This ensures the genesis block is always the same across all instances
        let genesis_timestamp: i64 = 1704067200;

        let mut genesis_block = Block {
            header: BlockHeader {
                height: 0,
                previous_hash: [0; 32],
                timestamp: genesis_timestamp,
                difficulty: 2,
                nonce: 0,
                merkle_root: [0; 32],
                headline: Some(
                    "TrinityChain Genesis Block - Sierpinski Triangle Blockchain".to_string(),
                ),
            },
            hash: [0; 32], // Will be calculated based on header content
            transactions: vec![],
        };

        // Calculate the actual genesis block hash
        genesis_block.hash = genesis_block.calculate_hash();

        let mut block_index = HashMap::new();
        block_index.insert(genesis_block.hash, genesis_block.clone());

        Blockchain {
            blocks: vec![genesis_block],
            block_index,
            forks: HashMap::new(),
            state,
            difficulty: 2,
            mempool: Mempool::new(),
        }
    }

    pub fn validate_block(&self, block: &Block) -> Result<(), ChainError> {
        let parent_block = match self.block_index.get(&block.header.previous_hash) {
            Some(block) => block,
            None => return Err(ChainError::InvalidBlockLinkage),
        };

        if block.header.height != parent_block.header.height + 1 {
            return Err(ChainError::InvalidBlockLinkage);
        }

        // Validate timestamp is greater than parent's timestamp (skip for genesis block)
        if block.header.height > 0 && block.header.timestamp <= parent_block.header.timestamp {
            return Err(ChainError::InvalidTransaction(
                "Block timestamp must be greater than parent timestamp".to_string(),
            ));
        }

        // Validate timestamp is not too far in the future (allow 24 hours of clock drift)
        // This accounts for potential system clock issues and network delays
        const MAX_FUTURE_TIMESTAMP_DRIFT: i64 = 24 * 3600; // 24 hours in seconds
        let current_time = Utc::now().timestamp();
        if block.header.timestamp > current_time + MAX_FUTURE_TIMESTAMP_DRIFT {
            return Err(ChainError::InvalidTransaction(format!(
                "Block timestamp is too far in the future (block: {}, current: {}, max drift: {}s)",
                block.header.timestamp, current_time, MAX_FUTURE_TIMESTAMP_DRIFT
            )));
        }

        if !block.verify_proof_of_work() {
            return Err(ChainError::InvalidProofOfWork);
        }

        let calculated_merkle = Block::calculate_merkle_root(&block.transactions);
        if block.header.merkle_root != calculated_merkle {
            return Err(ChainError::InvalidMerkleRoot);
        }

        // Validate coinbase transaction rules
        let mut coinbase_count = 0;
        let mut coinbase_reward = Coord::from_num(0);
        for (i, tx) in block.transactions.iter().enumerate() {
            if let Transaction::Coinbase(coinbase_tx) = tx {
                coinbase_count += 1;
                coinbase_reward = coinbase_tx.reward_area;
                // Coinbase must be the first transaction
                if i != 0 {
                    return Err(ChainError::InvalidTransaction(
                        "Coinbase transaction must be the first transaction in the block"
                            .to_string(),
                    ));
                }
            }
        }

        // Exactly one coinbase transaction per block (or zero for genesis)
        if block.header.height > 0 && coinbase_count != 1 {
            return Err(ChainError::InvalidTransaction(format!(
                "Block must contain exactly one coinbase transaction, found {}",
                coinbase_count
            )));
        }

        // Validate coinbase reward doesn't exceed block reward + fees
        if block.header.height > 0 {
            let block_reward = Blockchain::calculate_block_reward(block.header.height);
            let total_fees = Self::calculate_total_fees(&block.transactions);

            // Max reward = static block reward + geometric fee area
            let max_reward = Coord::from_num(block_reward) + total_fees;

            // Use a tolerance for floating point comparison
            if coinbase_reward > max_reward + Coord::from_num(1e-9) {
                return Err(ChainError::InvalidTransaction(format!(
                    "Coinbase reward {} exceeds maximum allowed {} (block reward: {}, fees: {})",
                    coinbase_reward, max_reward, block_reward, total_fees
                )));
            }
        }

        for tx in block.transactions.iter() {
            match tx {
                Transaction::Subdivision(tx) => {
                    if !self.state.utxo_set.contains_key(&tx.parent_hash) {
                        return Err(ChainError::InvalidTransaction(format!(
                            "Parent triangle {} not in UTXO set",
                            hex::encode(tx.parent_hash)
                        )));
                    }
                    tx.validate(&self.state)?;
                }
                Transaction::Coinbase(cb_tx) => {
                    cb_tx.validate()?;
                }
                Transaction::Transfer(tx) => {
                    // Full validation including UTXO existence and fee_area check
                    tx.validate_with_state(&self.state)?;
                }
            }
        }

        Ok(())
    }

    pub fn apply_block(&mut self, valid_block: Block) -> Result<(), ChainError> {
        self.validate_block(&valid_block)?;

        let parent_hash = valid_block.header.previous_hash;
        let last_block_hash = match self.blocks.last() {
            Some(block) => block.hash,
            None => return Err(ChainError::InvalidBlockLinkage), // Should not happen if genesis exists
        };

        // Case 1: The new block extends the main chain
        if parent_hash == last_block_hash {
            // Collect transaction hashes before applying
            let tx_hashes: Vec<Sha256Hash> = valid_block
                .transactions
                .iter()
                .map(|tx| tx.hash())
                .collect();

            for tx in valid_block.transactions.iter() {
                match tx {
                    Transaction::Subdivision(sub_tx) => {
                        self.state.apply_subdivision(sub_tx)?;
                    }
                    Transaction::Coinbase(cb_tx) => {
                        self.state
                            .apply_coinbase(cb_tx, valid_block.header.height)?;
                    }
                    Transaction::Transfer(tx) => {
                        // New transfer logic with "change"
                        // 1. Remove input triangle from UTXO set.
                        // 2. Calculate change value.
                        // 3. Create a new triangle for the recipient with `tx.amount`.
                        // 4. Create a new "change" triangle for the sender with the remaining value.
                        // 5. Add both new triangles to the UTXO set and update address indexes.

                        let input_triangle =
                            self.state.utxo_set.remove(&tx.input_hash).ok_or_else(|| {
                                ChainError::TriangleNotFound(format!(
                                    "Transfer input {} missing from UTXO set",
                                    hex::encode(tx.input_hash)
                                ))
                            })?;

                        let input_value = input_triangle.effective_value();
                        let change_value = input_value - tx.amount - tx.fee_area;

                        // --- Create Recipient's Triangle ---
                        let recipient_triangle = crate::geometry::Triangle::new_with_value(
                            input_triangle.a,
                            input_triangle.b,
                            input_triangle.c,
                            Some(tx.input_hash),
                            tx.new_owner.clone(),
                            tx.amount,
                        );
                        let recipient_hash = recipient_triangle.hash();
                        self.state
                            .utxo_set
                            .insert(recipient_hash, recipient_triangle);
                        self.state
                            .address_index
                            .entry(tx.new_owner.clone())
                            .or_default()
                            .push(recipient_hash);

                        // --- Create Sender's Change Triangle ---
                        if change_value >= crate::geometry::GEOMETRIC_TOLERANCE {
                            let change_triangle = crate::geometry::Triangle::new_with_value(
                                input_triangle.a,
                                input_triangle.b,
                                input_triangle.c,
                                Some(tx.input_hash),
                                tx.sender.clone(),
                                change_value,
                            );
                            let change_hash = change_triangle.hash();
                            self.state.utxo_set.insert(change_hash, change_triangle);
                            self.state
                                .address_index
                                .entry(tx.sender.clone())
                                .or_default()
                                .push(change_hash);
                        }

                        // --- Update Sender's Address Index (Remove Old Hash) ---
                        if let Some(hashes) = self.state.address_index.get_mut(&tx.sender) {
                            hashes.retain(|h| h != &tx.input_hash);
                            if hashes.is_empty() {
                                self.state.address_index.remove(&tx.sender);
                            }
                        }
                    }
                }
            }

            let block_height = valid_block.header.height;
            self.blocks.push(valid_block.clone());
            self.block_index
                .insert(valid_block.hash, valid_block.clone());

            // Only adjust difficulty every DIFFICULTY_ADJUSTMENT_WINDOW blocks to prevent oscillation
            // Adjust after accumulating enough blocks (at multiples of the window)
            if block_height > 0 && block_height.is_multiple_of(DIFFICULTY_ADJUSTMENT_WINDOW) {
                self.adjust_difficulty();
            }

            self.mempool.remove_transactions(&tx_hashes);
            self.mempool.prune(&self.state);
        } else if self.block_index.contains_key(&parent_hash) {
            // Case 2: The new block creates a fork
            println!("🍴 Fork detected at height {}", valid_block.header.height);
            self.forks.insert(valid_block.hash, valid_block.clone());
            self.block_index
                .insert(valid_block.hash, valid_block.clone());

            // Check if the new fork is longer than the main chain.
            // Height is 0-indexed, length is 1-indexed.
            if valid_block.header.height + 1 > self.blocks.len() as u64 {
                println!("⚠️  Switching to a longer fork! Rebuilding state...");

                // Atomically rebuild state to switch to the new fork
                match self.reorganize_to_fork(&valid_block) {
                    Ok(_) => {
                        println!("✅ Fork reorganization complete - state rebuilt");
                    }
                    Err(e) => {
                        // If the fork is invalid, we don't switch. Log the error.
                        eprintln!("🔥 Failed to switch to a longer fork: {:?}", e);
                    }
                }
            }
        } else {
            // Case 3: Orphan block
            return Err(ChainError::OrphanBlock);
        }

        Ok(())
    }

    /// Atomically reorganizes the blockchain to a new, longer fork.
    /// The entire new chain is validated and its state is built in memory.
    /// Only if that process succeeds is the main chain's state replaced.
    fn reorganize_to_fork(&mut self, new_head: &Block) -> Result<(), ChainError> {
        let (_, ancestor_height) = match self.find_common_ancestor(new_head) {
            Some(result) => result,
            None => return Err(ChainError::ForkNotFound),
        };

        // Prevent excessively deep reorganizations
        if new_head.header.height - ancestor_height > MAX_REORG_DEPTH {
            return Err(ChainError::InvalidBlock(format!(
                "Fork reorganization depth {} exceeds maximum of {}",
                new_head.header.height - ancestor_height,
                MAX_REORG_DEPTH
            )));
        }

        // 1. Build the full chain of the new fork in memory.
        let mut new_chain = Vec::new();
        let mut current_hash = new_head.hash;
        while let Some(block) = self.block_index.get(&current_hash) {
            new_chain.push(block.clone());
            if block.header.height == 0 {
                break; // Reached genesis
            }
            current_hash = block.header.previous_hash;
        }
        new_chain.reverse(); // Order from genesis to new_head

        // 2. Build the new UTXO state from scratch in a temporary variable.
        let new_state = Self::build_state_for_chain(&new_chain)?;

        // 3. Identify old main chain blocks that are now part of a fork.
        for old_block in self
            .blocks
            .iter()
            .filter(|b| !new_chain.iter().any(|nb| nb.hash == b.hash))
        {
            self.forks.insert(old_block.hash, old_block.clone());
        }

        // 4. ATOMIC SWAP: If state building was successful, replace the old chain and state.
        self.blocks = new_chain;
        self.state = new_state;

        // 5. Clean up forks map: remove blocks that are now on the main chain.
        for block in &self.blocks {
            self.forks.remove(&block.hash);
        }

        Ok(())
    }

    /// Finds the common ancestor block between the main chain and a fork.
    /// Returns the hash and height of the common ancestor.
    fn find_common_ancestor(&self, fork_head: &Block) -> Option<(Sha256Hash, BlockHeight)> {
        let mut current = fork_head.clone();
        loop {
            // If the hash is in our main chain's index, we've found the ancestor
            if self.blocks.iter().any(|b| b.hash == current.hash) {
                return Some((current.hash, current.header.height));
            }
            // If we've reached the genesis of the fork without finding a common point, something is wrong
            if current.header.height == 0 {
                return None;
            }
            // Move to the previous block in the fork
            current = match self.block_index.get(&current.header.previous_hash) {
                Some(block) => block.clone(),
                None => return None, // Should not happen in a valid fork
            };
        }
    }

    /// Builds a new TriangleState by replaying all transactions from a given chain of blocks.
    /// This is a pure function and doesn't modify the blockchain's current state.
    fn build_state_for_chain(blocks: &[Block]) -> Result<TriangleState, ChainError> {
        let mut new_state = TriangleState::new();
        // Initialize with genesis triangle
        let genesis = genesis_triangle();
        new_state.utxo_set.insert(genesis.hash(), genesis);

        // Replay all transactions, skipping the genesis block (as it has no transactions)
        for block in blocks.iter().skip(1) {
            for tx in &block.transactions {
                match tx {
                    Transaction::Subdivision(sub_tx) => {
                        new_state.apply_subdivision(sub_tx)?;
                    }
                    Transaction::Coinbase(cb_tx) => {
                        new_state.apply_coinbase(cb_tx, block.header.height)?;
                    }
                    Transaction::Transfer(transfer_tx) => {
                        // GEOMETRIC FEE DEDUCTION during fork rebuild:
                        // Same logic as apply_block

                        let old_triangle = new_state
                            .utxo_set
                            .remove(&transfer_tx.input_hash)
                            .ok_or_else(|| {
                                ChainError::TriangleNotFound(format!(
                                    "During fork rebuild, transfer input {} not found",
                                    hex::encode(transfer_tx.input_hash)
                                ))
                            })?;

                        let old_owner = old_triangle.owner.clone();
                        let old_value = old_triangle.effective_value();
                        let new_value = old_value - transfer_tx.fee_area;

                        // Remove from old owner's index
                        if let Some(hashes) = new_state.address_index.get_mut(&old_owner) {
                            hashes.retain(|h| h != &transfer_tx.input_hash);
                            if hashes.is_empty() {
                                new_state.address_index.remove(&old_owner);
                            }
                        }

                        // Create new triangle with reduced value and new owner
                        let new_triangle = crate::geometry::Triangle::new_with_value(
                            old_triangle.a,
                            old_triangle.b,
                            old_triangle.c,
                            old_triangle.parent_hash,
                            transfer_tx.new_owner.clone(),
                            new_value,
                        );

                        let new_hash = new_triangle.hash();
                        new_state.utxo_set.insert(new_hash, new_triangle);

                        // Add to new owner's index
                        new_state
                            .address_index
                            .entry(transfer_tx.new_owner.clone())
                            .or_default()
                            .push(new_hash);
                    }
                }
            }
        }
        Ok(new_state)
    }

    /// Calculate the total supply that has been mined up to a given block height
    /// This accounts for all halvings that have occurred
    pub fn calculate_current_supply(height: BlockHeight) -> u64 {
        if height == 0 {
            return 0;
        }

        let mut total_supply = 0u64;
        let mut current_height = 1u64; // Start from block 1 (first mined block)

        while current_height <= height {
            let reward = Self::calculate_block_reward(current_height);
            total_supply = total_supply.saturating_add(reward);
            current_height += 1;
        }

        total_supply
    }

    /// Calculate remaining supply that can still be mined
    pub fn calculate_remaining_supply(&self) -> u64 {
        let height = match self.blocks.last() {
            Some(block) => block.header.height,
            None => return MAX_SUPPLY, // Nothing mined yet
        };
        let current = Self::calculate_current_supply(height);
        MAX_SUPPLY.saturating_sub(current)
    }

    /// Get percentage of total supply mined
    pub fn supply_percentage(&self) -> f64 {
        let height = match self.blocks.last() {
            Some(block) => block.header.height,
            None => return 0.0,
        };
        let current = Self::calculate_current_supply(height);
        (current as f64 / MAX_SUPPLY as f64) * 100.0
    }

    /// Get the current halving era (0 = first era, 1 = first halving, etc.)
    pub fn current_halving_era(&self) -> u64 {
        match self.blocks.last() {
            Some(block) => block.header.height / REWARD_HALVING_INTERVAL,
            None => 0,
        }
    }

    /// Blocks until next halving
    pub fn blocks_until_next_halving(&self) -> u64 {
        let current_height = match self.blocks.last() {
            Some(block) => block.header.height,
            None => return REWARD_HALVING_INTERVAL,
        };
        let next_halving_height = (self.current_halving_era() + 1) * REWARD_HALVING_INTERVAL;
        next_halving_height.saturating_sub(current_height)
    }

    /// Calculate total geometric fee area in a block
    /// Returns the sum of all fee_area values from transfer and subdivision transactions
    pub fn calculate_total_fees(transactions: &[Transaction]) -> Coord {
        transactions
            .iter()
            .filter(|tx| !matches!(tx, Transaction::Coinbase(_)))
            .map(|tx| tx.fee_area())
            .sum()
    }

    /// Calculate total fees as u64 (for backward compatibility)
    /// Deprecated: Use calculate_total_fees() which returns f64
    pub fn calculate_total_fees_u64(transactions: &[Transaction]) -> u64 {
        Self::calculate_total_fees(transactions).to_num::<u64>()
    }

    fn adjust_difficulty(&mut self) {
        if self.blocks.len() < DIFFICULTY_ADJUSTMENT_WINDOW as usize {
            return; // Not enough blocks to adjust
        }

        let window_start_index = self.blocks.len() - DIFFICULTY_ADJUSTMENT_WINDOW as usize;
        let window = &self.blocks[window_start_index..];

        let (last_block, first_block) = match (window.last(), window.first()) {
            (Some(last), Some(first)) => (last, first),
            _ => {
                eprintln!("⚠️ Warning: Could not get first and last blocks from window in difficulty adjustment");
                return; // Should be unreachable
            }
        };
        let actual_time = last_block.header.timestamp - first_block.header.timestamp;

        // Timestamps should always increase; if they don't, there's a bug
        if actual_time <= 0 {
            eprintln!("⚠️  Warning: Invalid timestamp range detected in difficulty adjustment");
            return; // Don't adjust with invalid data
        }

        // Expected time for the window
        let expected_time = (DIFFICULTY_ADJUSTMENT_WINDOW as i64 - 1) * TARGET_BLOCK_TIME_SECONDS;

        // Calculate adjustment factor - how much faster/slower than target
        let adjustment_factor = expected_time as f64 / actual_time as f64;

        // Bitcoin-style clamping: limit adjustment to 4x in either direction per period
        // This prevents wild swings while still allowing quick convergence
        const MIN_ADJUSTMENT: f64 = 0.25; // Can decrease by up to 4x
        const MAX_ADJUSTMENT: f64 = 4.0; // Can increase by up to 4x

        let clamped_factor = adjustment_factor.clamp(MIN_ADJUSTMENT, MAX_ADJUSTMENT);

        let old_difficulty = self.difficulty;
        let new_difficulty = ((self.difficulty as f64 * clamped_factor).round() as u64).max(1);
        self.difficulty = new_difficulty;

        let avg_block_time = actual_time as f64 / (DIFFICULTY_ADJUSTMENT_WINDOW as f64 - 1.0);
        println!(
            "⚙️  Difficulty adjusted: {} -> {} (avg block time: {:.1}s, target: {}s)",
            old_difficulty, new_difficulty, avg_block_time, TARGET_BLOCK_TIME_SECONDS
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::transaction::{SubdivisionTx, Transaction};

    #[test]
    fn test_genesis_triangle_is_canonical() {
        let genesis = genesis_triangle();
        assert_eq!(genesis.a.x, Coord::from_num(0.0));
        assert_eq!(genesis.a.y, Coord::from_num(0.0));
        assert_eq!(genesis.b.x, Coord::from_num(1.0));
        assert_eq!(genesis.c.x, Coord::from_num(0.5));
        assert!((genesis.c.y - Coord::from_num(0.8660254)).abs() < Coord::from_num(1e-6));
    }

    #[test]
    fn test_block_merkle_root_calculation() {
        let coinbase = CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: "test".to_string(),
        };
        let transactions = vec![Transaction::Coinbase(coinbase)];
        let merkle = Block::calculate_merkle_root(&transactions);
        assert!(!merkle.is_empty());
    }

    #[test]
    fn test_merkle_tree_empty() {
        let root = Block::calculate_merkle_root(&[]);
        assert_eq!(root, [0; 32]);
    }

    #[test]
    fn test_merkle_tree_single() {
        let coinbase = CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: "miner".to_string(),
        };
        let txs = vec![Transaction::Coinbase(coinbase)];
        let root = Block::calculate_merkle_root(&txs);
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_merkle_tree_even() {
        let tx1 = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: "miner1".to_string(),
        });
        let tx2 = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(2000),
            beneficiary_address: "miner2".to_string(),
        });
        let root = Block::calculate_merkle_root(&[tx1, tx2]);
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_merkle_tree_odd() {
        let tx1 = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: "miner1".to_string(),
        });
        let tx2 = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(2000),
            beneficiary_address: "miner2".to_string(),
        });
        let tx3 = Transaction::Coinbase(CoinbaseTx {
            reward_area: Coord::from_num(3000),
            beneficiary_address: "miner3".to_string(),
        });
        let root = Block::calculate_merkle_root(&[tx1, tx2, tx3]);
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_apply_block_updates_state() {
        let mut chain = Blockchain::new();
        let initial_count = chain.state.count();

        let genesis_hash = *chain
            .state
            .utxo_set
            .keys()
            .next()
            .expect("Test setup should ensure this exists");
        let genesis_tri = chain
            .state
            .utxo_set
            .get(&genesis_hash)
            .expect("Test setup should ensure this exists")
            .clone();
        let children = genesis_tri.subdivide();

        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address.clone(),
            Coord::from_num(0),
            1,
        );
        let message = tx.signable_message();
        let signature = keypair
            .sign(&message)
            .expect("Test setup should ensure this exists");
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        let coinbase = CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: address,
        };

        let transactions = vec![
            Transaction::Coinbase(coinbase),
            Transaction::Subdivision(tx),
        ];

        let last_block = chain
            .blocks
            .last()
            .expect("Test setup should ensure this exists");
        let mut new_block = Block::new(
            last_block.header.height + 1,
            last_block.hash,
            chain.difficulty,
            transactions,
        );

        // Ensure timestamp is greater than parent
        new_block.header.timestamp = last_block.header.timestamp + 1;
        new_block.hash = new_block.calculate_hash();

        while !new_block.verify_proof_of_work() {
            new_block.header.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        chain
            .apply_block(new_block)
            .expect("Test setup should ensure this exists");

        // Initial state has 1 triangle (genesis).
        // Subdivision tx consumes 1 and creates 3 (+2).
        // Coinbase tx creates 1 (+1).
        // Total should be 1 + 2 + 1 = 4.
        assert_eq!(chain.state.count(), initial_count + 3);
    }

    #[test]
    fn test_block_validation_success() {
        let chain = Blockchain::new();
        let genesis_hash = *chain
            .state
            .utxo_set
            .keys()
            .next()
            .expect("Test setup should ensure this exists");
        let genesis_tri = chain
            .state
            .utxo_set
            .get(&genesis_hash)
            .expect("Test setup should ensure this exists")
            .clone();
        let children = genesis_tri.subdivide();

        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address.clone(),
            Coord::from_num(0),
            1,
        );
        let message = tx.signable_message();
        let signature = keypair
            .sign(&message)
            .expect("Test setup should ensure this exists");
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        let coinbase = CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: address,
        };

        let transactions = vec![
            Transaction::Coinbase(coinbase),
            Transaction::Subdivision(tx),
        ];

        let last_block = chain
            .blocks
            .last()
            .expect("Test setup should ensure this exists");
        let mut new_block = Block::new(
            last_block.header.height + 1,
            last_block.hash,
            chain.difficulty,
            transactions,
        );

        // Ensure timestamp is greater than parent
        new_block.header.timestamp = last_block.header.timestamp + 1;
        new_block.hash = new_block.calculate_hash();

        while !new_block.verify_proof_of_work() {
            new_block.header.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        assert!(chain.validate_block(&new_block).is_ok());
    }

    #[test]
    fn test_block_validation_failure_linkage() {
        let chain = Blockchain::new();
        let last_block = chain
            .blocks
            .last()
            .expect("Test setup should ensure this exists");

        let mut bad_block = Block::new(
            last_block.header.height + 1,
            [1; 32],
            chain.difficulty,
            vec![],
        );

        bad_block.hash = bad_block.calculate_hash();

        while !bad_block.verify_proof_of_work() {
            bad_block.header.nonce += 1;
            bad_block.hash = bad_block.calculate_hash();
        }

        assert!(chain.validate_block(&bad_block).is_err());
    }

    #[test]
    fn test_block_validation_failure_pow() {
        let chain = Blockchain::new();
        let last_block = chain
            .blocks
            .last()
            .expect("Test setup should ensure this exists");

        let bad_block = Block::new(
            last_block.header.height + 1,
            last_block.hash,
            chain.difficulty,
            vec![],
        );

        assert!(chain.validate_block(&bad_block).is_err());
    }

    #[test]
    fn test_block_validation_double_spend_in_block() {
        let mut chain = Blockchain::new();
        let genesis_hash = *chain
            .state
            .utxo_set
            .keys()
            .next()
            .expect("Test setup should ensure this exists");
        let genesis_tri = chain
            .state
            .utxo_set
            .get(&genesis_hash)
            .expect("Test setup should ensure this exists")
            .clone();
        let children = genesis_tri.subdivide();

        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();

        let mut tx1 = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address.clone(),
            Coord::from_num(0),
            1,
        );
        let message1 = tx1.signable_message();
        let signature1 = keypair
            .sign(&message1)
            .expect("Test setup should ensure this exists");
        let public_key1 = keypair.public_key.serialize().to_vec();
        tx1.sign(signature1, public_key1);

        let mut tx2 = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address.clone(),
            Coord::from_num(0),
            2,
        );
        let message2 = tx2.signable_message();
        let signature2 = keypair
            .sign(&message2)
            .expect("Test setup should ensure this exists");
        let public_key2 = keypair.public_key.serialize().to_vec();
        tx2.sign(signature2, public_key2);

        let coinbase = CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: address,
        };

        let transactions = vec![
            Transaction::Coinbase(coinbase),
            Transaction::Subdivision(tx1),
            Transaction::Subdivision(tx2),
        ];

        let last_block = chain
            .blocks
            .last()
            .expect("Test setup should ensure this exists");
        let mut new_block = Block::new(
            last_block.header.height + 1,
            last_block.hash,
            chain.difficulty,
            transactions,
        );

        new_block.hash = new_block.calculate_hash();

        while !new_block.verify_proof_of_work() {
            new_block.header.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        assert!(chain.apply_block(new_block).is_err());
    }

    #[test]
    fn test_difficulty_adjustment_increase() {
        let mut chain = Blockchain::new();

        for i in 1..=10 {
            let block = Block {
                header: BlockHeader {
                    height: i,
                    previous_hash: chain
                        .blocks
                        .last()
                        .expect("Test setup should ensure this exists")
                        .hash,
                    timestamp: Utc::now().timestamp() + (i as i64 * 10),
                    difficulty: chain.difficulty,
                    nonce: 0,
                    merkle_root: [0; 32],
                    headline: None,
                },
                hash: [i as u8; 32],
                transactions: vec![],
            };

            chain.blocks.push(block);
            chain.adjust_difficulty();
        }

        assert!(chain.difficulty >= 2);
    }

    #[test]
    fn test_difficulty_adjustment_decrease() {
        let mut chain = Blockchain::new();

        for i in 1..=10 {
            let block = Block {
                header: BlockHeader {
                    height: i,
                    previous_hash: chain
                        .blocks
                        .last()
                        .expect("Test setup should ensure this exists")
                        .hash,
                    timestamp: Utc::now().timestamp() + (i as i64 * 200),
                    difficulty: chain.difficulty,
                    nonce: 0,
                    merkle_root: [0; 32],
                    headline: None,
                },
                hash: [i as u8; 32],
                transactions: vec![],
            };

            chain.blocks.push(block);
            chain.adjust_difficulty();
        }

        assert!(chain.difficulty <= 2);
    }

    #[test]
    fn test_difficulty_adjustment_no_change() {
        let mut chain = Blockchain::new();
        let initial_difficulty = chain.difficulty;

        for i in 1..=10 {
            let block = Block {
                header: BlockHeader {
                    height: i,
                    previous_hash: chain
                        .blocks
                        .last()
                        .expect("Test setup should ensure this exists")
                        .hash,
                    timestamp: Utc::now().timestamp() + (i as i64 * 60),
                    difficulty: chain.difficulty,
                    nonce: 0,
                    merkle_root: [0; 32],
                    headline: None,
                },
                hash: [i as u8; 32],
                transactions: vec![],
            };

            chain.blocks.push(block);
            chain.adjust_difficulty();
        }

        assert_eq!(chain.difficulty, initial_difficulty);
    }

    #[test]
    fn test_mempool_add_transaction() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash, genesis.clone());
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let message = valid_tx.signable_message();
        let signature = keypair
            .sign(&message)
            .expect("Test setup should ensure this exists");
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);

        mempool
            .add_transaction(tx.clone())
            .expect("Test setup should ensure this exists");
        assert_eq!(mempool.len(), 1);
        assert!(!mempool.is_empty());
    }

    #[test]
    fn test_mempool_remove_transaction() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash, genesis.clone());
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let message = valid_tx.signable_message();
        let signature = keypair
            .sign(&message)
            .expect("Test setup should ensure this exists");
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);
        let tx_hash = tx.hash();

        mempool
            .add_transaction(tx.clone())
            .expect("Test setup should ensure this exists");
        assert_eq!(mempool.len(), 1);

        mempool.remove_transaction(&tx_hash);
        assert_eq!(mempool.len(), 0);
    }

    #[test]
    fn test_mempool_duplicate_transaction() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash, genesis.clone());
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let message = valid_tx.signable_message();
        let signature = keypair
            .sign(&message)
            .expect("Test setup should ensure this exists");
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);

        mempool
            .add_transaction(tx.clone())
            .expect("Test setup should ensure this exists");
        let result = mempool.add_transaction(tx.clone());

        assert!(result.is_err());
        assert_eq!(mempool.len(), 1);
    }

    #[test]
    fn test_blockchain_with_mempool() {
        let mut chain = Blockchain::new();
        assert!(chain.mempool.is_empty());

        // Add a transaction to mempool
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(
            genesis_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let message = valid_tx.signable_message();
        let signature = keypair
            .sign(&message)
            .expect("Test setup should ensure this exists");
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);
        chain
            .mempool
            .add_transaction(tx.clone())
            .expect("Test setup should ensure this exists");
        assert_eq!(chain.mempool.len(), 1);

        // Create and apply a block with that transaction
        let last_block = chain
            .blocks
            .last()
            .expect("Test setup should ensure this exists");
        let coinbase = CoinbaseTx {
            reward_area: Coord::from_num(1000),
            beneficiary_address: "miner_address".to_string(),
        };
        let mut new_block = Block::new(
            last_block.header.height + 1,
            last_block.hash,
            chain.difficulty,
            vec![Transaction::Coinbase(coinbase), tx],
        );

        // Ensure timestamp is greater than parent
        new_block.header.timestamp = last_block.header.timestamp + 1;

        // Before applying, mempool has 1 transaction
        assert_eq!(chain.mempool.len(), 1);

        // Apply block (this should remove the transaction from mempool)
        let mut mined_block = new_block.clone();
        loop {
            mined_block.hash = mined_block.calculate_hash();
            if mined_block.verify_proof_of_work() {
                break;
            }
            mined_block.header.nonce += 1;
        }

        chain
            .apply_block(mined_block)
            .expect("Test setup should ensure this exists");

        // After applying, mempool should be empty
        assert_eq!(chain.mempool.len(), 0);
    }

    #[test]
    fn test_mining_reward_halving() {
        // Test initial reward
        assert_eq!(Blockchain::calculate_block_reward(0), 1000);
        assert_eq!(Blockchain::calculate_block_reward(1), 1000);
        assert_eq!(Blockchain::calculate_block_reward(209_999), 1000);

        // Test first halving at block 210,000
        assert_eq!(Blockchain::calculate_block_reward(210_000), 500);
        assert_eq!(Blockchain::calculate_block_reward(419_999), 500);

        // Test second halving at block 420,000
        assert_eq!(Blockchain::calculate_block_reward(420_000), 250);

        // Test third halving
        assert_eq!(Blockchain::calculate_block_reward(630_000), 125);

        // Test many halvings (reward approaches zero)
        assert_eq!(Blockchain::calculate_block_reward(210_000 * 10), 0); // After 10 halvings, reward is <1
    }

    #[test]
    fn test_transaction_fee_calculation() {
        use crate::transaction::{SubdivisionTx, TransferTx};

        let genesis = genesis_triangle();
        let children = genesis.subdivide();
        let address = "test_address".to_string();

        // Test subdivision transaction with fee (still u64 for SubdivisionTx)
        let sub_tx = SubdivisionTx::new(
            genesis.hash(),
            children.to_vec(),
            address.clone(),
            Coord::from_num(100),
            1,
        );
        let tx1 = Transaction::Subdivision(sub_tx);
        assert_eq!(tx1.fee_area(), Coord::from_num(100));

        // Test transfer transaction with geometric fee_area (f64)
        let transfer_tx = TransferTx::new(
            genesis.hash(),
            "new_owner".to_string(),
            address,
            Coord::from_num(0),
            Coord::from_num(50.5), // Geometric fee area
            1,
        );
        let tx2 = Transaction::Transfer(transfer_tx);
        assert_eq!(tx2.fee_area(), Coord::from_num(50.5));

        // Test total fees calculation (now returns f64)
        let transactions = vec![tx1, tx2];
        let total_fees = Blockchain::calculate_total_fees(&transactions);
        assert_eq!(total_fees, Coord::from_num(150.5));
    }

    #[test]
    fn test_mempool_fee_prioritization() {
        use crate::transaction::SubdivisionTx;

        let mut chain = Blockchain::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().expect("Test setup should ensure this exists");
        let address = keypair.address();

        // Create transactions with different fees
        for (i, fee) in [10u64, 50, 25, 100, 5].iter().enumerate() {
            let mut tx = SubdivisionTx::new(
                genesis_hash,
                children.to_vec(),
                address.clone(),
                Coord::from_num(*fee),
                i as u64,
            );
            let message = tx.signable_message();
            let signature = keypair
                .sign(&message)
                .expect("Test setup should ensure this exists");
            let public_key = keypair.public_key.serialize().to_vec();
            tx.sign(signature, public_key);
            chain
                .mempool
                .add_transaction(Transaction::Subdivision(tx))
                .expect("Test setup should ensure this exists");
        }

        assert_eq!(chain.mempool.len(), 5);

        // Get transactions sorted by fee
        let sorted_txs = chain.mempool.get_transactions_by_fee(5);
        assert_eq!(sorted_txs.len(), 5);

        // Verify they're sorted by fee (highest first)
        assert_eq!(sorted_txs[0].fee(), 100);
        assert_eq!(sorted_txs[1].fee(), 50);
        assert_eq!(sorted_txs[2].fee(), 25);
        assert_eq!(sorted_txs[3].fee(), 10);
        assert_eq!(sorted_txs[4].fee(), 5);

        // Test limit parameter
        let top_3 = chain.mempool.get_transactions_by_fee(3);
        assert_eq!(top_3.len(), 3);
        assert_eq!(top_3[0].fee(), 100);
        assert_eq!(top_3[1].fee(), 50);
        assert_eq!(top_3[2].fee(), 25);
    }

    #[test]
    fn test_wallet_to_wallet_transfer_with_change() {
        use crate::transaction::TransferTx;

        // 1. Setup blockchain and two wallets
        let mut chain = Blockchain::new();
        let wallet_a = KeyPair::generate().unwrap();
        let wallet_b = KeyPair::generate().unwrap();
        let address_a = wallet_a.address();
        let address_b = wallet_b.address();

        // 2. Create a large "coin" (triangle) for Wallet A
        let initial_triangle = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(10.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(5.0), Coord::from_num(10.0)), // Area = 50
            None,
            address_a.clone(),
        );
        let initial_hash = initial_triangle.hash();
        chain.state.utxo_set.insert(initial_hash, initial_triangle);
        chain.state.rebuild_address_index();

        assert_eq!(chain.state.get_balance(&address_a), Coord::from_num(50.0));
        assert_eq!(chain.state.get_balance(&address_b), Coord::from_num(0.0));

        // 3. Create a transfer transaction from A to B for a smaller amount
        let amount_to_send = Coord::from_num(10.0);
        let fee = Coord::from_num(1.0);
        let mut tx = TransferTx::new(
            initial_hash,
            address_b.clone(),
            address_a.clone(),
            amount_to_send,
            fee,
            1, // nonce
        );

        let message = tx.signable_message();
        let signature = wallet_a.sign(&message).unwrap();
        let public_key = wallet_a.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        let transaction = Transaction::Transfer(tx);

        // 4. Create and apply a new block with this transaction
        let last_block = chain.blocks.last().unwrap();
        let mut new_block = Block::new_with_parent_time(
            last_block.header.height + 1,
            last_block.hash,
            last_block.header.timestamp,
            chain.difficulty,
            vec![
                Transaction::Coinbase(CoinbaseTx {
                    reward_area: Coord::from_num(1000), // Miner gets block reward
                    beneficiary_address: "miner".to_string(),
                }),
                transaction,
            ],
        );

        // Mine the block
        while !new_block.verify_proof_of_work() {
            new_block.header.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        chain.apply_block(new_block).unwrap();

        // 5. Verify the state
        // The original large triangle should be gone
        assert!(!chain.state.utxo_set.contains_key(&initial_hash));

        // Wallet B should have a new triangle with the exact amount
        let triangles_b = chain.state.get_triangles_by_owner(&address_b);
        assert_eq!(triangles_b.len(), 1);
        assert_eq!(triangles_b[0].effective_value(), amount_to_send);

        // Wallet A should have a new "change" triangle
        let triangles_a = chain.state.get_triangles_by_owner(&address_a);
        let expected_change = Coord::from_num(50.0) - amount_to_send - fee;
        assert_eq!(triangles_a.len(), 1);
        assert_eq!(triangles_a[0].effective_value(), expected_change);

        // Check total balances
        assert_eq!(chain.state.get_balance(&address_a), expected_change);
        assert_eq!(chain.state.get_balance(&address_b), amount_to_send);
    }
}
