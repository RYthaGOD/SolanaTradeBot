use anyhow::Result;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use std::sync::Arc;
use tokio::sync::Mutex;

/// DEX executor for real swap operations on Solana
/// Integrates Jupiter Aggregator for optimal routing
pub struct DexExecutor {
    jupiter_client: Arc<Mutex<super::jupiter_integration::JupiterClient>>,
    rpc_client: Arc<Mutex<super::solana_rpc::SolanaRpcClient>>,
    keypair: Keypair,
    enable_real_trading: bool,
}

/// Swap result with transaction details
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub signature: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub input_token: String,
    pub output_token: String,
    pub price_impact: f64,
    pub success: bool,
    pub error: Option<String>,
}

impl DexExecutor {
    /// Create new DEX executor
    pub fn new(
        jupiter_client: Arc<Mutex<super::jupiter_integration::JupiterClient>>,
        rpc_client: Arc<Mutex<super::solana_rpc::SolanaRpcClient>>,
        keypair: Keypair,
        enable_real_trading: bool,
    ) -> Self {
        log::info!("🔷 Initializing DEX Executor");
        log::info!("📝 Real trading enabled: {}", enable_real_trading);
        
        Self {
            jupiter_client,
            rpc_client,
            keypair,
            enable_real_trading,
        }
    }
    
    /// Execute a swap on DEX via Jupiter
    /// 
    /// # Arguments
    /// * `input_mint` - Input token mint address
    /// * `output_mint` - Output token mint address
    /// * `amount` - Amount in smallest unit (lamports for SOL)
    /// * `slippage_bps` - Slippage tolerance in basis points (50 = 0.5%)
    /// 
    /// # Returns
    /// * `Result<SwapResult>` - Swap result with transaction signature
    pub async fn execute_swap(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<SwapResult> {
        log::info!("🔄 Executing swap: {} -> {} (amount: {})", input_mint, output_mint, amount);
        
        // Validate swap parameters
        self.validate_swap_params(input_mint, output_mint, amount, slippage_bps)?;
        
        // Get quote from Jupiter
        let quote = {
            let jupiter = self.jupiter_client.lock().await;
            jupiter.get_quote(super::jupiter_integration::JupiterQuoteRequest {
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                amount,
                slippage_bps,
            }).await?
        };
        
        log::info!("💹 Quote received: {} -> {} (impact: {:.2}%)", 
            quote.in_amount, quote.out_amount, quote.price_impact_pct);
        
        // Check if price impact is acceptable (>5% is risky)
        if quote.price_impact_pct > 5.0 {
            log::warn!("⚠️ High price impact: {:.2}%", quote.price_impact_pct);
            super::monitoring::SIGNALS_REJECTED.inc();
            
            return Ok(SwapResult {
                signature: String::new(),
                input_amount: amount,
                output_amount: quote.out_amount.parse().unwrap_or(0),
                input_token: input_mint.to_string(),
                output_token: output_mint.to_string(),
                price_impact: quote.price_impact_pct,
                success: false,
                error: Some(format!("Price impact too high: {:.2}%", quote.price_impact_pct)),
            });
        }
        
        // If real trading is disabled, return simulated result
        if !self.enable_real_trading {
            log::info!("📝 Simulated swap (real trading disabled)");
            return Ok(SwapResult {
                signature: "SIMULATED_SIGNATURE".to_string(),
                input_amount: amount,
                output_amount: quote.out_amount.parse().unwrap_or(0),
                input_token: input_mint.to_string(),
                output_token: output_mint.to_string(),
                price_impact: quote.price_impact_pct,
                success: true,
                error: None,
            });
        }
        
        // Get swap transaction from Jupiter
        let swap_response = {
            let jupiter = self.jupiter_client.lock().await;
            jupiter.execute_swap(super::jupiter_integration::SwapRequest {
                quote_response: quote.clone(),
                user_public_key: self.keypair.pubkey().to_string(),
                wrap_unwrap_sol: true,
                compute_unit_price_micro_lamports: Some(1000), // Priority fee
            }).await?
        };
        
        // Deserialize and sign transaction
        use base64::Engine;
        let transaction_bytes = base64::engine::general_purpose::STANDARD
            .decode(&swap_response.swap_transaction)?;
        let mut transaction: Transaction = bincode::deserialize(&transaction_bytes)?;
        
        // Simulate transaction first
        log::info!("🔍 Simulating transaction...");
        let mut rpc = self.rpc_client.lock().await;
        let simulation = rpc.simulate_transaction(&transaction).await?;
        
        if !simulation.success {
            log::error!("❌ Transaction simulation failed: {:?}", simulation.err);
            super::monitoring::TRADES_FAILED.inc();
            
            return Ok(SwapResult {
                signature: String::new(),
                input_amount: amount,
                output_amount: quote.out_amount.parse().unwrap_or(0),
                input_token: input_mint.to_string(),
                output_token: output_mint.to_string(),
                price_impact: quote.price_impact_pct,
                success: false,
                error: Some(format!("Simulation failed: {:?}", simulation.err)),
            });
        }
        
        log::info!("✅ Simulation successful, sending transaction...");
        
        // Sign transaction
        // Note: Transaction is already partially signed by Jupiter
        // We need to add our signature
        let recent_blockhash = rpc.get_latest_blockhash().await?;
        transaction.message.recent_blockhash = recent_blockhash;
        transaction.sign(&[&self.keypair], recent_blockhash);
        
        // Send transaction
        let signature = rpc.send_and_confirm_transaction(&transaction).await?;
        
        log::info!("✅ Swap executed successfully: {}", signature);
        super::monitoring::TRADES_SUCCESSFUL.inc();
        
        Ok(SwapResult {
            signature: signature.to_string(),
            input_amount: amount,
            output_amount: quote.out_amount.parse().unwrap_or(0),
            input_token: input_mint.to_string(),
            output_token: output_mint.to_string(),
            price_impact: quote.price_impact_pct,
            success: true,
            error: None,
        })
    }
    
    /// Check liquidity for a token pair
    pub async fn check_liquidity(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<bool> {
        log::debug!("🔍 Checking liquidity for {} -> {}", input_mint, output_mint);
        
        // Get quote to check if route exists with good liquidity
        let jupiter = self.jupiter_client.lock().await;
        let quote = jupiter.get_quote(super::jupiter_integration::JupiterQuoteRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount,
            slippage_bps: 50, // 0.5% slippage
        }).await?;
        
        // Check if price impact is acceptable (liquidity indicator)
        let has_good_liquidity = quote.price_impact_pct < 3.0;
        
        if has_good_liquidity {
            log::debug!("✅ Good liquidity available (impact: {:.2}%)", quote.price_impact_pct);
        } else {
            log::warn!("⚠️ Low liquidity (impact: {:.2}%)", quote.price_impact_pct);
        }
        
        Ok(has_good_liquidity)
    }
    
    /// Validate swap parameters
    fn validate_swap_params(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<()> {
        if input_mint.is_empty() || output_mint.is_empty() {
            anyhow::bail!("Token mints cannot be empty");
        }
        
        if input_mint == output_mint {
            anyhow::bail!("Cannot swap same token");
        }
        
        if amount == 0 {
            anyhow::bail!("Amount must be greater than 0");
        }
        
        if slippage_bps > 1000 {
            anyhow::bail!("Slippage tolerance too high (max 10%)");
        }
        
        Ok(())
    }
    
    /// Get supported tokens from Jupiter
    pub async fn get_supported_tokens(&self) -> Result<Vec<super::jupiter_integration::TokenInfo>> {
        let jupiter = self.jupiter_client.lock().await;
        jupiter.get_tokens().await
    }
}

/// Common token mints on Solana
pub mod token_mints {
    pub const SOL: &str = "So11111111111111111111111111111111111111112";
    pub const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    pub const USDT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
    pub const RAY: &str = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
    pub const SRM: &str = "SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt";
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_params() {
        let jupiter = Arc::new(Mutex::new(super::super::jupiter_integration::JupiterClient::new(
            "https://quote-api.jup.ag/v6".to_string(),
            true,
        )));
        let rpc = Arc::new(Mutex::new(super::super::solana_rpc::SolanaRpcClient::new(
            vec!["http://localhost:8899".to_string()],
            true,
            solana_sdk::commitment_config::CommitmentConfig::confirmed(),
        )));
        let keypair = Keypair::new();
        
        let executor = DexExecutor::new(jupiter, rpc, keypair, false);
        
        // Valid params
        assert!(executor.validate_swap_params(
            token_mints::SOL,
            token_mints::USDC,
            1000000,
            50
        ).is_ok());
        
        // Same token
        assert!(executor.validate_swap_params(
            token_mints::SOL,
            token_mints::SOL,
            1000000,
            50
        ).is_err());
        
        // Zero amount
        assert!(executor.validate_swap_params(
            token_mints::SOL,
            token_mints::USDC,
            0,
            50
        ).is_err());
        
        // High slippage
        assert!(executor.validate_swap_params(
            token_mints::SOL,
            token_mints::USDC,
            1000000,
            1500
        ).is_err());
    }
}
