use solana_sdk::{
    signature::{Keypair, Signer, Signature},
    pubkey::Pubkey,
    transaction::Transaction,
    system_instruction,
    native_token::LAMPORTS_PER_SOL,
    message::Message,
    hash::Hash,
    instruction::Instruction,
};
use solana_client::rpc_client::RpcClient;
use std::path::Path;
use std::fs;
use std::sync::Arc;

/// Real Solana Wallet SDK implementation
/// Provides full wallet functionality using official Solana SDK
pub struct Wallet {
    keypair: Keypair,
    rpc_client: Option<Arc<RpcClient>>,
}

impl Wallet {
    /// Create a new wallet with a generated keypair
    pub fn new() -> Self {
        let keypair = Keypair::new();
        log::info!("🔑 Generated new wallet: {}", keypair.pubkey());
        Self {
            keypair,
            rpc_client: None,
        }
    }

    /// Create a new wallet with RPC client connection
    pub fn new_with_rpc(rpc_url: String) -> Self {
        let keypair = Keypair::new();
        let rpc_client = Arc::new(RpcClient::new(rpc_url));
        log::info!("🔑 Generated new wallet with RPC: {}", keypair.pubkey());
        Self {
            keypair,
            rpc_client: Some(rpc_client),
        }
    }

    /// Set RPC client for this wallet
    pub fn set_rpc_client(&mut self, rpc_url: String) {
        self.rpc_client = Some(Arc::new(RpcClient::new(rpc_url)));
        log::info!("🌐 RPC client configured for wallet: {}", self.pubkey());
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
        Ok(Self {
            keypair,
            rpc_client: None,
        })
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
        Ok(Self {
            keypair,
            rpc_client: None,
        })
    }

    /// Load wallet from environment variable or generate new one
    pub fn from_env_or_new(env_var: &str) -> Self {
        match std::env::var(env_var) {
            Ok(key) => {
                match Self::from_base58(&key) {
                    Ok(mut wallet) => {
                        // Set RPC client if available
                        if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL") {
                            wallet.set_rpc_client(rpc_url);
                        }
                        wallet
                    }
                    Err(e) => {
                        log::warn!("Failed to load wallet from env: {}. Generating new wallet.", e);
                        let mut wallet = Self::new();
                        if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL") {
                            wallet.set_rpc_client(rpc_url);
                        }
                        wallet
                    }
                }
            }
            Err(_) => {
                log::info!("No wallet in environment variable {}. Generating new wallet.", env_var);
                let mut wallet = Self::new();
                if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL") {
                    wallet.set_rpc_client(rpc_url);
                }
                wallet
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

    // ========== Real Solana SDK Wallet Operations ==========

    /// Get wallet balance in SOL (requires RPC client)
    pub fn get_balance(&self) -> Result<f64, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        let lamports = rpc.get_balance(&self.pubkey())
            .map_err(|e| format!("Failed to get balance: {}", e))?;
        
        Ok(lamports as f64 / LAMPORTS_PER_SOL as f64)
    }

    /// Get wallet balance in lamports (requires RPC client)
    pub fn get_balance_lamports(&self) -> Result<u64, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        rpc.get_balance(&self.pubkey())
            .map_err(|e| format!("Failed to get balance: {}", e))
    }

    /// Build and sign a transaction
    pub fn build_and_sign_transaction(
        &self,
        instructions: Vec<Instruction>,
        recent_blockhash: Hash,
    ) -> Result<Transaction, String> {
        let message = Message::new(&instructions, Some(&self.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        
        transaction.sign(&[&self.keypair], recent_blockhash);
        
        Ok(transaction)
    }

    /// Send SOL to another address
    pub fn send_sol(&self, to: &Pubkey, amount_sol: f64) -> Result<Signature, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;

        let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
        
        // Create transfer instruction
        let instruction = system_instruction::transfer(&self.pubkey(), to, lamports);
        
        // Get recent blockhash
        let recent_blockhash = rpc.get_latest_blockhash()
            .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;
        
        // Build and sign transaction
        let transaction = self.build_and_sign_transaction(vec![instruction], recent_blockhash)?;
        
        // Send transaction and wait for confirmation
        // RpcClient::send_and_confirm_transaction is synchronous and takes only the transaction
        let signature = rpc.send_and_confirm_transaction(&transaction)
            .map_err(|e| format!("Failed to send transaction: {}", e))?;
        
        log::info!("✅ Sent {:.6} SOL to {} | TX: {}", amount_sol, to, signature);
        Ok(signature)
    }

    /// Send a signed transaction (for custom transactions)
    /// Note: Transaction must already be signed
    /// This is async but we'll use tokio::runtime for now
    pub fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        // Transaction should already be signed
        let signature = rpc.send_and_confirm_transaction(transaction)
            .map_err(|e| format!("Failed to send transaction: {}", e))?;
        
        log::info!("✅ Transaction sent: {}", signature);
        Ok(signature)
    }

    /// Request airdrop (devnet/testnet only)
    pub fn request_airdrop(&self, amount_sol: f64) -> Result<Signature, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;

        let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
        
        let signature = rpc.request_airdrop(&self.pubkey(), lamports)
            .map_err(|e| format!("Failed to request airdrop: {}", e))?;
        
        log::info!("💰 Airdrop requested: {:.6} SOL | TX: {}", amount_sol, signature);
        Ok(signature)
    }

    /// Get recent blockhash from RPC
    pub fn get_recent_blockhash(&self) -> Result<Hash, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        rpc.get_latest_blockhash()
            .map_err(|e| format!("Failed to get recent blockhash: {}", e))
    }

    /// Check if account exists on-chain
    pub fn account_exists(&self, pubkey: &Pubkey) -> Result<bool, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        match rpc.get_account(pubkey) {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's an account not found error
                let error_str = format!("{}", e);
                if error_str.contains("Invalid param") || error_str.contains("not found") {
                    Ok(false)
                } else {
                    Err(format!("Failed to check account: {}", e))
                }
            }
        }
    }

    /// Get minimum balance for rent exemption
    pub fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        rpc.get_minimum_balance_for_rent_exemption(data_len)
            .map_err(|e| format!("Failed to get rent exemption: {}", e))
    }

    /// Sign a message (for authentication/verification)
    pub fn sign_message(&self, message: &[u8]) -> Signature {
        // Use the Signer trait to sign a message
        // Note: Solana SDK signs message hashes, not raw messages
        let message_hash = solana_sdk::hash::hash(message);
        // Use try_sign_message from Signer trait
        self.keypair.try_sign_message(message_hash.as_ref())
            .unwrap_or_else(|_| {
                // Fallback: try signing the original message
                self.keypair.try_sign_message(message)
                    .unwrap_or_else(|_| Signature::default())
            })
    }

    /// Verify a signature
    pub fn verify_signature(&self, message: &[u8], signature: &Signature) -> bool {
        signature.verify(self.pubkey().as_ref(), message)
    }

    /// Get transaction history (recent transactions)
    pub fn get_transaction_history(&self, limit: usize) -> Result<Vec<Signature>, String> {
        let rpc = self.rpc_client.as_ref()
            .ok_or("RPC client not configured. Call set_rpc_client() first.")?;
        
        // Get signatures for this account
        // get_signatures_for_address returns Vec<RpcConfirmedTransactionStatusWithSignature>
        let signature_infos = rpc.get_signatures_for_address(&self.pubkey())
            .map_err(|e| format!("Failed to get transaction history: {}", e))?;
        
        // Extract signature strings and parse them
        let parsed_signatures: Result<Vec<Signature>, _> = signature_infos
            .into_iter()
            .take(limit)
            .map(|info| info.signature.parse())
            .collect();
        
        parsed_signatures.map_err(|e| format!("Failed to parse signature: {}", e))
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
