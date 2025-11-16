use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterQuote {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub price_impact_pct: f64,
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_mint: String,
    pub fee_amount: String,
}

#[derive(Debug, Serialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
}

pub struct JupiterClient {
    api_url: String,
    client: reqwest::Client,
}

impl JupiterClient {
    pub fn new() -> Self {
        Self {
            api_url: "https://quote-api.jup.ag/v6".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get a quote for swapping tokens
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote, Box<dyn Error>> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount, slippage_bps
        );

        log::debug!("Fetching Jupiter quote: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Jupiter API error: {}", error_text).into());
        }

        let quote: JupiterQuote = response.json().await?;
        log::info!(
            "Got Jupiter quote: {} {} -> {} {}",
            quote.in_amount,
            input_mint,
            quote.out_amount,
            output_mint
        );

        Ok(quote)
    }

    /// Get the best route for a swap
    pub async fn get_best_route(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<JupiterQuote, Box<dyn Error>> {
        // Use default 50 bps (0.5%) slippage
        self.get_quote(input_mint, output_mint, amount, 50).await
    }

    /// Check if a token pair is supported
    pub async fn is_pair_supported(
        &self,
        input_mint: &str,
        output_mint: &str,
    ) -> Result<bool, Box<dyn Error>> {
        // Try to get a quote with minimal amount (1 token unit)
        match self.get_quote(input_mint, output_mint, 1, 50).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jupiter_client_creation() {
        let client = JupiterClient::new();
        assert_eq!(client.api_url, "https://quote-api.jup.ag/v6");
    }
}
