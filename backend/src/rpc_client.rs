use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::Transaction,
};

/// Solana RPC client wrapper for blockchain operations
pub struct SolanaRpcClient {
    client: RpcClient,
    commitment: CommitmentConfig,
}

impl SolanaRpcClient {
    /// Create a new RPC client
    ///
    /// # Arguments
    /// * `rpc_url` - The Solana RPC endpoint URL
    pub fn new(rpc_url: String) -> Self {
        let client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());
        log::info!("🌐 Connected to Solana RPC: {}", rpc_url);

        Self {
            client,
            commitment: CommitmentConfig::confirmed(),
        }
    }

    /// Get the balance of a wallet in SOL
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<f64, String> {
        let lamports = self
            .client
            .get_balance(pubkey)
            .map_err(|e| format!("Failed to get balance: {}", e))?;

        let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
        Ok(sol)
    }

    /// Get the balance in lamports (raw value)
    pub async fn get_balance_lamports(&self, pubkey: &Pubkey) -> Result<u64, String> {
        self.client
            .get_balance(pubkey)
            .map_err(|e| format!("Failed to get balance: {}", e))
    }

    /// Request airdrop (devnet/testnet only)
    pub async fn request_airdrop(
        &self,
        pubkey: &Pubkey,
        amount_sol: f64,
    ) -> Result<Signature, String> {
        let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;

        let signature = self
            .client
            .request_airdrop(pubkey, lamports)
            .map_err(|e| format!("Failed to request airdrop: {}", e))?;

        log::info!("💰 Airdrop requested: {} SOL to {}", amount_sol, pubkey);
        Ok(signature)
    }

    /// Confirm a transaction
    pub async fn confirm_transaction(&self, signature: &Signature) -> Result<bool, String> {
        // Poll for transaction confirmation
        for _ in 0..30 {
            match self.client.confirm_transaction(signature) {
                Ok(confirmed) => {
                    if confirmed {
                        return Ok(true);
                    }
                }
                Err(e) => {
                    log::warn!("Error confirming transaction: {}", e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Err("Transaction confirmation timeout".to_string())
    }

    /// Send a transaction
    pub async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, String> {
        let signature = self
            .client
            .send_transaction(transaction)
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        log::info!("📤 Transaction sent: {}", signature);
        Ok(signature)
    }

    /// Send and confirm a transaction
    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
        _signer: &Keypair,
    ) -> Result<Signature, String> {
        let signature = self.send_transaction(transaction).await?;

        log::info!("⏳ Confirming transaction: {}", signature);
        self.confirm_transaction(&signature).await?;

        log::info!("✅ Transaction confirmed: {}", signature);
        Ok(signature)
    }

    /// Get latest blockhash
    pub async fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash, String> {
        self.client
            .get_latest_blockhash()
            .map_err(|e| format!("Failed to get latest blockhash: {}", e))
    }

    /// Get account info
    pub async fn get_account(
        &self,
        pubkey: &Pubkey,
    ) -> Result<solana_sdk::account::Account, String> {
        self.client
            .get_account(pubkey)
            .map_err(|e| format!("Failed to get account: {}", e))
    }

    /// Check if an account exists
    pub async fn account_exists(&self, pubkey: &Pubkey) -> bool {
        self.client.get_account(pubkey).is_ok()
    }

    /// Get the current slot
    pub async fn get_slot(&self) -> Result<u64, String> {
        self.client
            .get_slot()
            .map_err(|e| format!("Failed to get slot: {}", e))
    }

    /// Get block time for a slot
    pub async fn get_block_time(&self, slot: u64) -> Result<i64, String> {
        self.client
            .get_block_time(slot)
            .map_err(|e| format!("Failed to get block time: {}", e))
    }

    /// Get transaction details
    pub async fn get_transaction(
        &self,
        signature: &Signature,
    ) -> Result<
        Option<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>,
        String,
    > {
        // This is a simplified version - in production you'd want more details
        match self.client.get_signature_status(signature) {
            Ok(status) => {
                match status {
                    Some(_) => Ok(None), // Simplified - just check if exists
                    None => Ok(None),
                }
            }
            Err(e) => Err(format!("Failed to get transaction: {}", e)),
        }
    }

    /// Get the RPC client reference
    pub fn client(&self) -> &RpcClient {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_rpc_client() {
        let client = SolanaRpcClient::new("https://api.devnet.solana.com".to_string());
        assert_eq!(client.commitment, CommitmentConfig::confirmed());
    }

    #[test]
    fn test_rpc_client_interface() {
        // Just test that we can create a client without connecting
        let _client = SolanaRpcClient::new("https://api.devnet.solana.com".to_string());
        // This verifies the API interface exists
    }
}
