use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use crate::http_client::SharedHttpClient;

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

#[derive(Debug)]
pub struct JupiterClient {
    quote_api_url: String,
    ultra_api_url: String,
    api_key: Option<String>,
    client: Arc<reqwest::Client>, // Use shared client with connection pooling
}

impl JupiterClient {
    pub fn new() -> Self {
        // Get Jupiter API key from environment (optional for quote API, required for Ultra Swap API)
        let api_key = std::env::var("JUPITER_API_KEY").ok();
        
        if api_key.is_some() {
            log::info!("✅ Jupiter API key loaded from environment (Ultra Swap API enabled)");
        } else {
            log::warn!("⚠️ Jupiter API key not found. Ultra Swap API features will be unavailable.");
            log::info!("   Set JUPITER_API_KEY environment variable or add it to .env file");
            log::info!("   Get your API key from: https://portal.jup.ag/");
        }
        
        Self {
            quote_api_url: "https://quote-api.jup.ag/v6".to_string(),
            ultra_api_url: "https://api.jup.ag/ultra/v1".to_string(),
            api_key,
            client: SharedHttpClient::shared(), // Use shared HTTP client with connection pooling
        }
    }
    
    /// Build request with API key if available (for Ultra Swap API)
    fn build_ultra_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.get(url);
        
        if let Some(ref key) = self.api_key {
            request = request.header("x-jup-api-key", key);
            log::debug!("🔑 Using Jupiter API key for Ultra Swap API request");
        }
        
        request
    }
    
    /// Build POST request with API key if available (for Ultra Swap API)
    fn build_ultra_post_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.post(url);
        
        if let Some(ref key) = self.api_key {
            request = request.header("x-jup-api-key", key);
            log::debug!("🔑 Using Jupiter API key for Ultra Swap API POST request");
        }
        
        request.header("Content-Type", "application/json")
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
            self.quote_api_url, input_mint, output_mint, amount, slippage_bps
        );

        log::debug!("Fetching Jupiter quote: {}", url);

        // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable instead of retry_with_backoff
        // This prevents retrying non-retryable errors (e.g., ValidationError)
        // CRITICAL IMPROVEMENT #5: Use aggressive retry config for critical trading operations
        use crate::error_handling::{retry_with_backoff_retryable, RetryConfig, TradingError, map_http_status_to_error};
        
        let url_clone = url.clone();
        let client_clone = self.client.clone();
        
        // RETRY + ERROR HANDLING: Use retry_with_backoff_retryable for robust error handling
        // Circuit breaker integration can be added as a field to JupiterClient if needed
        // CRITICAL IMPROVEMENT #5: Use aggressive config for critical trading operations
        let result: Result<JupiterQuote, TradingError> = retry_with_backoff_retryable(
            || {
                let url = url_clone.clone();
                let client = client_clone.clone();
                Box::pin(async move {
                    let response = client.get(&url).send().await
                        .map_err(|e| {
                            let error_str = e.to_string();
                            // CRITICAL IMPROVEMENT #4: Detect timeout errors
                            if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
                                TradingError::TimeoutError(format!("Request timeout: {}", e))
                            } else if error_str.contains("dns") || error_str.contains("connection") || error_str.contains("No such host") {
                                TradingError::NetworkError(format!("Network error: {}", e))
                            } else {
                                TradingError::NetworkError(format!("Network error: {}", e))
                            }
                        })?;
                    
                    let status = response.status().as_u16();
                    if !response.status().is_success() {
                        // CRITICAL IMPROVEMENT #2: Use map_http_status_to_error() for proper error type mapping
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let error = map_http_status_to_error(status, error_text);
                        // Error is already properly mapped, retry_with_backoff_retryable will check if it's retryable
                        return Err(error);
                    }
                    
                    let quote: JupiterQuote = response.json().await
                        .map_err(|e| TradingError::ApiError(format!("Failed to parse response: {}", e)))?;
                    Ok(quote)
                })
            },
            RetryConfig::aggressive(), // CRITICAL IMPROVEMENT #5: Aggressive retry for critical trading operations
            "Jupiter get_quote",
        ).await;
        
        // Convert TradingError to Box<dyn Error> for return type
        result.map_err(|e| Box::new(e) as Box<dyn Error>)
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
    
    // ========== Jupiter Ultra Swap API Methods ==========
    // Ultra Swap API documentation: https://dev.jup.ag/api-reference/ultra
    
    /// Ultra Swap API: Get unsigned swap transaction
    /// Endpoint: POST /ultra/v1/order
    /// Returns: Base64-encoded unsigned transaction
    #[allow(dead_code)]
    pub async fn ultra_get_order(
        &self,
        user_public_key: &str,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: Option<u16>,
    ) -> Result<UltraOrderResponse, Box<dyn Error>> {
        if self.api_key.is_none() {
            return Err("Jupiter API key required for Ultra Swap API. Set JUPITER_API_KEY environment variable.".into());
        }
        
        let url = format!("{}/order", self.ultra_api_url);
        
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct OrderRequest {
            user_public_key: String,
            input_mint: String,
            output_mint: String,
            amount: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            slippage_bps: Option<u16>,
        }
        
        let request_body = OrderRequest {
            user_public_key: user_public_key.to_string(),
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount: amount.to_string(),
            slippage_bps: slippage_bps.or(Some(50)),
        };
        
        log::debug!("Requesting Ultra Swap order: {}", url);
        
        let response = self.build_ultra_post_request(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Jupiter Ultra Swap API error: {}", error_text).into());
        }
        
        let order: UltraOrderResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(order)
    }
    
    /// Ultra Swap API: Execute signed transaction
    /// Endpoint: POST /ultra/v1/execute
    /// Returns: Execution status
    #[allow(dead_code)]
    pub async fn ultra_execute(
        &self,
        signed_transaction: &str,
    ) -> Result<UltraExecuteResponse, Box<dyn Error>> {
        if self.api_key.is_none() {
            return Err("Jupiter API key required for Ultra Swap API. Set JUPITER_API_KEY environment variable.".into());
        }
        
        let url = format!("{}/execute", self.ultra_api_url);
        
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ExecuteRequest {
            signed_transaction: String,
        }
        
        let request_body = ExecuteRequest {
            signed_transaction: signed_transaction.to_string(),
        };
        
        log::debug!("Executing Ultra Swap transaction: {}", url);
        
        let response = self.build_ultra_post_request(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Jupiter Ultra Swap API error: {}", error_text).into());
        }
        
        let result: UltraExecuteResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(result)
    }
    
    /// Ultra Swap API: Get token holdings/balances
    /// Endpoint: GET /ultra/v1/holdings
    #[allow(dead_code)]
    pub async fn ultra_get_holdings(
        &self,
        user_public_key: &str,
    ) -> Result<UltraHoldingsResponse, Box<dyn Error>> {
        if self.api_key.is_none() {
            return Err("Jupiter API key required for Ultra Swap API. Set JUPITER_API_KEY environment variable.".into());
        }
        
        let url = format!("{}/holdings?userPublicKey={}", self.ultra_api_url, user_public_key);
        
        log::debug!("Fetching Ultra Swap holdings: {}", url);
        
        let response = self.build_ultra_request(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Jupiter Ultra Swap API error: {}", error_text).into());
        }
        
        let holdings: UltraHoldingsResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(holdings)
    }
    
    /// Ultra Swap API: Get token information and warnings (Shield)
    /// Endpoint: GET /ultra/v1/shield
    #[allow(dead_code)]
    pub async fn ultra_shield(
        &self,
        mints: Vec<String>,
    ) -> Result<UltraShieldResponse, Box<dyn Error>> {
        if self.api_key.is_none() {
            return Err("Jupiter API key required for Ultra Swap API. Set JUPITER_API_KEY environment variable.".into());
        }
        
        let mints_param = mints.join(",");
        let url = format!("{}/shield?mints={}", self.ultra_api_url, mints_param);
        
        log::debug!("Fetching Ultra Swap shield info: {}", url);
        
        let response = self.build_ultra_request(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Jupiter Ultra Swap API error: {}", error_text).into());
        }
        
        let shield: UltraShieldResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(shield)
    }
    
    /// Ultra Swap API: Search tokens by symbol, name, or mint address
    /// Endpoint: GET /ultra/v1/search
    #[allow(dead_code)]
    pub async fn ultra_search(
        &self,
        query: &str,
    ) -> Result<UltraSearchResponse, Box<dyn Error>> {
        if self.api_key.is_none() {
            return Err("Jupiter API key required for Ultra Swap API. Set JUPITER_API_KEY environment variable.".into());
        }
        
        let url = format!("{}/search?q={}", self.ultra_api_url, query);
        
        log::debug!("Searching Ultra Swap: {}", url);
        
        let response = self.build_ultra_request(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Jupiter Ultra Swap API error: {}", error_text).into());
        }
        
        let search: UltraSearchResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(search)
    }
    
    /// Ultra Swap API: Get available routers
    /// Endpoint: GET /ultra/v1/routers
    #[allow(dead_code)]
    pub async fn ultra_get_routers(&self) -> Result<UltraRoutersResponse, Box<dyn Error>> {
        if self.api_key.is_none() {
            return Err("Jupiter API key required for Ultra Swap API. Set JUPITER_API_KEY environment variable.".into());
        }
        
        let url = format!("{}/routers", self.ultra_api_url);
        
        log::debug!("Fetching Ultra Swap routers: {}", url);
        
        let response = self.build_ultra_request(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Jupiter Ultra Swap API error: {}", error_text).into());
        }
        
        let routers: UltraRoutersResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(routers)
    }
}

// ========== Jupiter Ultra Swap API Response Structures ==========

/// Ultra Swap API: Order response (unsigned transaction)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UltraOrderResponse {
    pub transaction: String, // Base64-encoded unsigned transaction
    #[serde(default)]
    pub quote: Option<serde_json::Value>, // Optional quote information
}

/// Ultra Swap API: Execute response (transaction status)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UltraExecuteResponse {
    pub status: String, // "pending", "processing", "completed", "failed"
    #[serde(default)]
    pub transaction_signature: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Ultra Swap API: Holdings response (token balances)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UltraHoldingsResponse {
    #[serde(default)]
    pub balances: Vec<TokenBalance>,
    #[serde(default)]
    pub total_value_usd: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TokenBalance {
    pub mint: String,
    pub amount: String,
    pub decimals: u8,
    #[serde(default)]
    pub value_usd: Option<f64>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Ultra Swap API: Shield response (token information and warnings)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UltraShieldResponse {
    #[serde(default)]
    pub tokens: Vec<TokenShieldInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TokenShieldInfo {
    pub mint: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub risk_score: Option<f64>,
    #[serde(default)]
    pub is_verified: Option<bool>,
}

/// Ultra Swap API: Search response (token search results)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UltraSearchResponse {
    #[serde(default)]
    pub results: Vec<TokenSearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TokenSearchResult {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    #[serde(default)]
    pub decimals: Option<u8>,
    #[serde(default)]
    pub logo_uri: Option<String>,
}

/// Ultra Swap API: Routers response (available routers)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UltraRoutersResponse {
    #[serde(default)]
    pub routers: Vec<RouterInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RouterInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jupiter_client_creation() {
        let client = JupiterClient::new();
        // Access private fields through public methods or test internals
        // For now, just verify client is created successfully
        assert!(std::mem::size_of_val(&client) > 0);
    }
}
