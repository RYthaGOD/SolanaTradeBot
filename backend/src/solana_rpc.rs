use solana_client::rpc_client::RpcClient;
use solana_client::client_error::ClientError;
use solana_sdk::signature::Signature;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;
use std::str::FromStr;
use anyhow::Result;

/// Enhanced Solana RPC client with fallback and simulation support
pub struct SolanaRpcClient {
    rpc_urls: Vec<String>,
    current_index: usize,
    simulation_mode: bool,
    commitment: CommitmentConfig,
    timeout: Duration,
}

impl SolanaRpcClient {
    /// Create new RPC client
    pub fn new(
        rpc_urls: Vec<String>,
        simulation_mode: bool,
        commitment: CommitmentConfig,
    ) -> Self {
        log::info!("🌐 Initializing Solana RPC client");
        log::info!("📝 RPC endpoints configured: {}", rpc_urls.len());
        log::info!("📝 Simulation mode: {}", simulation_mode);
        
        Self {
            rpc_urls,
            current_index: 0,
            simulation_mode,
            commitment,
            timeout: Duration::from_secs(30),
        }
    }
    
    /// Get RPC client with automatic fallback
    fn get_client(&mut self) -> Result<RpcClient, ClientError> {
        if self.simulation_mode {
            // In simulation mode, create a client but don't actually connect
            return Ok(RpcClient::new_with_timeout_and_commitment(
                self.rpc_urls[0].clone(),
                self.timeout,
                self.commitment,
            ));
        }
        
        let url = &self.rpc_urls[self.current_index];
        log::debug!("Connecting to RPC: {}", url);
        
        let client = RpcClient::new_with_timeout_and_commitment(
            url.clone(),
            self.timeout,
            self.commitment,
        );
        
        // Test connection with getHealth
        match client.get_health() {
            Ok(_) => {
                super::monitoring::RPC_REQUESTS_TOTAL.inc();
                Ok(client)
            }
            Err(e) => {
                super::monitoring::RPC_ERRORS_TOTAL.inc();
                log::warn!("❌ RPC connection failed: {}. Trying fallback...", e);
                
                // Try next endpoint
                self.current_index = (self.current_index + 1) % self.rpc_urls.len();
                
                if self.current_index == 0 {
                    // We've tried all endpoints
                    Err(ClientError::from(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "All RPC endpoints failed"
                    )))
                } else {
                    // Retry with next endpoint
                    self.get_client()
                }
            }
        }
    }
    
    /// Get SOL balance for an address
    pub async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64> {
        if self.simulation_mode {
            log::debug!("📝 Simulating balance check for {}", pubkey);
            return Ok(10_000_000_000); // 10 SOL simulated
        }
        
        let client = self.get_client()?;
        
        let balance = client.get_balance(pubkey)
            .map_err(|e| anyhow::anyhow!("Failed to get balance: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        log::debug!("💰 Balance for {}: {} lamports", pubkey, balance);
        
        Ok(balance)
    }
    
    /// Get latest blockhash
    pub async fn get_latest_blockhash(&mut self) -> Result<solana_sdk::hash::Hash> {
        if self.simulation_mode {
            log::debug!("📝 Simulating blockhash fetch");
            // Return a dummy hash for simulation
            return Ok(solana_sdk::hash::Hash::from_str(
                "11111111111111111111111111111111"
            ).unwrap_or_default());
        }
        
        let client = self.get_client()?;
        
        let blockhash = client.get_latest_blockhash()
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        log::debug!("🔗 Latest blockhash: {}", blockhash);
        
        Ok(blockhash)
    }
    
    /// Send and confirm transaction
    pub async fn send_and_confirm_transaction(
        &mut self,
        transaction: &Transaction,
    ) -> Result<Signature> {
        if self.simulation_mode {
            log::debug!("📝 Simulating transaction send");
            // Return a dummy signature for simulation
            return Ok(Signature::from_str(
                "5VVvQg5DdvDxAYSKLC3e3EJ3fy3Tn8nzqZP3J8KjCZx8qQEqJGN5qQEqJGN5qQEqJGN5qQEqJGN5qQEqJGN5qQEq"
            ).unwrap_or_default());
        }
        
        let client = self.get_client()?;
        
        log::info!("📤 Sending transaction...");
        
        let signature = client.send_and_confirm_transaction(transaction)
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        super::monitoring::TRADES_TOTAL.inc();
        
        log::info!("✅ Transaction confirmed: {}", signature);
        
        Ok(signature)
    }
    
    /// Simulate transaction before sending
    pub async fn simulate_transaction(
        &mut self,
        transaction: &Transaction,
    ) -> Result<SimulationResult> {
        if self.simulation_mode {
            log::debug!("📝 Simulating transaction simulation");
            return Ok(SimulationResult {
                success: true,
                logs: vec!["Simulated log 1".to_string(), "Simulated log 2".to_string()],
                units_consumed: Some(100_000),
                err: None,
            });
        }
        
        let client = self.get_client()?;
        
        log::debug!("🔍 Simulating transaction before send...");
        
        let result = client.simulate_transaction(transaction)
            .map_err(|e| anyhow::anyhow!("Failed to simulate transaction: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        
        let simulation_result = SimulationResult {
            success: result.value.err.is_none(),
            logs: result.value.logs.unwrap_or_default(),
            units_consumed: result.value.units_consumed,
            err: result.value.err.map(|e| format!("{:?}", e)),
        };
        
        if !simulation_result.success {
            log::warn!("⚠️ Transaction simulation failed: {:?}", simulation_result.err);
        } else {
            log::debug!("✅ Transaction simulation successful");
        }
        
        Ok(simulation_result)
    }
    
    /// Get token account balance
    pub async fn get_token_account_balance(
        &mut self,
        token_account: &Pubkey,
    ) -> Result<TokenBalance> {
        if self.simulation_mode {
            log::debug!("📝 Simulating token balance check");
            return Ok(TokenBalance {
                amount: "1000000".to_string(),
                decimals: 6,
                ui_amount: Some(1.0),
                ui_amount_string: "1.0".to_string(),
            });
        }
        
        let client = self.get_client()?;
        
        let balance = client.get_token_account_balance(token_account)
            .map_err(|e| anyhow::anyhow!("Failed to get token balance: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        
        Ok(TokenBalance {
            amount: balance.amount,
            decimals: balance.decimals,
            ui_amount: balance.ui_amount,
            ui_amount_string: balance.ui_amount_string,
        })
    }
    
    /// Get multiple accounts info
    pub async fn get_multiple_accounts(
        &mut self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<AccountInfo>>> {
        if self.simulation_mode {
            log::debug!("📝 Simulating multiple account fetch");
            return Ok(pubkeys.iter().map(|_| None).collect());
        }
        
        let client = self.get_client()?;
        
        let accounts = client.get_multiple_accounts(pubkeys)
            .map_err(|e| anyhow::anyhow!("Failed to get accounts: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        
        Ok(accounts.into_iter().map(|acc| {
            acc.map(|a| AccountInfo {
                lamports: a.lamports,
                owner: a.owner,
                data: a.data,
                executable: a.executable,
                rent_epoch: a.rent_epoch,
            })
        }).collect())
    }
    
    /// Check if RPC is healthy
    pub async fn health_check(&mut self) -> bool {
        if self.simulation_mode {
            log::debug!("📝 Simulating health check - OK");
            return true;
        }
        
        match self.get_client() {
            Ok(_) => {
                log::debug!("✅ RPC health check passed");
                true
            }
            Err(e) => {
                log::error!("❌ RPC health check failed: {}", e);
                false
            }
        }
    }
    
    /// Get current epoch info
    pub async fn get_epoch_info(&mut self) -> Result<EpochInfo> {
        if self.simulation_mode {
            log::debug!("📝 Simulating epoch info");
            return Ok(EpochInfo {
                epoch: 500,
                slot_index: 12345,
                slots_in_epoch: 432000,
                absolute_slot: 200000000,
                block_height: 150000000,
            });
        }
        
        let client = self.get_client()?;
        
        let info = client.get_epoch_info()
            .map_err(|e| anyhow::anyhow!("Failed to get epoch info: {}", e))?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        
        Ok(EpochInfo {
            epoch: info.epoch,
            slot_index: info.slot_index,
            slots_in_epoch: info.slots_in_epoch,
            absolute_slot: info.absolute_slot,
            block_height: 0, // Block height not directly available in RPC response
        })
    }
}

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub logs: Vec<String>,
    pub units_consumed: Option<u64>,
    pub err: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub amount: String,
    pub decimals: u8,
    pub ui_amount: Option<f64>,
    pub ui_amount_string: String,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub lamports: u64,
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub executable: bool,
    pub rent_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct EpochInfo {
    pub epoch: u64,
    pub slot_index: u64,
    pub slots_in_epoch: u64,
    pub absolute_slot: u64,
    pub block_height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simulated_balance() {
        let mut client = SolanaRpcClient::new(
            vec!["http://localhost:8899".to_string()],
            true,
            CommitmentConfig::confirmed(),
        );
        
        let pubkey = Keypair::new().pubkey();
        let balance = client.get_balance(&pubkey).await.unwrap();
        assert_eq!(balance, 10_000_000_000);
    }
    
    #[tokio::test]
    async fn test_health_check_simulated() {
        let mut client = SolanaRpcClient::new(
            vec!["http://localhost:8899".to_string()],
            true,
            CommitmentConfig::confirmed(),
        );
        
        assert!(client.health_check().await);
    }
}
