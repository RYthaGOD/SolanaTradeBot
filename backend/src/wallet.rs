use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
};
use std::path::Path;
use std::fs;

/// Wallet manager for Solana operations
pub struct Wallet {
    keypair: Keypair,
}

impl Wallet {
    /// Create a new wallet with a generated keypair
    pub fn new() -> Self {
        let keypair = Keypair::new();
        log::info!("🔑 Generated new wallet: {}", keypair.pubkey());
        Self { keypair }
    }

    /// Load wallet from base58 private key string
    pub fn from_base58(private_key: &str) -> Result<Self, String> {
        let decoded = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| format!("Failed to decode base58 key: {}", e))?;

        if decoded.len() != 64 {
            return Err(format!("Invalid key length: {} (expected 64)", decoded.len()));
        }

        let keypair = Keypair::from_bytes(&decoded)
            .map_err(|e| format!("Failed to create keypair: {}", e))?;

        log::info!("🔑 Loaded wallet from base58: {}", keypair.pubkey());
        Ok(Self { keypair })
    }

    /// Load wallet from JSON keypair file (Solana CLI format)
    pub fn from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!("Wallet file not found: {:?}", path));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read wallet file: {}", e))?;

        let key_bytes: Vec<u8> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse wallet JSON: {}", e))?;

        if key_bytes.len() != 64 {
            return Err(format!("Invalid key length: {} (expected 64)", key_bytes.len()));
        }

        let keypair = Keypair::from_bytes(&key_bytes)
            .map_err(|e| format!("Failed to create keypair: {}", e))?;

        log::info!("🔑 Loaded wallet from file {:?}: {}", path, keypair.pubkey());
        Ok(Self { keypair })
    }

    /// Load wallet from environment variable or generate new one
    pub fn from_env_or_new(env_var: &str) -> Self {
        match std::env::var(env_var) {
            Ok(key) => {
                match Self::from_base58(&key) {
                    Ok(wallet) => wallet,
                    Err(e) => {
                        log::warn!("Failed to load wallet from env: {}. Generating new wallet.", e);
                        Self::new()
                    }
                }
            }
            Err(_) => {
                log::info!("No wallet in environment variable {}. Generating new wallet.", env_var);
                Self::new()
            }
        }
    }

    /// Get the public key of this wallet
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Get the keypair reference
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Export wallet to base58 string (private key)
    pub fn to_base58(&self) -> String {
        bs58::encode(self.keypair.to_bytes()).into_string()
    }

    /// Save wallet to JSON file (Solana CLI format)
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let key_bytes = self.keypair.to_bytes();
        let json = serde_json::to_string_pretty(&key_bytes.to_vec())
            .map_err(|e| format!("Failed to serialize key: {}", e))?;

        fs::write(path, json)
            .map_err(|e| format!("Failed to write wallet file: {}", e))?;

        // Set restrictive permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(path, perms)
                .map_err(|e| format!("Failed to set file permissions: {}", e))?;
        }

        log::info!("💾 Saved wallet to file: {:?}", path);
        Ok(())
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_wallet() {
        let wallet = Wallet::new();
        assert_eq!(wallet.pubkey().to_bytes().len(), 32);
    }

    #[test]
    fn test_wallet_pubkey() {
        let wallet = Wallet::new();
        let pubkey = wallet.pubkey();
        assert_eq!(pubkey.to_bytes().len(), 32);
    }

    #[test]
    fn test_to_base58() {
        let wallet = Wallet::new();
        let base58 = wallet.to_base58();
        assert!(!base58.is_empty());
        
        // Should be able to load it back
        let wallet2 = Wallet::from_base58(&base58).unwrap();
        assert_eq!(wallet.pubkey(), wallet2.pubkey());
    }

    #[test]
    fn test_invalid_base58() {
        let result = Wallet::from_base58("invalid-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load_file() {
        let wallet = Wallet::new();
        let path = PathBuf::from("/tmp/test_wallet.json");
        
        wallet.save_to_file(&path).unwrap();
        let wallet2 = Wallet::from_file(&path).unwrap();
        
        assert_eq!(wallet.pubkey(), wallet2.pubkey());
        
        // Cleanup
        let _ = fs::remove_file(path);
    }
}
