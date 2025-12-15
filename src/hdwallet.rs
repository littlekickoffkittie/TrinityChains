use bip39::Mnemonic;
use rand::RngCore;

/// Lightweight HD wallet helpers using BIP-39 for mnemonic and seed derivation.
///
/// This module currently provides mnemonic generation and seed derivation
/// (BIP-39). It intentionally keeps the API small: callers can obtain the
/// mnemonic phrase and derive the 64-byte seed (PBKDF2-HMAC-SHA512) from it.
pub struct HDWallet;

impl HDWallet {
    /// Generate a new mnemonic phrase with the given word count.
    /// Supported `word_count` values: 12, 15, 18, 21, 24.
    pub fn generate_mnemonic(word_count: usize) -> Result<String, String> {
        let entropy_bytes = match word_count {
            12 => 16,
            15 => 20,
            18 => 24,
            21 => 28,
            24 => 32,
            _ => return Err("unsupported word count; choose 12/15/18/21/24".into()),
        };

        let mut entropy = vec![0u8; entropy_bytes];
        rand::thread_rng().fill_bytes(&mut entropy);

        let m = Mnemonic::from_entropy(&entropy)
            .map_err(|e| format!("mnemonic generation failed: {}", e))?;
        Ok(m.to_string())
    }

    /// Derive the BIP-39 seed bytes from a mnemonic phrase and optional passphrase.
    /// The returned `Vec<u8>` is the 64-byte seed produced by PBKDF2 as defined in BIP-39.
    pub fn seed_from_mnemonic(phrase: &str, passphrase: Option<&str>) -> Result<Vec<u8>, String> {
        let m = Mnemonic::parse_normalized(phrase)
            .map_err(|e| format!("invalid mnemonic phrase: {}", e))?;
        let pass = passphrase.unwrap_or("");
        Ok(m.to_seed_normalized(pass).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::HDWallet;

    #[test]
    fn test_generate_mnemonic_and_seed() {
        // Generate a 12-word mnemonic
        let m = HDWallet::generate_mnemonic(12).expect("mnemonic generation");
        assert!(!m.is_empty());

        // Derive seed from the generated mnemonic
        let seed =
            HDWallet::seed_from_mnemonic(&m, Some("my_passphrase")).expect("seed derivation");
        assert_eq!(seed.len(), 64); // BIP-39 seed is 64 bytes
    }

    #[test]
    fn test_invalid_word_count() {
        let r = HDWallet::generate_mnemonic(13);
        assert!(r.is_err());
    }
}
