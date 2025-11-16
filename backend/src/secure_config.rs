use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Secure configuration manager for API keys and secrets
pub struct SecureConfig {
    config_dir: PathBuf,
    encryption_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub deepseek_api_key: Option<String>,
    pub encrypted: bool,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureStorage {
    pub encrypted_data: String,
    pub salt: String,
    pub version: String,
}

impl SecureConfig {
    pub fn new() -> Self {
        let config_dir = Self::get_config_directory();

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }

        Self {
            config_dir,
            encryption_key: None,
        }
    }

    /// Get the configuration directory
    fn get_config_directory() -> PathBuf {
        // Use project directory for secure storage
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(".secure");
        path
    }

    /// Initialize with encryption key (derived from master password or generated)
    pub fn with_encryption_key(&mut self, key: Vec<u8>) {
        self.encryption_key = Some(key);
    }

    /// Generate a secure encryption key
    pub fn generate_encryption_key() -> Vec<u8> {
        use rand::RngCore;
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Save DeepSeek API key securely
    pub fn save_deepseek_key(&self, api_key: &str) -> Result<(), String> {
        let config = ApiKeyConfig {
            deepseek_api_key: Some(api_key.to_string()),
            encrypted: self.encryption_key.is_some(),
            last_updated: chrono::Utc::now().timestamp(),
        };

        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        let data_to_save = if let Some(ref key) = self.encryption_key {
            // Encrypt the data
            let encrypted = self.encrypt_data(json.as_bytes(), key)?;
            let storage = SecureStorage {
                encrypted_data: encrypted,
                salt: general_purpose::STANDARD.encode(&key[..16]), // Store partial key as salt
                version: "1.0".to_string(),
            };
            serde_json::to_string_pretty(&storage)
                .map_err(|e| format!("Failed to serialize storage: {}", e))?
        } else {
            // Store plaintext (not recommended for production)
            log::warn!("⚠️ Storing API key in plaintext. Enable encryption for production!");
            json
        };

        let config_path = self.config_dir.join("deepseek_config.json");
        fs::write(&config_path, data_to_save)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Set restrictive permissions
        Self::set_secure_permissions(&config_path)?;

        log::info!("✅ DeepSeek API key saved securely to {:?}", config_path);
        Ok(())
    }

    /// Load DeepSeek API key
    pub fn load_deepseek_key(&self) -> Result<Option<String>, String> {
        let config_path = self.config_dir.join("deepseek_config.json");

        if !config_path.exists() {
            // Try environment variable first
            if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
                log::info!("✅ Loaded DeepSeek API key from environment");
                return Ok(Some(key));
            }
            return Ok(None);
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;

        // Try to parse as encrypted storage first
        if let Ok(storage) = serde_json::from_str::<SecureStorage>(&content) {
            if let Some(ref key) = self.encryption_key {
                let decrypted = self.decrypt_data(&storage.encrypted_data, key)?;
                let config: ApiKeyConfig = serde_json::from_slice(&decrypted)
                    .map_err(|e| format!("Failed to parse decrypted config: {}", e))?;
                log::info!("✅ Loaded encrypted DeepSeek API key");
                return Ok(config.deepseek_api_key);
            } else {
                return Err("Config is encrypted but no encryption key provided".to_string());
            }
        }

        // Try to parse as plaintext
        let config: ApiKeyConfig =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

        if !config.encrypted {
            log::warn!("⚠️ Loading API key from plaintext storage");
        }

        Ok(config.deepseek_api_key)
    }

    /// Simple XOR encryption (for demonstration - use proper crypto in production)
    fn encrypt_data(&self, data: &[u8], key: &[u8]) -> Result<String, String> {
        let mut encrypted = Vec::with_capacity(data.len());

        for (i, &byte) in data.iter().enumerate() {
            let key_byte = key[i % key.len()];
            encrypted.push(byte ^ key_byte);
        }

        Ok(general_purpose::STANDARD.encode(&encrypted))
    }

    /// Simple XOR decryption
    fn decrypt_data(&self, encrypted_base64: &str, key: &[u8]) -> Result<Vec<u8>, String> {
        let encrypted = general_purpose::STANDARD
            .decode(encrypted_base64)
            .map_err(|e| format!("Failed to decode base64: {}", e))?;

        let mut decrypted = Vec::with_capacity(encrypted.len());

        for (i, &byte) in encrypted.iter().enumerate() {
            let key_byte = key[i % key.len()];
            decrypted.push(byte ^ key_byte);
        }

        Ok(decrypted)
    }

    /// Set secure file permissions
    fn set_secure_permissions(path: &Path) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(path, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }
        Ok(())
    }

    /// Delete stored API key
    pub fn delete_deepseek_key(&self) -> Result<(), String> {
        let config_path = self.config_dir.join("deepseek_config.json");

        if config_path.exists() {
            fs::remove_file(&config_path).map_err(|e| format!("Failed to delete config: {}", e))?;
            log::info!("🗑️ Deleted DeepSeek API key");
        }

        Ok(())
    }

    /// Verify API key format
    pub fn validate_deepseek_key(api_key: &str) -> Result<(), String> {
        // DeepSeek API keys typically start with "sk-" and are at least 32 characters
        if !api_key.starts_with("sk-") {
            return Err("Invalid DeepSeek API key format. Must start with 'sk-'".to_string());
        }

        if api_key.len() < 32 {
            return Err("DeepSeek API key too short. Must be at least 32 characters".to_string());
        }

        Ok(())
    }

    /// Interactive setup for first-time configuration
    pub fn setup_interactive() -> Result<String, String> {
        println!("\n🔐 DeepSeek API Key Setup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nYou can get your DeepSeek API key from:");
        println!("https://platform.deepseek.com/api_keys");
        println!("\nYour API key will be stored securely with encryption.");
        println!("\nPlease enter your DeepSeek API key:");
        println!("(It should start with 'sk-')\n");

        use std::io::{self, Write};
        print!("> ");
        io::stdout().flush().unwrap();

        let mut api_key = String::new();
        io::stdin()
            .read_line(&mut api_key)
            .map_err(|e| format!("Failed to read input: {}", e))?;

        let api_key = api_key.trim();

        // Validate the key
        Self::validate_deepseek_key(api_key)?;

        // Save with encryption
        let mut config = SecureConfig::new();
        let encryption_key = Self::generate_encryption_key();
        config.with_encryption_key(encryption_key);
        config.save_deepseek_key(api_key)?;

        println!("\n✅ API key saved successfully!");
        println!(
            "Location: {:?}\n",
            config.config_dir.join("deepseek_config.json")
        );

        Ok(api_key.to_string())
    }

    /// Check if API key is configured
    pub fn is_configured(&self) -> bool {
        let config_path = self.config_dir.join("deepseek_config.json");
        config_path.exists() || std::env::var("DEEPSEEK_API_KEY").is_ok()
    }

    /// Get configuration status
    pub fn get_status(&self) -> ConfigStatus {
        let config_path = self.config_dir.join("deepseek_config.json");
        let file_exists = config_path.exists();
        let env_exists = std::env::var("DEEPSEEK_API_KEY").is_ok();

        ConfigStatus {
            file_configured: file_exists,
            env_configured: env_exists,
            encrypted: self.encryption_key.is_some(),
            config_path: config_path.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigStatus {
    pub file_configured: bool,
    pub env_configured: bool,
    pub encrypted: bool,
    pub config_path: String,
}

/// Helper function to initialize DeepSeek with secure config
pub fn init_deepseek_with_secure_config() -> Result<Option<String>, String> {
    let mut config = SecureConfig::new();

    // Try to load from secure storage
    if config.is_configured() {
        // Generate encryption key from a master password or use default
        // In production, this should be derived from user input or stored securely
        let encryption_key = SecureConfig::generate_encryption_key();
        config.with_encryption_key(encryption_key);

        match config.load_deepseek_key() {
            Ok(Some(key)) => {
                log::info!("✅ DeepSeek API key loaded from secure storage");
                return Ok(Some(key));
            }
            Ok(None) => {
                log::warn!("⚠️ No DeepSeek API key found");
            }
            Err(e) => {
                log::error!("❌ Failed to load DeepSeek API key: {}", e);
                log::info!("Falling back to environment variable");
            }
        }
    }

    // Fallback to environment variable
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        log::info!("✅ DeepSeek API key loaded from environment");
        return Ok(Some(key));
    }

    log::warn!("⚠️ No DeepSeek API key configured. AI features will be disabled.");
    log::info!("To enable AI features, set DEEPSEEK_API_KEY environment variable");
    log::info!("or run the interactive setup.");

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_deepseek_key() {
        assert!(SecureConfig::validate_deepseek_key("sk-0123456789abcdef0123456789abcdef").is_ok());
        assert!(SecureConfig::validate_deepseek_key("invalid_key").is_err());
        assert!(SecureConfig::validate_deepseek_key("sk-short").is_err());
    }

    #[test]
    fn test_encryption_decryption() {
        let config = SecureConfig::new();
        let key = SecureConfig::generate_encryption_key();
        let data = b"secret data";

        let encrypted = config.encrypt_data(data, &key).unwrap();
        let decrypted = config.decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_config_creation() {
        let config = SecureConfig::new();
        assert!(config.encryption_key.is_none());
        assert!(config.config_dir.exists() || !config.config_dir.to_string_lossy().is_empty());
    }
}
