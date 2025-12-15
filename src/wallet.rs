//! Wallet management for TrinityChain
//!
//! Provides functionality for creating, loading, and managing wallets
//! that store keypairs and track triangle ownership.

// Suppress deprecation warnings from aes-gcm's generic-array dependency
#![allow(deprecated)]

use crate::crypto::KeyPair;
use crate::error::ChainError;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Wallet data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Optional wallet name
    pub name: Option<String>,
    /// Wallet address (derived from public key)
    pub address: String,
    /// Secret key (hex-encoded)
    #[serde(rename = "secret_key")]
    pub secret_key_hex: String,
    /// Creation timestamp
    pub created: String,
}

impl Wallet {
    /// Create a new wallet with a generated keypair
    pub fn new(name: Option<String>) -> Result<Self, ChainError> {
        let keypair = KeyPair::generate()?;
        let address = hex::encode(keypair.address());
        let secret_key_hex = hex::encode(keypair.secret_key.secret_bytes());

        Ok(Wallet {
            name,
            address,
            secret_key_hex,
            created: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Load a wallet from a file
    pub fn load(path: &PathBuf) -> Result<Self, ChainError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ChainError::WalletError(format!("Failed to read wallet: {}", e)))?;

        let wallet: Wallet = serde_json::from_str(&contents)
            .map_err(|e| ChainError::WalletError(format!("Failed to parse wallet: {}", e)))?;

        Ok(wallet)
    }

    /// Save the wallet to a file
    pub fn save(&self, path: &PathBuf) -> Result<(), ChainError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ChainError::WalletError(format!("Failed to serialize wallet: {}", e)))?;

        let mut temp_file = NamedTempFile::new()
            .map_err(|e| ChainError::WalletError(format!("Failed to create temp file: {}", e)))?;
        write!(temp_file, "{}", json)
            .map_err(|e| ChainError::WalletError(format!("Failed to write to temp file: {}", e)))?;
        temp_file.persist(path).map_err(|e| {
            ChainError::WalletError(format!("Failed to persist wallet file: {}", e))
        })?;

        Ok(())
    }

    /// Get the keypair from the wallet
    pub fn get_keypair(&self) -> Result<KeyPair, ChainError> {
        let secret_bytes = hex::decode(&self.secret_key_hex)
            .map_err(|e| ChainError::WalletError(format!("Failed to decode secret key: {}", e)))?;

        KeyPair::from_secret_bytes(&secret_bytes)
    }
}

/// Get the default wallet directory
pub fn get_wallet_dir() -> Result<PathBuf, ChainError> {
    let home = std::env::var("HOME")
        .map_err(|e| ChainError::WalletError(format!("Failed to get HOME env var: {}", e)))?;
    Ok(PathBuf::from(home).join(".trinitychain"))
}

/// Get the default wallet file path
pub fn get_default_wallet_path() -> Result<PathBuf, ChainError> {
    Ok(get_wallet_dir()?.join("wallet.json"))
}

/// Get a named wallet file path
pub fn get_named_wallet_path(name: &str) -> Result<PathBuf, ChainError> {
    Ok(get_wallet_dir()?.join(format!("wallet_{}.json", name)))
}

/// Create the wallet directory if it doesn't exist
pub fn ensure_wallet_dir() -> Result<(), ChainError> {
    let wallet_dir = get_wallet_dir()?;
    fs::create_dir_all(&wallet_dir).map_err(|e| {
        ChainError::WalletError(format!("Failed to create wallet directory: {}", e))
    })?;
    Ok(())
}

/// Create a new wallet and save it to the default location
pub fn create_default_wallet() -> Result<Wallet, ChainError> {
    ensure_wallet_dir()?;
    let path = get_default_wallet_path()?;
    if path.exists() {
        return Err(ChainError::WalletError(
            "Wallet already exists at default location".to_string(),
        ));
    }
    let password = prompt_password("Enter a password for your new wallet: ")
        .map_err(|e| ChainError::WalletError(format!("Failed to read password: {}", e)))?;
    let wallet = Wallet::new(None)?;
    let encrypted_wallet = EncryptedWallet::from_wallet(&wallet, &password)?;
    encrypted_wallet.save(&path)?;
    Ok(wallet)
}

/// Create a named wallet
pub fn create_named_wallet(name: &str) -> Result<Wallet, ChainError> {
    ensure_wallet_dir()?;
    let path = get_named_wallet_path(name)?;
    if path.exists() {
        return Err(ChainError::WalletError(format!(
            "Wallet '{}' already exists",
            name
        )));
    }
    let password = prompt_password("Enter a password for your new wallet: ")
        .map_err(|e| ChainError::WalletError(format!("Failed to read password: {}", e)))?;
    let wallet = Wallet::new(Some(name.to_string()))?;
    let encrypted_wallet = EncryptedWallet::from_wallet(&wallet, &password)?;
    encrypted_wallet.save(&path)?;
    Ok(wallet)
}

/// Load the default wallet
pub fn load_default_wallet() -> Result<Wallet, ChainError> {
    let path = get_default_wallet_path()?;
    if !path.exists() {
        return Err(ChainError::WalletError(
            "No wallet found. Run 'trinity-wallet new' first.".to_string(),
        ));
    }
    let encrypted_wallet = EncryptedWallet::load(&path)?;
    let password = prompt_password("Enter your wallet password: ")
        .map_err(|e| ChainError::WalletError(format!("Failed to read password: {}", e)))?;
    encrypted_wallet.decrypt(&password)
}

/// Load a named wallet
pub fn load_named_wallet(name: &str) -> Result<Wallet, ChainError> {
    let path = get_named_wallet_path(name)?;
    if !path.exists() {
        return Err(ChainError::WalletError(format!(
            "Wallet '{}' not found",
            name
        )));
    }
    let encrypted_wallet = EncryptedWallet::load(&path)?;
    let password = prompt_password("Enter your wallet password: ")
        .map_err(|e| ChainError::WalletError(format!("Failed to read password: {}", e)))?;
    encrypted_wallet.decrypt(&password)
}

/// List all available wallets in the wallet directory
pub fn list_wallets() -> Result<Vec<String>, ChainError> {
    let wallet_dir = get_wallet_dir()?;
    if !wallet_dir.exists() {
        return Ok(Vec::new());
    }
    let mut wallets = Vec::new();
    let entries = fs::read_dir(&wallet_dir)
        .map_err(|e| ChainError::WalletError(format!("Failed to read wallet directory: {}", e)))?;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read directory entry: {}", e);
                continue;
            }
        };
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                wallets.push(filename.to_string());
            }
        }
    }
    wallets.sort();
    Ok(wallets)
}

// ============================================================================
// Wallet Encryption/Decryption
// ============================================================================

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, Params, Version,
};

/// Encrypted wallet structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedWallet {
    pub name: Option<String>,
    pub address: String,
    pub encrypted_secret_key: String, // Base64 encoded encrypted data
    pub password_hash: String,        // Argon2 password hash
    pub nonce: String,                // Base64 encoded nonce
    pub created: String,
}

impl EncryptedWallet {
    /// Encrypt a wallet with a password using secure Argon2 parameters.
    pub fn from_wallet(wallet: &Wallet, password: &str) -> Result<Self, ChainError> {
        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);

        // Configure Argon2 with recommended parameters
        let params = Params::new(19456, 2, 1, None)
            .map_err(|e| ChainError::CryptoError(format!("Argon2 params error: {}", e)))?;
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

        // Hash the password
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ChainError::CryptoError(format!("Password hashing failed: {}", e)))?
            .to_string();

        // Derive encryption key from the raw hash
        let hash_bytes = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ChainError::CryptoError(format!("Password hashing failed: {}", e)))?
            .hash
            .ok_or_else(|| ChainError::CryptoError("No hash generated".to_string()))?
            .as_bytes()
            .to_vec();

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&hash_bytes[..32])
            .map_err(|e| ChainError::CryptoError(format!("Failed to create cipher: {}", e)))?;

        // Generate a random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the secret key
        let ciphertext = cipher
            .encrypt(&nonce, wallet.secret_key_hex.as_bytes())
            .map_err(|e| ChainError::CryptoError(format!("Encryption failed: {}", e)))?;

        use base64::{engine::general_purpose, Engine as _};

        Ok(EncryptedWallet {
            name: wallet.name.clone(),
            address: wallet.address.clone(),
            encrypted_secret_key: general_purpose::STANDARD.encode(&ciphertext),
            password_hash,
            nonce: general_purpose::STANDARD.encode(nonce),
            created: wallet.created.clone(),
        })
    }

    /// Decrypt the wallet using a password.
    pub fn decrypt(&self, password: &str) -> Result<Wallet, ChainError> {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};

        // Verify password against the stored hash
        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| ChainError::CryptoError(format!("Invalid password hash: {}", e)))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| {
                ChainError::CryptoError("Decryption failed - wrong password?".to_string())
            })?;

        // Re-derive the key for decryption (must use same params)
        let salt = parsed_hash.salt.ok_or(ChainError::CryptoError(
            "Salt not found in password hash".to_string(),
        ))?;
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            Version::V0x13,
            Params::new(19456, 2, 1, None)
                .map_err(|e| ChainError::CryptoError(format!("Argon2 params error: {}", e)))?,
        );
        let hash_bytes = argon2
            .hash_password(password.as_bytes(), salt)
            .map_err(|e| ChainError::CryptoError(format!("Password hashing failed: {}", e)))?
            .hash
            .ok_or_else(|| ChainError::CryptoError("No hash generated".to_string()))?
            .as_bytes()
            .to_vec();

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&hash_bytes[..32])
            .map_err(|e| ChainError::CryptoError(format!("Failed to create cipher: {}", e)))?;

        // Decode nonce and ciphertext
        use base64::{engine::general_purpose, Engine as _};

        let nonce_bytes = general_purpose::STANDARD
            .decode(&self.nonce)
            .map_err(|e| ChainError::CryptoError(format!("Invalid nonce: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = general_purpose::STANDARD
            .decode(&self.encrypted_secret_key)
            .map_err(|e| ChainError::CryptoError(format!("Invalid ciphertext: {}", e)))?;

        // Decrypt
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
            ChainError::CryptoError("Decryption failed - wrong password?".to_string())
        })?;

        let secret_key_hex = String::from_utf8(plaintext)
            .map_err(|e| ChainError::CryptoError(format!("Invalid UTF-8: {}", e)))?;

        Ok(Wallet {
            name: self.name.clone(),
            address: self.address.clone(),
            secret_key_hex,
            created: self.created.clone(),
        })
    }

    /// Save encrypted wallet to file
    pub fn save(&self, path: &PathBuf) -> Result<(), ChainError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ChainError::WalletError(format!("Failed to serialize wallet: {}", e)))?;

        // Create the file in a temporary location
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| ChainError::WalletError(format!("Failed to create temp file: {}", e)))?;

        // Write the wallet data
        write!(temp_file, "{}", json)
            .map_err(|e| ChainError::WalletError(format!("Failed to write to temp file: {}", e)))?;

        // Set file permissions before persisting
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = temp_file
                .as_file()
                .metadata()
                .map_err(|e| {
                    ChainError::WalletError(format!("Failed to get file metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(temp_file.path(), perms).map_err(|e| {
                ChainError::WalletError(format!("Failed to set file permissions: {}", e))
            })?;
        }

        // Atomically move the file to the final destination
        temp_file.persist(path).map_err(|e| {
            ChainError::WalletError(format!("Failed to persist wallet file: {}", e))
        })?;

        Ok(())
    }

    /// Load encrypted wallet from file
    pub fn load(path: &PathBuf) -> Result<Self, ChainError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ChainError::WalletError(format!("Failed to read wallet: {}", e)))?;

        let wallet: EncryptedWallet = serde_json::from_str(&contents).map_err(|e| {
            ChainError::WalletError(format!("Failed to parse encrypted wallet: {}", e))
        })?;

        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wallet_creation_and_keypair_recovery() {
        let wallet_result = Wallet::new(Some("test".to_string()));
        assert!(wallet_result.is_ok());
        let wallet = wallet_result.unwrap();

        assert_eq!(wallet.name, Some("test".to_string()));
        assert!(!wallet.address.is_empty());
        assert!(!wallet.secret_key_hex.is_empty());

        let keypair_result = wallet.get_keypair();
        assert!(keypair_result.is_ok());
        let keypair = keypair_result.unwrap();
        assert_eq!(wallet.address, hex::encode(keypair.address()));
    }

    #[test]
    fn test_encrypted_wallet_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let wallet_path = temp_dir.path().join("encrypted_wallet.json");

        let password = "strong_password";
        let wallet = Wallet::new(Some("encrypted_test".to_string())).unwrap();

        let encrypted_wallet = EncryptedWallet::from_wallet(&wallet, password).unwrap();
        let save_result = encrypted_wallet.save(&wallet_path);
        assert!(save_result.is_ok());

        let loaded_encrypted = EncryptedWallet::load(&wallet_path).unwrap();
        let decrypted_wallet_result = loaded_encrypted.decrypt(password);
        assert!(decrypted_wallet_result.is_ok());
        let decrypted_wallet = decrypted_wallet_result.unwrap();

        assert_eq!(wallet.address, decrypted_wallet.address);
        assert_eq!(wallet.secret_key_hex, decrypted_wallet.secret_key_hex);
    }

    #[test]
    fn test_atomic_save() {
        let temp_dir = tempdir().unwrap();
        let wallet_path = temp_dir.path().join("atomic_save_test.json");

        let wallet = Wallet::new(Some("atomic".to_string())).unwrap();
        let encrypted_wallet = EncryptedWallet::from_wallet(&wallet, "password").unwrap();

        let save_result = encrypted_wallet.save(&wallet_path);
        assert!(save_result.is_ok());

        let loaded_wallet = EncryptedWallet::load(&wallet_path);
        assert!(loaded_wallet.is_ok());
    }

    #[test]
    fn test_wrong_password_fails() {
        let temp_dir = tempdir().unwrap();
        let wallet_path = temp_dir.path().join("wrong_password_test.json");

        let wallet = Wallet::new(None).unwrap();
        let encrypted_wallet = EncryptedWallet::from_wallet(&wallet, "correct_password").unwrap();
        encrypted_wallet.save(&wallet_path).unwrap();

        let loaded_wallet = EncryptedWallet::load(&wallet_path).unwrap();
        let decrypt_result = loaded_wallet.decrypt("wrong_password");
        assert!(decrypt_result.is_err());
    }
}
