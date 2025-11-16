use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Jupiter Aggregator API integration
/// This module provides integration with Jupiter for optimal swap routing
/// 
/// Jupiter Documentation: https://station.jup.ag/docs/apis/swap-api

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterQuoteRequest {
    pub input_mint: String,  // Token mint address to swap from
    pub output_mint: String, // Token mint address to swap to
    pub amount: u64,         // Amount in smallest unit (lamports/token decimals)
    pub slippage_bps: u16,   // Slippage tolerance in basis points (50 = 0.5%)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterQuoteResponse {
    pub in_amount: String,
    pub out_amount: String,
    pub price_impact_pct: f64,
    pub route_plan: Vec<RoutePlan>,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub quote_response: JupiterQuoteResponse,
    pub user_public_key: String,
    pub wrap_unwrap_sol: bool,
    pub compute_unit_price_micro_lamports: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swap_transaction: String, // Base64 encoded transaction
    pub last_valid_block_height: u64,
}

/// Jupiter API client with simulated and real modes
pub struct JupiterClient {
    pub api_url: String,
    pub simulation_mode: bool,
    client: reqwest::Client,
}

impl JupiterClient {
    /// Create new Jupiter client
    /// 
    /// # Arguments
    /// * `api_url` - Jupiter API endpoint (default: https://quote-api.jup.ag/v6)
    /// * `simulation_mode` - If true, returns simulated responses
    pub fn new(api_url: String, simulation_mode: bool) -> Self {
        log::info!("🔷 Initializing Jupiter Aggregator client");
        log::info!("📝 Simulation mode: {}", simulation_mode);
        
        Self {
            api_url,
            simulation_mode,
            client: reqwest::Client::new(),
        }
    }
    
    /// Get a quote for a swap
    /// 
    /// # Arguments
    /// * `request` - Quote request parameters
    /// 
    /// # Returns
    /// * `Result<JupiterQuoteResponse>` - Quote with routing information
    pub async fn get_quote(&self, request: JupiterQuoteRequest) -> Result<JupiterQuoteResponse> {
        if self.simulation_mode {
            return Ok(self.simulate_quote(&request));
        }
        
        log::info!("🔍 Getting Jupiter quote: {} -> {}", request.input_mint, request.output_mint);
        
        let url = format!("{}/quote", self.api_url);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("inputMint", request.input_mint.as_str()),
                ("outputMint", request.output_mint.as_str()),
                ("amount", &request.amount.to_string()),
                ("slippageBps", &request.slippage_bps.to_string()),
            ])
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Jupiter API error: {}", error_text);
        }
        
        let quote: JupiterQuoteResponse = response.json().await?;
        
        log::info!("✅ Quote received: {} -> {} (impact: {:.2}%)", 
            quote.in_amount, quote.out_amount, quote.price_impact_pct);
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        Ok(quote)
    }
    
    /// Execute a swap based on a quote
    /// 
    /// # Arguments
    /// * `request` - Swap request with quote and user info
    /// 
    /// # Returns
    /// * `Result<SwapResponse>` - Serialized transaction to sign and send
    pub async fn execute_swap(&self, request: SwapRequest) -> Result<SwapResponse> {
        if self.simulation_mode {
            return Ok(self.simulate_swap(&request));
        }
        
        log::info!("🔄 Preparing swap transaction via Jupiter");
        
        let url = format!("{}/swap", self.api_url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Jupiter swap API error: {}", error_text);
        }
        
        let swap_response: SwapResponse = response.json().await?;
        
        log::info!("✅ Swap transaction prepared");
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        
        Ok(swap_response)
    }
    
    /// Get list of supported tokens
    pub async fn get_tokens(&self) -> Result<Vec<TokenInfo>> {
        if self.simulation_mode {
            return Ok(self.get_simulated_tokens());
        }
        
        let url = format!("{}/tokens", self.api_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        let tokens: Vec<TokenInfo> = response.json().await?;
        
        super::monitoring::RPC_REQUESTS_TOTAL.inc();
        Ok(tokens)
    }
    
    /// Simulate quote for testing
    fn simulate_quote(&self, request: &JupiterQuoteRequest) -> JupiterQuoteResponse {
        log::debug!("📝 Simulating Jupiter quote");
        
        // Simulate price with 1-2% slippage
        let out_amount = (request.amount as f64 * 0.98) as u64;
        
        JupiterQuoteResponse {
            in_amount: request.amount.to_string(),
            out_amount: out_amount.to_string(),
            price_impact_pct: 0.15,
            route_plan: vec![RoutePlan {
                swap_info: SwapInfo {
                    amm_key: "SimulatedAMM".to_string(),
                    label: "Simulated DEX".to_string(),
                    input_mint: request.input_mint.clone(),
                    output_mint: request.output_mint.clone(),
                    in_amount: request.amount.to_string(),
                    out_amount: out_amount.to_string(),
                    fee_amount: "1000".to_string(),
                    fee_mint: request.input_mint.clone(),
                },
                percent: 100,
            }],
            other_amount_threshold: (out_amount * 995 / 1000).to_string(), // 0.5% slippage
            swap_mode: "ExactIn".to_string(),
            slippage_bps: request.slippage_bps,
        }
    }
    
    /// Simulate swap transaction
    fn simulate_swap(&self, _request: &SwapRequest) -> SwapResponse {
        log::debug!("📝 Simulating Jupiter swap");
        
        SwapResponse {
            swap_transaction: "SimulatedTransactionBase64".to_string(),
            last_valid_block_height: 999999999,
        }
    }
    
    /// Get simulated token list
    fn get_simulated_tokens(&self) -> Vec<TokenInfo> {
        vec![
            TokenInfo {
                address: "So11111111111111111111111111111111111111112".to_string(),
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
                decimals: 9,
                logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png".to_string()),
            },
            TokenInfo {
                address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png".to_string()),
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_uri: Option<String>,
}

/// Helper function to convert SOL amount to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

/// Helper function to convert lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Helper function to convert token amount to smallest unit
pub fn token_to_smallest_unit(amount: f64, decimals: u8) -> u64 {
    (amount * 10_f64.powi(decimals as i32)) as u64
}

/// Helper function to convert smallest unit to token amount
pub fn smallest_unit_to_token(amount: u64, decimals: u8) -> f64 {
    amount as f64 / 10_f64.powi(decimals as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simulated_quote() {
        let client = JupiterClient::new(
            "https://quote-api.jup.ag/v6".to_string(),
            true
        );
        
        let request = JupiterQuoteRequest {
            input_mint: "So11111111111111111111111111111111111111112".to_string(),
            output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: sol_to_lamports(1.0),
            slippage_bps: 50,
        };
        
        let quote = client.get_quote(request).await.unwrap();
        assert!(!quote.out_amount.is_empty());
    }
    
    #[test]
    fn test_conversions() {
        assert_eq!(sol_to_lamports(1.0), 1_000_000_000);
        assert_eq!(lamports_to_sol(1_000_000_000), 1.0);
        assert_eq!(token_to_smallest_unit(1.0, 6), 1_000_000);
        assert_eq!(smallest_unit_to_token(1_000_000, 6), 1.0);
    }
}
