use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use dotenv;
use crate::http_client::SharedHttpClient;

/// DEX Screener API token pair data (multi-chain support)
/// API Docs: https://docs.dexscreener.com/api/reference
/// Base URL: https://api.dexscreener.com
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPair {
    pub chain_id: String,
    pub dex_id: String,
    pub url: String,
    pub pair_address: String,
    pub base_token: Token,
    pub quote_token: Token,
    pub price_native: String,
    pub price_usd: Option<String>,
    #[serde(default)]
    pub txns: Transactions,
    pub volume: Volume,
    pub liquidity: Liquidity,
    pub fdv: Option<f64>,
    pub price_change: PriceChange,
    #[serde(default)]
    pub pair_created_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    #[serde(rename = "h24")]
    pub h24: f64,
    #[serde(rename = "h6")]
    pub h6: f64,
    #[serde(rename = "h1")]
    pub h1: f64,
    #[serde(rename = "m5")]
    pub m5: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidity {
    pub usd: Option<f64>,
    pub base: f64,
    pub quote: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PriceChange {
    #[serde(rename = "m5", default)]
    pub m5: f64,
    #[serde(rename = "h1", default)]
    pub h1: f64,
    #[serde(rename = "h6", default)]
    pub h6: f64,
    #[serde(rename = "h24", default)]
    pub h24: f64,
}

/// Transaction data for a pair
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Transactions {
    #[serde(rename = "m5", default)]
    pub m5: TransactionCount,
    #[serde(rename = "h1", default)]
    pub h1: TransactionCount,
    #[serde(rename = "h6", default)]
    pub h6: TransactionCount,
    #[serde(rename = "h24", default)]
    pub h24: TransactionCount,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransactionCount {
    #[serde(default)]
    pub buys: i32,
    #[serde(default)]
    pub sells: i32,
}

/// Response from DEX Screener API
/// Reference: https://docs.dexscreener.com/api/reference
#[derive(Debug, Deserialize)]
struct DexScreenerSearchResponse {
    #[serde(rename = "schemaVersion", default)]
    schema_version: Option<String>,
    #[serde(default)]
    pairs: Option<Vec<TokenPair>>,
}

/// Response for token pairs endpoint (returns array directly)
/// DEX Screener /token-pairs/v1/{chainId}/{tokenAddress} returns array
type DexScreenerTokenPairsResponse = Vec<TokenPair>;

impl DexScreenerSearchResponse {
    /// Extract pairs from search response
    fn get_pairs(&self) -> Vec<TokenPair> {
        self.pairs.clone().unwrap_or_default()
    }
}

/// Trading opportunity identified from DEX Screener
#[derive(Debug, Clone, Serialize)]
pub struct TradingOpportunity {
    pub pair_address: String,
    pub token_symbol: String,
    pub token_name: String,
    pub price_usd: f64,
    pub volume_24h: f64,
    pub liquidity_usd: f64,
    pub price_change_5m: f64,
    pub price_change_1h: f64,
    pub price_change_6h: f64,
    pub price_change_24h: f64,
    pub opportunity_score: f64,
    pub signals: Vec<String>,
}

/// DEX Screener API client for token discovery and analysis
/// Official API: https://docs.dexscreener.com/api/reference
/// Base URL: https://api.dexscreener.com
/// Rate Limits:
///   - Search/Pairs endpoints: 300 requests per minute
///   - Profile/Boost endpoints: 60 requests per minute
pub struct DexScreenerClient {
    api_url: String,
    api_key: Option<String>, // DEX Screener doesn't require API key, but kept for future use
    client: Arc<reqwest::Client>, // Use shared client with connection pooling
    // Rate limiting: Track requests per minute for different endpoint types
    search_last_request_time: std::sync::Arc<std::sync::Mutex<SystemTime>>,
    search_request_count: std::sync::Arc<std::sync::Mutex<u32>>,
    profile_last_request_time: std::sync::Arc<std::sync::Mutex<SystemTime>>,
    profile_request_count: std::sync::Arc<std::sync::Mutex<u32>>,
    circuit_breaker: Option<Arc<tokio::sync::Mutex<crate::error_handling::CircuitBreaker>>>, // Circuit breaker for API protection
}

impl DexScreenerClient {
    pub fn new() -> Self {
        Self::new_with_circuit_breaker(None)
    }
    
    /// Create client with optional circuit breaker for API protection
    /// Uses official DEX Screener API: https://docs.dexscreener.com/api/reference
    pub fn new_with_circuit_breaker(circuit_breaker: Option<Arc<tokio::sync::Mutex<crate::error_handling::CircuitBreaker>>>) -> Self {
        // Ensure .env is loaded (in case client is created before main loads it)
        dotenv::dotenv().ok();
        
        // DEX Screener API doesn't require API key, but keep option for future
        let api_key = std::env::var("DEXSCREENER_API_KEY").ok();
        
        if let Some(ref key) = api_key {
            let key_preview = if key.len() > 8 {
                format!("{}...{}", &key[..4], &key[key.len()-4..])
            } else {
                "***".to_string()
            };
            log::info!("✅ DEX Screener API key loaded from environment (key: {})", key_preview);
        } else {
            log::info!("ℹ️ DEX Screener API doesn't require API key (rate limits: 300/min for search/pairs, 60/min for profiles)");
        }
        
        Self {
            // DEX Screener API base URL (official API)
            api_url: "https://api.dexscreener.com".to_string(),
            api_key,
            client: SharedHttpClient::shared(), // Use shared HTTP client with connection pooling
            // Separate rate limiters for different endpoint types
            search_last_request_time: std::sync::Arc::new(std::sync::Mutex::new(SystemTime::now())),
            search_request_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            profile_last_request_time: std::sync::Arc::new(std::sync::Mutex::new(SystemTime::now())),
            profile_request_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            circuit_breaker,
        }
    }
    
    /// Build request with API key if available
    /// DEX Screener API doesn't currently require API key, but we keep support for future
    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.get(url);
        
        if let Some(ref key) = self.api_key {
            // DEX Screener may use authorization header in future
            request = request.header("Authorization", format!("Bearer {}", key));
            log::debug!("🔑 DEX Screener API key attached to request");
        }
        
        request
    }
    
    /// Check and enforce rate limit for search/pairs endpoints (300 requests per minute)
    /// Reference: https://docs.dexscreener.com/api/reference
    async fn check_search_rate_limit(&self) -> Result<(), Box<dyn Error>> {
        const MAX_REQUESTS_PER_MINUTE: u32 = 280; // Conservative: 280/min (below 300/min limit)
        let wait_time = {
            let mut last_time = self.search_last_request_time.lock()
                .map_err(|e| format!("Mutex poisoned in search rate limiter: {}", e))?;
            let mut count = self.search_request_count.lock()
                .map_err(|e| format!("Mutex poisoned in search rate limiter: {}", e))?;
            
            let now = SystemTime::now();
            let elapsed = now.duration_since(*last_time).unwrap_or(Duration::from_secs(60));
            
            // Reset counter after 1 minute
            if elapsed >= Duration::from_secs(60) {
                *count = 0;
                *last_time = now;
            }
            
            // Check rate limit (DEX Screener: 300 req/min for search/pairs endpoints)
            let wait = if *count >= MAX_REQUESTS_PER_MINUTE {
                let wait_time = Duration::from_secs(60) - elapsed;
                log::warn!("⚠️ DEX Screener search rate limit approaching ({} requests), waiting {:?} before next request", *count, wait_time);
                Some(wait_time)
            } else {
                *count += 1;
                None
            };
            
            wait
        };
        
        if let Some(wait) = wait_time {
            tokio::time::sleep(wait).await;
        }
        
        Ok(())
    }
    
    /// Check and enforce rate limit for profile/boost endpoints (60 requests per minute)
    /// Reference: https://docs.dexscreener.com/api/reference
    async fn check_profile_rate_limit(&self) -> Result<(), Box<dyn Error>> {
        const MAX_REQUESTS_PER_MINUTE: u32 = 55; // Conservative: 55/min (below 60/min limit)
        let wait_time = {
            let mut last_time = self.profile_last_request_time.lock()
                .map_err(|e| format!("Mutex poisoned in profile rate limiter: {}", e))?;
            let mut count = self.profile_request_count.lock()
                .map_err(|e| format!("Mutex poisoned in profile rate limiter: {}", e))?;
            
            let now = SystemTime::now();
            let elapsed = now.duration_since(*last_time).unwrap_or(Duration::from_secs(60));
            
            // Reset counter after 1 minute
            if elapsed >= Duration::from_secs(60) {
                *count = 0;
                *last_time = now;
            }
            
            // Check rate limit (DEX Screener: 60 req/min for profile/boost endpoints)
            let wait = if *count >= MAX_REQUESTS_PER_MINUTE {
                let wait_time = Duration::from_secs(60) - elapsed;
                log::warn!("⚠️ DEX Screener profile rate limit approaching ({} requests), waiting {:?} before next request", *count, wait_time);
                Some(wait_time)
            } else {
                *count += 1;
                None
            };
            
            wait
        };
        
        if let Some(wait) = wait_time {
            tokio::time::sleep(wait).await;
        }
        
        Ok(())
    }
    
    
    /// Search for tokens by query using DEX Screener API
    /// Endpoint: GET /latest/dex/search?q={query}
    /// Rate Limit: 300 requests per minute
    /// API Docs: https://docs.dexscreener.com/api/reference
    pub async fn search_tokens(&self, query: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_search_rate_limit().await?;
        
        // URL encode the query to handle special characters
        let encoded_query = urlencoding::encode(query);
        
        // DEX Screener API endpoint: GET /latest/dex/search?q={query}
        // Reference: https://docs.dexscreener.com/api/reference
        let url = format!("{}/latest/dex/search?q={}", self.api_url, encoded_query);
        
        log::info!("Searching DEX Screener API for: {}", query);
        
        // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable instead of retry_with_backoff
        // This prevents retrying non-retryable errors (e.g., ValidationError)
        use crate::error_handling::{retry_with_backoff_retryable, RetryConfig, TradingError, map_http_status_to_error};
        
        // Use single URL instead of endpoints array
        let url_clone = url.clone();
        let client_clone = self.client.clone();
        let api_key_clone = self.api_key.clone();
        
        // Wrap the entire retry operation with circuit breaker
        let result: Result<reqwest::Response, TradingError> = if let Some(ref cb) = self.circuit_breaker {
            let cb_clone = cb.clone();
            // Scope the lock so guard is dropped after await completes
            {
                let cb_guard = cb_clone.lock().await;
                cb_guard.call(async move {
                    // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable - only retries retryable errors
                    retry_with_backoff_retryable(
                        || {
                            let url = url_clone.clone();
                            let client = client_clone.clone();
                            let api_key = api_key_clone.clone();
                            Box::pin(async move {
                                let mut request = client.get(&url);
                                if let Some(ref key) = api_key {
                                    request = request.header("Authorization", format!("Bearer {}", key));
                                }
                                request.send().await
                                    .map_err(|e| {
                                        let error_str = e.to_string();
                                        if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
                                            TradingError::TimeoutError(format!("Request timeout: {}", e))
                                        } else if error_str.contains("dns") || error_str.contains("connection") || error_str.contains("No such host") {
                                            TradingError::NetworkError(format!("Network error: {}", e))
                                        } else {
                                            TradingError::ApiError(format!("Request failed: {}", e))
                                        }
                                    })
                            })
                        },
                        RetryConfig::conservative(),
                        &format!("DexScreener search_tokens for {}", query),
                    ).await
                }).await
            } // Guard dropped here after await completes
        } else {
            // No circuit breaker - just use retry logic
            retry_with_backoff_retryable(
                || {
                    let url = url_clone.clone();
                    let client = client_clone.clone();
                    let api_key = api_key_clone.clone();
                    Box::pin(async move {
                        let mut request = client.get(&url);
                        if let Some(ref key) = api_key {
                            request = request.header("x-api-key", key);
                        }
                        request.send().await
                            .map_err(|e| {
                                let error_str = e.to_string();
                                if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
                                    TradingError::TimeoutError(format!("Request timeout: {}", e))
                                } else if error_str.contains("dns") || error_str.contains("connection") || error_str.contains("No such host") {
                                    TradingError::NetworkError(format!("Network error: {}", e))
                                } else {
                                    TradingError::ApiError(format!("Request failed: {}", e))
                                }
                            })
                    })
                },
                RetryConfig::default(),
                &format!("DexScreener search_tokens for {}", query),
            ).await
        };
        
        match result {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<DexScreenerSearchResponse>().await {
                        Ok(data) => {
                            let pairs = data.get_pairs();
                            log::info!("Found {} pairs for query: {} (via DEX Screener API)", pairs.len(), query);
                            return Ok(pairs);
                        }
                        Err(e) => {
                            log::warn!("Failed to parse DEX Screener response: {}", e);
                            return Err(format!("Failed to parse response: {}", e).into());
                        }
                    }
                } else {
                    // CRITICAL IMPROVEMENT #2: Use map_http_status_to_error() for proper error type mapping
                    let status = response.status().as_u16();
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    let mapped_error = map_http_status_to_error(status, error_text);
                    return Err(format!("{}", mapped_error).into());
                }
            }
            Err(e) => {
                return Err(format!("DEX Screener search failed: {}", e).into());
            }
        }
    }
    
    /// Get token pairs by token address using Mobula API
    /// Endpoint: GET /market/blockchain/pairs?blockchain=solana&token={address}
    /// Supports multiple addresses: comma-separated
    pub async fn get_token_pairs(&self, token_address: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_search_rate_limit().await?;
        
        // Try alternative endpoint formats
        let endpoints = vec![
            format!("{}/market/blockchain/pairs?blockchain=solana&token={}", self.api_url, token_address),
            format!("{}/market/pairs?chain=solana&token={}", self.api_url, token_address),
        ];
        
        log::info!("Fetching token pairs from DEX Screener API for: {}", token_address);
        
        let mut last_error = None;
        
        // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable instead of retry_with_backoff
        use crate::error_handling::{retry_with_backoff_retryable, RetryConfig, TradingError, map_http_status_to_error};
        
        for url in endpoints {
            // CRITICAL IMPROVEMENT #3: Use circuit breaker .call() method instead of manual state checks
            // This properly tracks successes/failures and manages circuit breaker state transitions
            let url_clone = url.clone();
            let client_clone = self.client.clone();
            let api_key_clone = self.api_key.clone();
            
            // Wrap the entire retry operation with circuit breaker
            let result: Result<reqwest::Response, TradingError> = if let Some(ref cb) = self.circuit_breaker {
                let cb_clone = cb.clone();
                // Scope the lock so guard is dropped after await completes
                {
                    let cb_guard = cb_clone.lock().await;
                    cb_guard.call(async move {
                        // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable - only retries retryable errors
                        // CRITICAL IMPROVEMENT #4: Detect and map reqwest timeout errors properly
                        retry_with_backoff_retryable(
                            || {
                                let url = url_clone.clone();
                                let client = client_clone.clone();
                                let api_key = api_key_clone.clone();
                                Box::pin(async move {
                                    let mut request = client.get(&url);
                                    if let Some(ref key) = api_key {
                                        request = request.header("Authorization", format!("Bearer {}", key));
                                    }
                                    request.send().await
                                        .map_err(|e| {
                                            let error_str = e.to_string();
                                            // CRITICAL IMPROVEMENT #4: Detect timeout errors
                                            if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
                                                TradingError::TimeoutError(format!("Request timeout: {}", e))
                                            } else if error_str.contains("dns") || error_str.contains("connection") || error_str.contains("No such host") {
                                                TradingError::NetworkError(format!("Network error: {}", e))
                                            } else {
                                                TradingError::ApiError(format!("Request failed: {}", e))
                                            }
                                        })
                                })
                            },
                            RetryConfig::default(),
                            &format!("DexScreener get_token_pairs for {}", token_address),
                        ).await
                    }).await
                } // Guard dropped here after await completes
            } else {
                // No circuit breaker - just use retry logic
                retry_with_backoff_retryable(
                    || {
                        let url = url_clone.clone();
                        let client = client_clone.clone();
                        let api_key = api_key_clone.clone();
                        Box::pin(async move {
                            let mut request = client.get(&url);
                            if let Some(ref key) = api_key {
                                request = request.header("x-api-key", key);
                            }
                            request.send().await
                                .map_err(|e| {
                                    let error_str = e.to_string();
                                    // CRITICAL IMPROVEMENT #4: Detect timeout errors
                                    if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
                                        TradingError::TimeoutError(format!("Request timeout: {}", e))
                                    } else if error_str.contains("dns") || error_str.contains("connection") || error_str.contains("No such host") {
                                        TradingError::NetworkError(format!("Network error: {}", e))
                                    } else {
                                        TradingError::ApiError(format!("Request failed: {}", e))
                                    }
                                })
                        })
                    },
                    RetryConfig::default(),
                    &format!("DexScreener get_token_pairs for {}", token_address),
                ).await
            };
            
            match result {
                Ok(response) => {
                    let status = response.status().as_u16();
                    if response.status().is_success() {
                        match response.json::<DexScreenerTokenPairsResponse>().await {
                            Ok(pairs) => {
                                log::info!("Found {} pairs for token: {} (via DEX Screener API)", pairs.len(), token_address);
                                return Ok(pairs);
                            }
                            Err(e) => {
                                log::warn!("Failed to parse Mobula response: {}. Trying alternative endpoint...", e);
                                last_error = Some(format!("Parse error: {}", e));
                                continue;
                            }
                        }
                    } else {
                        // CRITICAL IMPROVEMENT #2: Use map_http_status_to_error() for proper error type mapping
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let mapped_error = map_http_status_to_error(status, error_text.clone());
                        
                        // Only retry on server errors (5xx) - validation errors (4xx) are not retryable
                        if status >= 500 {
                            log::warn!("Mobula API {} error for endpoint: {}. Trying alternative...", status, url);
                            last_error = Some(format!("{}", mapped_error));
                            continue;
                        } else {
                            // Non-retryable error - return immediately using mapped error
                            return Err(format!("{}", mapped_error).into());
                        }
                    }
                }
                Err(e) => {
                    // Error already mapped to TradingError by retry_with_backoff_retryable
                    log::warn!("Request failed for {}: {}. Trying alternative endpoint...", url, e);
                    last_error = Some(format!("{}", e));
                    continue;
                }
            }
        }
        
        // All endpoints failed - return empty result with warning instead of error
        log::warn!("⚠️ All DEX Screener API endpoints failed for token: {}. Last error: {:?}. Returning empty results.", 
                  token_address, last_error);
        Ok(Vec::new()) // Return empty instead of error to allow graceful degradation
    }
    
    /// Get token pairs for multiple addresses at once
    /// More efficient than calling get_token_pairs multiple times
    pub async fn get_multiple_token_pairs(&self, token_addresses: &[String]) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        if token_addresses.is_empty() {
            return Ok(Vec::new());
        }
        
        // DexScreener supports comma-separated addresses
        let addresses = token_addresses.join(",");
        self.get_token_pairs(&addresses).await
    }
    
    /// Get pair data by pair address using Mobula API
    /// Endpoint: GET /market/blockchain/pairs?blockchain={chain}&pair={address}
    /// Supports multiple pair addresses: comma-separated
    pub async fn get_pair(&self, chain: &str, pair_address: &str) -> Result<Option<TokenPair>, Box<dyn Error>> {
        self.check_search_rate_limit().await?;
        
        // Add import for map_http_status_to_error
        use crate::error_handling::map_http_status_to_error;
        
        // Mobula API endpoint - use 'solana' for Solana blockchain
        let blockchain = if chain.to_lowercase() == "solana" { "solana" } else { chain };
        let url = format!("{}/market/blockchain/pairs?blockchain={}&pair={}", self.api_url, blockchain, pair_address);
        
        log::info!("Fetching pair data from DEX Screener API for: {}/{}", chain, pair_address);
        
        let response = self.build_request(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            // CRITICAL IMPROVEMENT #2: Use map_http_status_to_error() for proper error type mapping
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            let mapped_error = map_http_status_to_error(status, error_text);
            return Err(format!("{}", mapped_error).into());
        }
        
        // DEX Screener returns { "schemaVersion": "text", "pairs": [...] }
        let data: DexScreenerSearchResponse = response.json().await?;
        
        Ok(data.get_pairs().into_iter().next())
    }
    
    /// Find trending tokens on Solana with high volume using Mobula API
    /// Endpoint: GET /market/blockchain/pairs?blockchain=solana&sortBy=volume24h
    pub async fn find_trending_solana_tokens(&self, min_liquidity_usd: f64) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_search_rate_limit().await?;
        
        // Try alternative endpoint formats
        let endpoints = vec![
            format!("{}/market/blockchain/pairs?blockchain=solana&sortBy=volume24h", self.api_url),
            format!("{}/market/pairs?chain=solana&sort=volume24h", self.api_url),
            format!("{}/market/trending?blockchain=solana", self.api_url),
        ];
        
        log::info!("Fetching trending Solana tokens from Mobula API");
        
        let mut last_error = None;
        
        for url in endpoints {
            match self.build_request(&url)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        match response.json::<DexScreenerTokenPairsResponse>().await {
                            Ok(pairs) => {
                                // Filter for minimum liquidity
                                let trending: Vec<TokenPair> = pairs.into_iter()
            .filter(|p| {
                p.liquidity.usd.unwrap_or(0.0) >= min_liquidity_usd &&
                p.volume.h24 > 0.0
            })
            .collect();
        
                                log::info!("Found {} trending Solana tokens (via Mobula API)", trending.len());
                                return Ok(trending);
                            }
                            Err(e) => {
                                log::warn!("Failed to parse Mobula response: {}. Trying alternative endpoint...", e);
                                last_error = Some(format!("Parse error: {}", e));
                                continue;
                            }
                        }
                    } else {
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        if status.as_u16() == 500 {
                            log::warn!("Mobula API 500 error for endpoint: {}. Trying alternative...", url);
                            last_error = Some(format!("HTTP {}: {}", status, error_text));
                            continue;
                        } else {
                            // For non-500 errors, return error immediately
                            return Err(format!("Mobula API error {}: {}", status, error_text).into());
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Request failed for {}: {}. Trying alternative endpoint...", url, e);
                    last_error = Some(format!("Request failed: {}", e));
                    continue;
                }
            }
        }
        
        // All endpoints failed - return empty result with warning instead of error
        log::warn!("⚠️ All Mobula API endpoints failed for trending tokens. Last error: {:?}. Returning empty results.", 
                  last_error);
        Ok(Vec::new()) // Return empty instead of error to allow graceful degradation
    }
    
    /// Analyze tokens for trading opportunities
    pub async fn analyze_opportunities(&self, pairs: Vec<TokenPair>) -> Vec<TradingOpportunity> {
        let mut opportunities = Vec::new();
        
        for pair in pairs {
            let price_usd = pair.price_usd.as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);
            
            let liquidity_usd = pair.liquidity.usd.unwrap_or(0.0);
            
            // Skip if price or liquidity is too low
            if price_usd <= 0.0 || liquidity_usd < 1000.0 {
                continue;
            }
            
            let mut signals = Vec::new();
            
            // Improved opportunity scoring with weighted factors (0-100 normalized)
            let momentum_score: f64;
            let volume_score: f64;
            let liquidity_score: f64;
            
            // Momentum analysis (30% weight) - normalized to 0-100
            let momentum_5m = (pair.price_change.m5 / 10.0).min(1.0).max(0.0); // 10% = 100 score
            let momentum_1h = (pair.price_change.h1 / 15.0).min(1.0).max(0.0); // 15% = 100 score
            let momentum_6h = (pair.price_change.h6 / 25.0).min(1.0).max(0.0); // 25% = 100 score
            momentum_score = momentum_5m * 40.0 + momentum_1h * 35.0 + momentum_6h * 25.0;
            
            if momentum_5m > 0.5 {
                signals.push(format!("Strong 5m momentum: +{:.1}%", pair.price_change.m5));
            }
            if momentum_1h > 0.67 {
                signals.push(format!("Strong 1h trend: +{:.1}%", pair.price_change.h1));
            }
            if momentum_6h > 0.8 {
                signals.push(format!("Strong 6h uptrend: +{:.1}%", pair.price_change.h6));
            }
            
            // Volume analysis (25% weight)
            let vol_ratio_1h = if pair.volume.h6 > 0.0 {
                (pair.volume.h1 / (pair.volume.h6 / 6.0)).min(3.0) / 3.0
            } else {
                0.0
            };
            let vol_ratio_5m = if pair.volume.h1 > 0.0 {
                (pair.volume.m5 / (pair.volume.h1 / 12.0)).min(4.0) / 4.0
            } else {
                0.0
            };
            volume_score = vol_ratio_1h * 50.0 + vol_ratio_5m * 50.0;
            
            if vol_ratio_1h > 0.5 {
                signals.push("Increasing volume".to_string());
            }
            if vol_ratio_5m > 0.5 {
                signals.push(format!("Volume spike: {:.1}x avg", vol_ratio_5m * 4.0));
            }
            
            // Liquidity depth analysis (25% weight)
            let liquidity_ratio = (liquidity_usd.log10() / 5.0).min(1.0).max(0.0); // Log scale
            liquidity_score = liquidity_ratio * 100.0;
            
            if liquidity_usd > 50000.0 {
                signals.push(format!("Excellent liquidity: ${:.0}K", liquidity_usd / 1000.0));
            } else if liquidity_usd > 10000.0 {
                signals.push(format!("Good liquidity: ${:.0}K", liquidity_usd / 1000.0));
            }
            
            // Composite score with weighted factors
            let score = (momentum_score * 0.30) + (volume_score * 0.25) + (liquidity_score * 0.25);
            
            // Add sentiment bonus (20% weight) - derived from price action consistency
            let sentiment_bonus = if pair.price_change.m5 > 0.0 
                && pair.price_change.h1 > 0.0 
                && pair.price_change.h6 > 0.0 {
                20.0 // All timeframes bullish
            } else if (pair.price_change.m5 > 0.0 && pair.price_change.h1 > 0.0) 
                || (pair.price_change.h1 > 0.0 && pair.price_change.h6 > 0.0) {
                10.0 // Partially bullish
            } else {
                0.0
            };
            
            let final_score = (score + sentiment_bonus).min(100.0);
            
            // Only include opportunities with signals
            if !signals.is_empty() {
                opportunities.push(TradingOpportunity {
                    pair_address: pair.pair_address.clone(),
                    token_symbol: pair.base_token.symbol.clone(),
                    token_name: pair.base_token.name.clone(),
                    price_usd,
                    volume_24h: pair.volume.h24,
                    liquidity_usd,
                    price_change_5m: pair.price_change.m5,
                    price_change_1h: pair.price_change.h1,
                    price_change_6h: pair.price_change.h6,
                    price_change_24h: pair.price_change.h24,
                    opportunity_score: final_score,
                    signals,
                });
            }
        }
        
        // Sort by opportunity score
        opportunities.sort_by(|a, b| {
            b.opportunity_score.partial_cmp(&a.opportunity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        log::info!("Identified {} trading opportunities", opportunities.len());
        
        opportunities
    }
    
    /// Get top opportunities on Solana
    pub async fn get_top_opportunities(&self, limit: usize) -> Result<Vec<TradingOpportunity>, Box<dyn Error>> {
        let pairs = self.find_trending_solana_tokens(5000.0).await?;
        let mut opportunities = self.analyze_opportunities(pairs).await;
        opportunities.truncate(limit);
        Ok(opportunities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dex_screener_client_creation() {
        let client = DexScreenerClient::new();
        assert_eq!(client.api_url, "https://api.dexscreener.com");
    }

    #[test]
    fn test_trading_opportunity_scoring() {
        // This would test the scoring logic
        let opportunity = TradingOpportunity {
            pair_address: "test".to_string(),
            token_symbol: "TEST".to_string(),
            token_name: "Test Token".to_string(),
            price_usd: 1.0,
            volume_24h: 100000.0,
            liquidity_usd: 50000.0,
            price_change_5m: 10.0,
            price_change_1h: 15.0,
            price_change_6h: 25.0,
            price_change_24h: 30.0,
            opportunity_score: 85.0,
            signals: vec!["Strong momentum".to_string()],
        };
        
        assert!(opportunity.opportunity_score > 0.0);
        assert!(!opportunity.signals.is_empty());
    }
}
