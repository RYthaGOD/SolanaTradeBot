use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use ring::aead::{Aad, BoundKey, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Secure key manager for handling Solana keypairs
pub struct KeyManager {
    rng: SystemRandom,
}

/// A simple nonce sequence that generates random nonces
struct RandomNonceSequence {
    rng: SystemRandom,
}

impl NonceSequence for RandomNonceSequence {
    fn advance(&mut self) -> Result<ring::aead::Nonce, Unspecified> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng.fill(&mut nonce_bytes)?;
        ring::aead::Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct EncryptedKeyData {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    salt: Vec<u8>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }
    
    /// Generate a new Solana keypair
    pub fn generate_keypair(&self) -> Keypair {
        log::info!("🔑 Generating new Solana keypair");
        Keypair::new()
    }
    
    /// Derive encryption key from password using Argon2
    fn derive_key(&self, password: &str, salt: &[u8]) -> Result<Vec<u8>> {
        use argon2::{Algorithm, Argon2, Params, Version};
        
        let params = Params::new(65536, 3, 4, Some(KEY_LEN))
            .map_err(|e| anyhow::anyhow!("Failed to create Argon2 params: {}", e))?;
        
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        
        let mut output = vec![0u8; KEY_LEN];
        argon2.hash_password_into(password.as_bytes(), salt, &mut output)
            .map_err(|e| anyhow::anyhow!("Failed to derive encryption key: {}", e))?;
        
        Ok(output)
    }
    
    /// Encrypt keypair and save to file
    pub fn save_encrypted_keypair(
        &self,
        keypair: &Keypair,
        path: &Path,
        password: &str,
    ) -> Result<()> {
        log::info!("💾 Saving encrypted keypair to {:?}", path);
        
        if password.is_empty() {
            anyhow::bail!("Password cannot be empty");
        }
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create wallet directory")?;
        }
        
        // Generate random salt
        let mut salt = [0u8; 16];
        self.rng.fill(&mut salt)
            .map_err(|_| anyhow::anyhow!("Failed to generate salt"))?;
        
        // Derive encryption key from password
        let key_bytes = self.derive_key(password, &salt)?;
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|_| anyhow::anyhow!("Failed to generate nonce"))?;
        
        // Prepare keypair data for encryption
        let keypair_bytes = keypair.to_bytes();
        let mut in_out = keypair_bytes.to_vec();
        
        // Create sealing key and encrypt
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
            .map_err(|_| anyhow::anyhow!("Failed to create encryption key"))?;
        
        let mut sealing_key = SealingKey::new(unbound_key, RandomNonceSequence { rng: SystemRandom::new() });
        
        sealing_key.seal_in_place_append_tag(Aad::empty(), &mut in_out)
            .map_err(|_| anyhow::anyhow!("Failed to encrypt keypair"))?;
        
        // Save encrypted data
        let encrypted_data = EncryptedKeyData {
            nonce: nonce_bytes.to_vec(),
            ciphertext: in_out,
            salt: salt.to_vec(),
        };
        
        let json = serde_json::to_string_pretty(&encrypted_data)
            .context("Failed to serialize encrypted data")?;
        
        fs::write(path, json)
            .context("Failed to write encrypted keypair to file")?;
        
        log::info!("✅ Keypair encrypted and saved successfully");
        Ok(())
    }
    
    /// Load and decrypt keypair from file
    pub fn load_encrypted_keypair(
        &self,
        path: &Path,
        password: &str,
    ) -> Result<Keypair> {
        log::info!("🔓 Loading encrypted keypair from {:?}", path);
        
        if password.is_empty() {
            anyhow::bail!("Password cannot be empty");
        }
        
        // Read encrypted data
        let json = fs::read_to_string(path)
            .context("Failed to read encrypted keypair file")?;
        
        let encrypted_data: EncryptedKeyData = serde_json::from_str(&json)
            .context("Failed to parse encrypted keypair data")?;
        
        // Derive decryption key
        let key_bytes = self.derive_key(password, &encrypted_data.salt)?;
        
        // Prepare data for decryption
        let mut in_out = encrypted_data.ciphertext;
        
        // Create opening key and decrypt
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
            .map_err(|_| anyhow::anyhow!("Failed to create decryption key"))?;
        
        let mut opening_key = OpeningKey::new(unbound_key, RandomNonceSequence { rng: SystemRandom::new() });
        
        let decrypted = opening_key.open_in_place(Aad::empty(), &mut in_out)
            .map_err(|_| anyhow::anyhow!("Failed to decrypt keypair - wrong password?"))?;
        
        // Create keypair from decrypted bytes
        if decrypted.len() != 64 {
            anyhow::bail!("Invalid keypair data length");
        }
        
        let mut keypair_bytes = [0u8; 64];
        keypair_bytes.copy_from_slice(decrypted);
        
        let keypair = Keypair::from_bytes(&keypair_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to create keypair: {}", e))?;
        
        log::info!("✅ Keypair decrypted successfully. Public key: {}", keypair.pubkey());
        Ok(keypair)
    }
    
    /// Load keypair from file (supports both encrypted and plain JSON formats)
    pub fn load_or_create_keypair(
        &self,
        path: &Path,
        password: &str,
    ) -> Result<Keypair> {
        if path.exists() {
            self.load_encrypted_keypair(path, password)
        } else {
            log::warn!("⚠️ No existing keypair found. Generating new keypair.");
            log::warn!("⚠️ This keypair will be used for simulated trading only.");
            let keypair = self.generate_keypair();
            
            if !password.is_empty() {
                self.save_encrypted_keypair(&keypair, path, password)?;
            }
            
            Ok(keypair)
        }
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_keypair_encryption_decryption() {
        let key_manager = KeyManager::new();
        let keypair = key_manager.generate_keypair();
        let original_pubkey = keypair.pubkey();
        
        let temp_path = Path::new("/tmp/test_keypair.json");
        let password = "test_password_123";
        
        // Save encrypted
        key_manager.save_encrypted_keypair(&keypair, temp_path, password)
            .expect("Failed to save keypair");
        
        // Load and decrypt
        let loaded_keypair = key_manager.load_encrypted_keypair(temp_path, password)
            .expect("Failed to load keypair");
        
        assert_eq!(original_pubkey, loaded_keypair.pubkey());
        
        // Cleanup
        fs::remove_file(temp_path).ok();
    }
    
    #[test]
    fn test_wrong_password() {
        let key_manager = KeyManager::new();
        let keypair = key_manager.generate_keypair();
        
        let temp_path = Path::new("/tmp/test_keypair_wrong_pw.json");
        let password = "correct_password";
        
        key_manager.save_encrypted_keypair(&keypair, temp_path, password)
            .expect("Failed to save keypair");
        
        // Try to load with wrong password
        let result = key_manager.load_encrypted_keypair(temp_path, "wrong_password");
        assert!(result.is_err());
        
        // Cleanup
        fs::remove_file(temp_path).ok();
    }
}
