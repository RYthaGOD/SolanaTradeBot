use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
// base64 encoding/decoding handled via base58 crate

/// Secure key manager for wallet private keys
pub struct KeyManager {
    encryption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub address: String,
    pub encrypted_key: String,
    pub key_type: KeyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyType {
    Base58,
    HexEncoded,
    JsonKeypair,
}

impl KeyManager {
    pub fn new(encryption_enabled: bool) -> Self {
        Self { encryption_enabled }
    }

    /// Load wallet from environment variable
    pub fn load_from_env(&self, env_var: &str) -> Result<String, String> {
        std::env::var(env_var)
            .map_err(|_| format!("Environment variable {} not found", env_var))
    }

    /// Load wallet from file (JSON format)
    pub fn load_from_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        if !path.exists() {
            return Err(format!("Wallet file not found: {:?}", path));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read wallet file: {}", e))?;

        // Parse JSON array of bytes
        let key_bytes: Vec<u8> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse wallet JSON: {}", e))?;

        if key_bytes.len() != 64 {
            return Err(format!("Invalid key length: {} (expected 64)", key_bytes.len()));
        }

        log::info!("✅ Loaded wallet from file: {:?}", path);
        Ok(key_bytes)
    }

    /// Save wallet to file securely
    pub fn save_to_file(&self, key_bytes: &[u8], path: &Path) -> Result<(), String> {
        if key_bytes.len() != 64 {
            return Err(format!("Invalid key length: {}", key_bytes.len()));
        }

        let json = serde_json::to_string_pretty(&key_bytes)
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

    /// Validate a base58 private key
    pub fn validate_base58_key(&self, key: &str) -> Result<(), String> {
        if key.len() < 32 || key.len() > 88 {
            return Err("Invalid key length".to_string());
        }

        // Check if it's valid base58
        bs58::decode(key)
            .into_vec()
            .map_err(|_| "Invalid base58 encoding".to_string())?;

        Ok(())
    }

    /// AES-256-GCM encryption for secure data protection
    /// Uses authenticated encryption with associated data (AEAD)
    pub fn obfuscate(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
        if !self.encryption_enabled {
            return Ok(data.to_vec());
        }

        // Validate key length (must be 32 bytes for AES-256)
        if key.len() != 32 {
            return Err(format!("Invalid key length: {} (expected 32 bytes for AES-256)", key.len()));
        }

        // Create cipher from key
        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Generate random nonce (12 bytes for GCM)
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the data
        let ciphertext = cipher.encrypt(&nonce, data)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext (nonce is 12 bytes, we'll base64 encode the whole thing)
        let mut encrypted_data = Vec::with_capacity(nonce.len() + ciphertext.len());
        encrypted_data.extend_from_slice(nonce.as_slice());
        encrypted_data.extend_from_slice(&ciphertext);

        Ok(encrypted_data)
    }

    /// AES-256-GCM decryption
    pub fn deobfuscate(&self, encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
        if !self.encryption_enabled {
            return Ok(encrypted_data.to_vec());
        }

        // Validate key length
        if key.len() != 32 {
            return Err(format!("Invalid key length: {} (expected 32 bytes for AES-256)", key.len()));
        }

        // Nonce is 12 bytes, must have at least that much data
        if encrypted_data.len() < 12 {
            return Err("Encrypted data too short (missing nonce)".to_string());
        }

        // Extract nonce (first 12 bytes) and ciphertext (rest)
        let nonce_bytes = &encrypted_data[..12];
        let ciphertext = &encrypted_data[12..];

        // Create nonce from bytes
        let nonce = Nonce::from_slice(nonce_bytes);

        // Create cipher from key
        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Decrypt the data
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// Generate a random encryption key
    pub fn generate_encryption_key() -> [u8; 32] {
        use rand::Rng;
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        key
    }

    /// Derive wallet address from public key bytes
    pub fn derive_address(public_key: &[u8]) -> String {
        bs58::encode(public_key).into_string()
    }

    /// Check if a wallet file exists
    pub fn wallet_exists(path: &Path) -> bool {
        path.exists() && path.is_file()
    }

    /// Get default wallet path
    pub fn default_wallet_path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config/solana/id.json", home)
    }
}

/// Wallet manager for multiple wallets
pub struct WalletManager {
    wallets: std::collections::HashMap<String, WalletConfig>,
}

impl WalletManager {
    pub fn new() -> Self {
        Self {
            wallets: std::collections::HashMap::new(),
        }
    }

    /// Add a wallet to the manager
    pub fn add_wallet(&mut self, name: String, config: WalletConfig) {
        log::info!("Added wallet: {}", name);
        self.wallets.insert(name, config);
    }

    /// Get a wallet by name
    pub fn get_wallet(&self, name: &str) -> Option<&WalletConfig> {
        self.wallets.get(name)
    }

    /// List all wallet names
    pub fn list_wallets(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    /// Remove a wallet
    pub fn remove_wallet(&mut self, name: &str) -> bool {
        self.wallets.remove(name).is_some()
    }

    /// Get the default wallet (first one added or named "default")
    pub fn get_default_wallet(&self) -> Option<&WalletConfig> {
        self.wallets.get("default")
            .or_else(|| self.wallets.values().next())
    }
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_key_manager_creation() {
        let km = KeyManager::new(true);
        assert!(km.encryption_enabled);
    }

    #[test]
    fn test_obfuscation() {
        let km = KeyManager::new(true);
        let data = b"secret data";
        let key = &KeyManager::generate_encryption_key(); // Use proper 32-byte key

        let encrypted = km.obfuscate(data, key).expect("Encryption should succeed");
        let decrypted = km.deobfuscate(&encrypted, key).expect("Decryption should succeed");

        assert_eq!(data, &decrypted[..]);
        assert_ne!(data, &encrypted[..]);
        
        // Verify that same plaintext produces different ciphertext (nonce is random)
        let encrypted2 = km.obfuscate(data, key).expect("Second encryption should succeed");
        assert_ne!(encrypted, encrypted2); // Different nonces = different ciphertexts
    }

    #[test]
    fn test_validate_base58_key() {
        let km = KeyManager::new(false);
        
        // Valid base58 string
        let valid_key = "5J3mBbAH58CpQ3Y5RNJpUKPE62SQ5tfcvU2JpbnkeyhfsYB1Jcn";
        assert!(km.validate_base58_key(valid_key).is_ok());

        // Invalid base58 (contains invalid characters)
        let invalid_key = "invalid-key-with-dashes";
        assert!(km.validate_base58_key(invalid_key).is_err());
    }

    #[test]
    fn test_wallet_manager() {
        let mut manager = WalletManager::new();
        
        let config = WalletConfig {
            address: "test_address".to_string(),
            encrypted_key: "test_key".to_string(),
            key_type: KeyType::Base58,
        };

        manager.add_wallet("test".to_string(), config.clone());
        
        assert_eq!(manager.list_wallets().len(), 1);
        assert!(manager.get_wallet("test").is_some());
        
        assert!(manager.remove_wallet("test"));
        assert_eq!(manager.list_wallets().len(), 0);
    }

    #[test]
    fn test_encryption_key_generation() {
        let key1 = KeyManager::generate_encryption_key();
        let key2 = KeyManager::generate_encryption_key();
        
        // Keys should be different
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_address() {
        let public_key = vec![1u8; 32];
        let address = KeyManager::derive_address(&public_key);
        
        // Should return a base58 encoded string
        assert!(!address.is_empty());
    }
}
