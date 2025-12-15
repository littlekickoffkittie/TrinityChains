//! Transaction types for TrinityChain

use crate::crypto::Address;
use crate::blockchain::{Sha256Hash, TriangleState};
use crate::error::ChainError;
use crate::geometry::{Coord, Triangle};
use sha2::{Digest, Sha256};


/// Maximum transaction size in bytes (100KB) to prevent DoS
pub const MAX_TRANSACTION_SIZE: usize = 100_000;

/// A transaction that can occur in a block
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Transaction {
    Transfer(TransferTx),
    Subdivision(SubdivisionTx),
    Coinbase(CoinbaseTx),
}

impl Transaction {
    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    /// Validate transaction size to prevent DoS attacks
    pub fn validate_size(&self) -> Result<(), ChainError> {
        let serialized = bincode::serialize(self)
            .map_err(|e| ChainError::InvalidTransaction(format!("Serialization failed: {}", e)))?;

        if serialized.len() > MAX_TRANSACTION_SIZE {
            return Err(ChainError::InvalidTransaction(format!(
                "Transaction too large: {} bytes (max: {})",
                serialized.len(),
                MAX_TRANSACTION_SIZE
            )));
        }
        Ok(())
    }

    /// Get the geometric fee area for this transaction
    pub fn fee_area(&self) -> crate::geometry::Coord {
        match self {
            Transaction::Subdivision(tx) => tx.fee_area,
            Transaction::Transfer(tx) => tx.fee_area,
            Transaction::Coinbase(_) => Coord::from_num(0), // Coinbase has no fee
        }
    }

    /// Get the fee as u64 (for backward compatibility, converts fee_area)
    /// Deprecated: Use fee_area() for geometric fees
    pub fn fee(&self) -> u64 {
        self.fee_area().to_num::<u64>()
    }

    /// Calculate the hash of this transaction
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match self {
            Transaction::Subdivision(tx) => {
                hasher.update(tx.parent_hash);
                for child in &tx.children {
                    hasher.update(child.hash());
                }
                hasher.update(tx.owner_address);
                hasher.update(tx.fee_area.to_le_bytes());
                hasher.update(tx.nonce.to_le_bytes());
            }
            Transaction::Coinbase(tx) => {
                hasher.update("coinbase".as_bytes());
                hasher.update(tx.reward_area.to_le_bytes());
                hasher.update(tx.beneficiary_address);
                hasher.update(tx.nonce.to_le_bytes());
            }
            Transaction::Transfer(tx) => {
                hasher.update("transfer".as_bytes());
                hasher.update(tx.input_hash);
                hasher.update(tx.new_owner);
                hasher.update(tx.sender);
                hasher.update(tx.amount.to_le_bytes());
                hasher.update(tx.fee_area.to_le_bytes());
                hasher.update(tx.nonce.to_le_bytes());
            }
        };
        hasher.finalize().into()
    }

    /// Validate this transaction against the current UTXO state
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        match self {
            Transaction::Subdivision(tx) => tx.validate(state),
            Transaction::Coinbase(tx) => tx.validate(),
            Transaction::Transfer(tx) => tx.validate(),
        }
    }
}

/// Subdivision transaction: splits one parent triangle into three children
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubdivisionTx {
    pub parent_hash: Sha256Hash,
    pub children: Vec<Triangle>,
    pub owner_address: Address,
    pub fee_area: Coord,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

impl SubdivisionTx {
    pub fn new(
        parent_hash: Sha256Hash,
        children: Vec<Triangle>,
        owner_address: Address,
        fee_area: Coord,
        nonce: u64,
    ) -> Self {
        SubdivisionTx {
            parent_hash,
            children,
            owner_address,
            fee_area,
            nonce,
            signature: None,
            public_key: None,
        }
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.parent_hash);
        for child in &self.children {
            message.extend_from_slice(&child.hash());
        }
        message.extend_from_slice(&self.owner_address);
        message.extend_from_slice(&self.fee_area.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }

    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }

    /// Validates just the signature of the transaction, without access to blockchain state.
    /// This is useful for early validation in the mempool.
    pub fn validate_signature(&self) -> Result<(), ChainError> {
        let (signature, public_key) = match (&self.signature, &self.public_key) {
            (Some(sig), Some(pk)) => (sig, pk),
            _ => {
                return Err(ChainError::InvalidTransaction(
                    "Transaction not signed".to_string(),
                ))
            }
        };

        let message = self.signable_message();
        crate::crypto::verify_signature(public_key, &message, signature)?;

        Ok(())
    }

    /// Performs a full validation of the transaction against the current blockchain state.
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        // First, perform a stateless signature check.
        self.validate_signature()?;

        // Then, validate against the current state (UTXO set).
        let parent = match state.utxo_set.get(&self.parent_hash) {
            Some(triangle) => triangle,
            None => {
                return Err(ChainError::TriangleNotFound(format!(
                    "Parent triangle {} not found in UTXO set",
                    hex::encode(self.parent_hash)
                )))
            }
        };

        // Verify that the transaction's owner address matches the parent triangle's owner
        if parent.owner != self.owner_address {
            return Err(ChainError::InvalidTransaction(format!(
                "Subdivision transaction owner {} does not match parent triangle owner {}",
                hex::encode(self.owner_address), hex::encode(parent.owner)
            )));
        }

        let expected_children = parent.subdivide();

        if self.children.len() != 3 {
            return Err(ChainError::InvalidTransaction(
                "Subdivision must produce exactly 3 children".to_string(),
            ));
        }

        for (i, child) in self.children.iter().enumerate() {
            let expected = &expected_children[i];
            if !child.a.equals(&expected.a)
                || !child.b.equals(&expected.b)
                || !child.c.equals(&expected.c)
            {
                return Err(ChainError::InvalidTransaction(format!(
                    "Child {} geometry does not match expected subdivision",
                    i
                )));
            }
        }

        Ok(())
    }
}

/// Coinbase transaction: miner reward
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoinbaseTx {
    pub reward_area: Coord,
    pub beneficiary_address: Address,
    #[serde(default)]
    pub nonce: u64,
}

impl CoinbaseTx {
    /// Maximum reward area that can be claimed in a coinbase transaction
    pub const MAX_REWARD_AREA: Coord = Coord::from_bits(1000i64 << 32);

    pub fn validate(&self) -> Result<(), ChainError> {
        // Validate reward area is within acceptable bounds
        if self.reward_area <= Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Coinbase reward area must be greater than zero".to_string(),
            ));
        }

        if self.reward_area > Self::MAX_REWARD_AREA {
            return Err(ChainError::InvalidTransaction(format!(
                "Coinbase reward area {} exceeds maximum {}",
                self.reward_area,
                Self::MAX_REWARD_AREA
            )));
        }

        // Validate beneficiary address is not empty
        if self.beneficiary_address == [0; 32] {
            return Err(ChainError::InvalidTransaction(
                "Coinbase beneficiary address cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Transfer transaction - moves ownership of a triangle
/// Fee is now geometric: fee_area is deducted from the triangle's value
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferTx {
    pub input_hash: Sha256Hash,
    pub new_owner: Address,
    pub sender: Address,
    /// Amount being sent to the new owner
    pub amount: crate::geometry::Coord,
    /// Geometric fee: area deducted from triangle value and given to miner
    pub fee_area: crate::geometry::Coord,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
    #[serde(default)]
    pub memo: Option<String>,
}

impl TransferTx {
    /// Maximum memo length (256 characters)
    pub const MAX_MEMO_LENGTH: usize = 256;
    pub fn new(
        input_hash: Sha256Hash,
        new_owner: Address,
        sender: Address,
        amount: crate::geometry::Coord,
        fee_area: crate::geometry::Coord,
        nonce: u64,
    ) -> Self {
        TransferTx {
            input_hash,
            new_owner,
            sender,
            amount,
            fee_area,
            nonce,
            signature: None,
            public_key: None,
            memo: None,
        }
    }

    pub fn with_memo(mut self, memo: String) -> Result<Self, ChainError> {
        if memo.len() > Self::MAX_MEMO_LENGTH {
            return Err(ChainError::InvalidTransaction(format!(
                "Memo exceeds maximum length of {} characters",
                Self::MAX_MEMO_LENGTH
            )));
        }
        self.memo = Some(memo);
        Ok(self)
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice("TRANSFER:".as_bytes());
        message.extend_from_slice(&self.input_hash);
        message.extend_from_slice(&self.new_owner);
        message.extend_from_slice(&self.sender);
        message.extend_from_slice(&self.amount.to_le_bytes());
        // Use f64 bytes for geometric fee
        message.extend_from_slice(&self.fee_area.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }

    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }

    /// Stateless validation: checks signature, addresses, memo, and fee bounds.
    /// Does NOT validate against UTXO state - use validate_with_state() for that.
    pub fn validate(&self) -> Result<(), ChainError> {
        if self.signature.is_none() || self.public_key.is_none() {
            return Err(ChainError::InvalidTransaction(
                "Transfer not signed".to_string(),
            ));
        }

        // Validate addresses are not empty
        if self.sender == [0; 32] {
            return Err(ChainError::InvalidTransaction(
                "Sender address cannot be empty".to_string(),
            ));
        }
        if self.new_owner == [0; 32] {
            return Err(ChainError::InvalidTransaction(
                "New owner address cannot be empty".to_string(),
            ));
        }
        // Prevent self-sends
        if self.sender == self.new_owner {
            return Err(ChainError::InvalidTransaction(
                "Sender and new owner cannot be the same".to_string(),
            ));
        }

        // Validate amount and fee are non-negative and not both zero
        if self.amount < Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Transfer amount cannot be negative".to_string(),
            ));
        }
        if self.fee_area < Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Fee area cannot be negative".to_string(),
            ));
        }
        if self.amount == Coord::from_num(0) && self.fee_area == Coord::from_num(0) {
            return Err(ChainError::InvalidTransaction(
                "Amount and fee cannot both be zero".to_string(),
            ));
        }

        // Validate memo length to prevent DoS attacks
        if let Some(ref memo) = self.memo {
            if memo.len() > Self::MAX_MEMO_LENGTH {
                return Err(ChainError::InvalidTransaction(format!(
                    "Memo exceeds maximum length of {} characters",
                    Self::MAX_MEMO_LENGTH
                )));
            }
        }

        let (signature, public_key) = match (&self.signature, &self.public_key) {
            (Some(sig), Some(pk)) => (sig, pk),
            _ => {
                return Err(ChainError::InvalidTransaction(
                    "Transfer not signed".to_string(),
                ))
            }
        };

        let message = self.signable_message();
        crate::crypto::verify_signature(public_key, &message, signature)?;

        Ok(())
    }

    /// Full validation including UTXO state check.
    /// Ensures: input triangle exists AND input.effective_value() > fee_area + TOLERANCE
    pub fn validate_with_state(&self, state: &TriangleState) -> Result<(), ChainError> {
        // First perform stateless validation
        self.validate()?;

        // Check input triangle exists in UTXO set
        let input_triangle = state.utxo_set.get(&self.input_hash).ok_or_else(|| {
            ChainError::TriangleNotFound(format!(
                "Transfer input {} not found in UTXO set",
                hex::encode(self.input_hash)
            ))
        })?;

        // Area balance check: input value must be strictly greater than fee
        let input_value = input_triangle.effective_value();
        let total_spent = self.amount + self.fee_area;
        let remaining_value = input_value - total_spent;

        if remaining_value < crate::geometry::GEOMETRIC_TOLERANCE {
            return Err(ChainError::InvalidTransaction(format!(
                "Insufficient triangle value: input has {} but amount + fee_area is {}, leaving {} (minimum: {})",
                input_value, total_spent, remaining_value, crate::geometry::GEOMETRIC_TOLERANCE
            )));
        }

        // Verify sender owns the triangle
        if input_triangle.owner != self.sender {
            return Err(ChainError::InvalidTransaction(format!(
                "Sender {} does not own input triangle (owned by {})",
                hex::encode(self.sender), hex::encode(input_triangle.owner)
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::TriangleState;
    use crate::crypto::KeyPair;
    use crate::geometry::{Coord, Point, Triangle};

    fn create_test_address(s: &str) -> Address {
        let mut address = [0u8; 32];
        let bytes = s.as_bytes();
        address[..bytes.len()].copy_from_slice(bytes);
        address
    }

    #[test]
    fn test_tx_validation_success() {
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            address.clone(),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();

        let mut tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);
        assert!(tx.validate(&state).is_ok());
    }

    #[test]
    fn test_unsigned_transaction_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let address = create_test_address("test_address");

        let tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_invalid_signature_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );
        let fake_signature = vec![0u8; 64];
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(fake_signature, public_key);

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_area_conservation_failure() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent);

        let bad_child = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(2.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(1.732)),
            None,
            create_test_address("test_owner"),
        );
        let children = vec![bad_child.clone(), bad_child.clone(), bad_child];

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let tx = SubdivisionTx::new(parent_hash, children, address, Coord::from_num(0), 1);
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_double_spend_check() {
        let state = TriangleState::new();

        let parent = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(0.866)),
            None,
            create_test_address("test_owner"),
        );
        let parent_hash = parent.hash();
        let children = parent.subdivide();

        let address = create_test_address("test_address");
        let tx = SubdivisionTx::new(
            parent_hash,
            children.to_vec(),
            address,
            Coord::from_num(0),
            1,
        );

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_geometric_fee_deduction() {
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let sender_address = keypair.address();

        let large_triangle = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(4.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.0), Coord::from_num(5.0)),
            None,
            sender_address.clone(),
        );

        let triangle_hash = large_triangle.hash();
        assert_eq!(large_triangle.area(), Coord::from_num(10.0));

        state.utxo_set.insert(triangle_hash, large_triangle);

        let fee_area = Coord::from_num(0.0001);
        let recipient_address = create_test_address("recipient_address");

        let mut tx = TransferTx::new(
            triangle_hash,
            recipient_address.clone(),
            sender_address.clone(),
            Coord::from_num(1.0), // Amount > fee
            fee_area,
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);

        assert!(tx.validate_with_state(&state).is_ok());

        let old_triangle = state.utxo_set.remove(&triangle_hash).unwrap();
        let new_value = old_triangle.effective_value() - fee_area;

        let new_triangle = Triangle::new_with_value(
            old_triangle.a,
            old_triangle.b,
            old_triangle.c,
            old_triangle.parent_hash,
            recipient_address.clone(),
            new_value,
        );

        let new_hash = new_triangle.hash();
        state.utxo_set.insert(new_hash, new_triangle);

        let result_triangle = state.utxo_set.get(&new_hash).unwrap();
        assert_eq!(result_triangle.owner, recipient_address);

        let expected_value = Coord::from_num(10.0) - Coord::from_num(0.0001);
        assert_eq!(result_triangle.effective_value(), expected_value);
        assert_eq!(result_triangle.area(), Coord::from_num(10.0));
    }

    #[test]
    fn test_geometric_fee_insufficient_value() {
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let sender_address = keypair.address();

        let small_triangle = Triangle::new(
            Point::new(Coord::from_num(0.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(1.0), Coord::from_num(0.0)),
            Point::new(Coord::from_num(0.5), Coord::from_num(1.0)),
            None,
            sender_address.clone(),
        );

        let triangle_hash = small_triangle.hash();
        let triangle_area = small_triangle.area();
        state.utxo_set.insert(triangle_hash, small_triangle);

        let fee_area = triangle_area + Coord::from_num(0.1);

        let mut tx = TransferTx::new(
            triangle_hash,
            create_test_address("recipient"),
            sender_address.clone(),
            Coord::from_num(0),
            fee_area,
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);

        let result = tx.validate_with_state(&state);
        assert!(result.is_err());

        if let Err(ChainError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Insufficient"));
        } else {
            panic!("Expected InvalidTransaction error");
        }
    }

    #[test]
    fn test_negative_fee_rejected() {
        let keypair = KeyPair::generate().unwrap();

        let mut tx = TransferTx::new(
            [0u8; 32],
            create_test_address("recipient"),
            keypair.address(),
            Coord::from_num(0),
            Coord::from_num(-1.0), // Negative fee
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature.to_vec(), public_key);

        let result = tx.validate();
        assert!(result.is_err());
    }
}
