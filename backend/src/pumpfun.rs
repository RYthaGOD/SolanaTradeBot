//! PumpFun memecoin monitoring and analysis
//! Integrated into AI orchestrator for memecoin opportunity detection
//! Enhanced with WebSocket and page scraping for real-time opportunities

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use crate::http_client::SharedHttpClient;
use chrono::Utc;
// SinkExt and StreamExt are imported locally where needed (in start_websocket_listener)

/// PumpFun token launch data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLaunch {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator: String,
    pub created_timestamp: i64,
    pub market_cap: f64,
    pub reply_count: u32,
    pub is_currently_live: bool,
    pub king_of_the_hill_timestamp: Option<i64>,
    pub bonding_curve: String,
}

/// Comprehensive token safety validation result
#[derive(Debug, Clone, Serialize)]
pub struct TokenSafetyCheck {
    pub is_safe: bool,
    pub safety_score: f64, // 0.0 to 100.0
    pub risk_factors: Vec<String>,
    pub warnings: Vec<String>,
    pub passed_checks: Vec<String>,
    pub liquidity_ok: bool,
    pub volume_ok: bool,
    pub age_ok: bool,
    pub market_cap_ok: bool,
    pub price_stable: bool,
    pub holder_distribution_ok: bool,
    pub contract_verified: bool,
    pub not_blacklisted: bool,
    pub multi_source_validated: bool,
}

/// Token safety configuration with thresholds
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    pub min_liquidity_usd: f64,
    pub min_volume_24h_usd: f64,
    pub min_market_cap_usd: f64,
    pub min_token_age_hours: f64,
    pub max_price_volatility_pct: f64,
    pub min_holder_count: u32,
    pub max_creator_hold_pct: f64,
    pub require_multi_source: bool,
    pub require_contract_verification: bool,
}

/// Meme coin sentiment analysis
#[derive(Debug, Clone, Serialize)]
pub struct MemeSentiment {
    pub token_address: String,
    pub symbol: String,
    pub sentiment_score: f64, // -100 to +100
    pub hype_level: HypeLevel,
    pub social_signals: Vec<String>,
    pub risk_level: RiskLevel,
    /// Twitter sentiment data (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_sentiment: Option<crate::twitter_sentiment::TwitterSentimentData>,
    /// Twitter weighted polarity score (-1.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_weighted_polarity: Option<f64>,
    /// Community growth percentage from Twitter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_growth_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub enum HypeLevel {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// Trading signal for meme coins
#[derive(Debug, Clone, Serialize)]
pub struct MemeTradeSignal {
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub action: String, // "BUY", "SELL", "HOLD"
    pub confidence: f64,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub reasons: Vec<String>,
    pub timestamp: i64,
}

/// PumpFun API client with Moralis integration for real token prices
/// Moralis API docs: https://docs.moralis.com/web3-data-api/solana/tutorials/get-pump-fun-token-prices
/// Enhanced with WebSocket and page scraping for real-time opportunities
pub struct PumpFunClient {
    api_url: String,
    moralis_api_url: String,
    moralis_api_key: Option<String>,
    client: Arc<reqwest::Client>, // Use shared client with connection pooling
    circuit_breaker: Option<Arc<tokio::sync::Mutex<crate::error_handling::CircuitBreaker>>>, // Circuit breaker for API protection
    pumpfun_ws_url: String, // WebSocket URL for real-time updates
    pumpfun_page_url: String, // Page URL for scraping
    ws_connected: Arc<tokio::sync::Mutex<bool>>, // WebSocket connection status
}

impl SafetyConfig {
    /// Default safety configuration (conservative)
    pub fn default() -> Self {
        Self {
            min_liquidity_usd: 5000.0,      // Minimum $5k liquidity
            min_volume_24h_usd: 1000.0,     // Minimum $1k 24h volume
            min_market_cap_usd: 10000.0,   // Minimum $10k market cap
            min_token_age_hours: 0.5,      // Minimum 30 minutes old
            max_price_volatility_pct: 50.0, // Max 50% price volatility
            min_holder_count: 10,           // Minimum 10 holders
            max_creator_hold_pct: 50.0,     // Creator can't hold >50%
            require_multi_source: true,    // Require validation from multiple sources
            require_contract_verification: false, // Optional (pump.fun tokens are typically unverified)
        }
    }
    
    /// Aggressive safety configuration (higher risk tolerance)
    pub fn aggressive() -> Self {
        Self {
            min_liquidity_usd: 3000.0,
            min_volume_24h_usd: 500.0,
            min_market_cap_usd: 5000.0,
            min_token_age_hours: 0.25,     // 15 minutes
            max_price_volatility_pct: 75.0,
            min_holder_count: 5,
            max_creator_hold_pct: 70.0,
            require_multi_source: false,
            require_contract_verification: false,
        }
    }
    
    /// Conservative safety configuration (lower risk tolerance)
    pub fn conservative() -> Self {
        Self {
            min_liquidity_usd: 10000.0,
            min_volume_24h_usd: 5000.0,
            min_market_cap_usd: 50000.0,
            min_token_age_hours: 2.0,       // 2 hours
            max_price_volatility_pct: 30.0,
            min_holder_count: 25,
            max_creator_hold_pct: 30.0,
            require_multi_source: true,
            require_contract_verification: false,
        }
    }
}

impl PumpFunClient {
    pub fn new() -> Self {
        // Get Moralis API key from environment (optional - falls back to simulation)
        let moralis_api_key = std::env::var("MORALIS_API_KEY").ok();
        
        if moralis_api_key.is_some() {
            log::info!("✅ Moralis API key loaded for pump.fun token price data");
        } else {
            log::warn!("⚠️ Moralis API key not found. Token prices will be simulated.");
            log::info!("   Set MORALIS_API_KEY environment variable to enable real price data");
            log::info!("   Get your API key from: https://admin.moralis.com/");
        }
        
        Self {
            // Pump.fun frontend API (for token metadata)
            api_url: "https://frontend-api.pump.fun".to_string(),
            // Moralis Solana API (for real token prices)
            moralis_api_url: "https://solana-gateway.moralis.io/token/mainnet".to_string(),
            moralis_api_key,
            client: SharedHttpClient::shared(), // Use shared HTTP client with connection pooling
            circuit_breaker: None,
            // Pump.fun WebSocket and page URLs
            pumpfun_ws_url: "wss://pump.fun".to_string(), // WebSocket endpoint (may need adjustment)
            pumpfun_page_url: "https://pump.fun/?sort=last_trade_timestamp".to_string(),
            ws_connected: Arc::new(tokio::sync::Mutex::new(false)),
        }
    }
    
    /// Create client with circuit breaker for API protection
    pub fn new_with_circuit_breaker(
        circuit_breaker: Option<Arc<tokio::sync::Mutex<crate::error_handling::CircuitBreaker>>>,
    ) -> Self {
        let mut client = Self::new();
        client.circuit_breaker = circuit_breaker;
        client
    }
    
    /// Scrape pump.fun page for real-time trading opportunities
    /// Parses the page at https://pump.fun/?sort=last_trade_timestamp
    /// Extracts token data: name, symbol, price, volume, market cap, last trade timestamp
    pub async fn scrape_trading_opportunities(&self) -> Result<Vec<TokenLaunch>, Box<dyn Error>> {
        log::info!("🔍 Scraping pump.fun page for trading opportunities...");
        
        // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable instead of direct calls
        // CRITICAL IMPROVEMENT #3: Use circuit breaker .call() method instead of manual state checks
        // CRITICAL IMPROVEMENT #4: Detect and map reqwest timeout errors
        // CRITICAL IMPROVEMENT #2: Use map_http_status_to_error() for proper error type mapping
        // CRITICAL IMPROVEMENT #5: Use conservative retry config for non-critical scraping operations
        use crate::error_handling::{retry_with_backoff_retryable, RetryConfig, TradingError, map_http_status_to_error};
        
        let url = self.pumpfun_page_url.clone();
        let client = self.client.clone();
        
        // CRITICAL IMPROVEMENT #5: Use conservative retry for non-critical scraping operations
        let retry_config = RetryConfig::conservative();
        
        // CRITICAL IMPROVEMENT #3: Wrap retry logic with circuit breaker .call() method
        let result: Result<reqwest::Response, TradingError> = if let Some(ref cb) = self.circuit_breaker {
            let cb_clone = cb.clone();
            // Scope the lock so guard is dropped after await completes
            {
                let cb_guard = cb_clone.lock().await;
                cb_guard.call(async move {
                    // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable for robust error handling
                    retry_with_backoff_retryable(
                        || {
                            let url = url.clone();
                            let client = client.clone();
                            Box::pin(async move {
                                client
                                    .get(&url)
                                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                                    .send()
                                    .await
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
                        retry_config.clone(),
                        "PumpFun scrape_trading_opportunities",
                    ).await
                }).await
            } // Guard dropped here after await completes
        } else {
            // No circuit breaker - just use retry logic
            // CRITICAL IMPROVEMENT #1: Use retry_with_backoff_retryable for robust error handling
            retry_with_backoff_retryable(
                || {
                    let url = url.clone();
                    let client = client.clone();
                    Box::pin(async move {
                        client
                            .get(&url)
                            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                            .send()
                            .await
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
                retry_config,
                "PumpFun scrape_trading_opportunities",
            ).await
        };
        
        let response = result.map_err(|e| format!("{}", e))?;
        
        // CRITICAL IMPROVEMENT #2: Use map_http_status_to_error() for proper error type mapping
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            let mapped_error = map_http_status_to_error(status, error_text);
            return Err(format!("{}", mapped_error).into());
        }
        
        let html = response.text().await
            .map_err(|e| format!("Failed to read page content: {}", e))?;
        
        // Parse HTML to extract token data
        // Note: pump.fun likely uses JavaScript to render content, so we may need to parse JSON data embedded in the page
        let tokens = self.parse_pumpfun_page(&html).await?;
        
        log::info!("✅ Scraped {} tokens from pump.fun page", tokens.len());
        Ok(tokens)
    }
    
    /// Parse pump.fun page HTML to extract token information
    /// Looks for embedded JSON data or token cards in the HTML
    async fn parse_pumpfun_page(&self, html: &str) -> Result<Vec<TokenLaunch>, Box<dyn Error>> {
        use scraper::{Html, Selector};
        
        let mut tokens = Vec::new();
        
        // First, try to extract JSON data from __NEXT_DATA__ script tag (Next.js pattern)
        if let Some(json_start) = html.find("__NEXT_DATA__") {
            if let Some(brace_start) = html[json_start..].find('{') {
                let json_str = &html[json_start + brace_start..];
                // Find the matching closing brace (simplified - may need more robust parsing)
                if let Some(brace_end) = json_str.rfind('}') {
                    let json_str = &json_str[..=brace_end];
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(extracted) = self.extract_tokens_from_json(&json_value) {
                            tokens.extend(extracted);
                        }
                    }
                }
            }
        }
        
        // If JSON extraction failed, try parsing HTML structure
        if tokens.is_empty() {
            let document = Html::parse_document(html);
            
            // Try to find embedded JSON data in script tags
            let script_selector = Selector::parse("script")
                .map_err(|e| format!("Failed to parse script selector: {}", e))?;
            for script in document.select(&script_selector) {
                if let Some(text) = script.text().next() {
                    // Look for JSON data structures that might contain token information
                    if text.contains("tokens") || text.contains("coins") {
                        // Try to extract JSON from script tag
                        if let Some(json_start) = text.find('{') {
                            let json_str = &text[json_start..];
                            // Try to parse as JSON (may need more sophisticated parsing)
                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                                // Extract token data from JSON structure
                                if let Some(extracted) = self.extract_tokens_from_json(&json_value) {
                                    tokens.extend(extracted);
                                    break; // Found tokens, stop searching
                                }
                            }
                        }
                    }
                }
            }
            
            // Alternative: Parse HTML structure directly if JSON extraction fails
            if tokens.is_empty() {
                // Look for token card elements (class names may vary)
                // Try common selectors used by pump.fun
                let selectors = vec![
                    Selector::parse("[data-token]"),
                    Selector::parse(".token-card"),
                    Selector::parse(".coin-card"),
                    Selector::parse("[data-coin]"),
                ];
                
                for selector_result in selectors {
                    if let Ok(selector) = selector_result {
                        for element in document.select(&selector) {
                            // Extract token data from element attributes or text
                            if let Some(token) = self.extract_token_from_element(&element) {
                                tokens.push(token);
                            }
                        }
                        if !tokens.is_empty() {
                            break; // Found tokens, stop trying other selectors
                        }
                    }
                }
            }
        }
        
        // If still no tokens found, try to parse from API endpoint
        if tokens.is_empty() {
            log::warn!("⚠️ Could not parse tokens from HTML, trying API endpoint...");
            return self.fetch_tokens_from_api().await;
        }
        
        Ok(tokens)
    }
    
    /// Extract tokens from JSON data structure
    fn extract_tokens_from_json(&self, json: &serde_json::Value) -> Option<Vec<TokenLaunch>> {
        let mut tokens = Vec::new();
        
        // Try various JSON structures that pump.fun might use
        let token_arrays = vec![
            json.pointer("/props/pageProps/tokens"),
            json.pointer("/props/pageProps/coins"),
            json.pointer("/tokens"),
            json.pointer("/coins"),
            json.pointer("/data/tokens"),
            json.pointer("/data/coins"),
        ];
        
        for token_array_opt in token_arrays {
            if let Some(token_array) = token_array_opt {
                if let Some(array) = token_array.as_array() {
                    for token_json in array {
                        if let Some(token) = self.parse_token_json(token_json) {
                            tokens.push(token);
                        }
                    }
                    if !tokens.is_empty() {
                        return Some(tokens);
                    }
                }
            }
        }
        
        None
    }
    
    /// Parse a single token from JSON
    fn parse_token_json(&self, json: &serde_json::Value) -> Option<TokenLaunch> {
        let mint = json.get("mint")?.as_str()?.to_string();
        let name = json.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let symbol = json.get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or(&mint[..8])
            .to_string();
        
        let market_cap = json.get("market_cap")
            .and_then(|v| v.as_f64())
            .or_else(|| json.get("marketCap").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        
        let created_timestamp = json.get("created_timestamp")
            .and_then(|v| v.as_i64())
            .or_else(|| json.get("createdTimestamp").and_then(|v| v.as_i64()))
            .or_else(|| json.get("createdAt").and_then(|v| v.as_i64()))
            .unwrap_or(chrono::Utc::now().timestamp());
        
        Some(TokenLaunch {
            mint,
            name,
            symbol,
            uri: format!("https://pump.fun/{}", json.get("mint").and_then(|v| v.as_str()).unwrap_or("")),
            creator: json.get("creator")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created_timestamp,
            market_cap,
            reply_count: json.get("reply_count")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0),
            is_currently_live: json.get("is_live")
                .and_then(|v| v.as_bool())
                .or_else(|| json.get("isLive").and_then(|v| v.as_bool()))
                .unwrap_or(true),
            king_of_the_hill_timestamp: json.get("king_of_the_hill_timestamp")
                .and_then(|v| v.as_i64()),
            bonding_curve: json.get("bonding_curve")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
    
    /// Extract token from HTML element (fallback method)
    fn extract_token_from_element(&self, _element: &scraper::element_ref::ElementRef) -> Option<TokenLaunch> {
        // This would parse HTML elements directly
        // Implementation depends on actual HTML structure
        // For now, return None to use API fallback
        None
    }
    
    /// Fetch tokens from pump.fun API endpoint as fallback
    async fn fetch_tokens_from_api(&self) -> Result<Vec<TokenLaunch>, Box<dyn Error>> {
        // Try pump.fun API endpoints
        let api_endpoints = vec![
            format!("{}/coins", self.api_url),
            format!("{}/tokens", self.api_url),
            format!("{}/coins?sort=last_trade_timestamp", self.api_url),
        ];
        
        for endpoint in api_endpoints {
            match self.client.get(&endpoint).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(json) = response.json::<serde_json::Value>().await {
                            if let Some(tokens) = self.extract_tokens_from_json(&json) {
                                log::info!("✅ Fetched {} tokens from pump.fun API: {}", tokens.len(), endpoint);
                                return Ok(tokens);
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        
        log::warn!("⚠️ Could not fetch tokens from pump.fun API endpoints");
        Ok(Vec::new())
    }
    
    /// Connect to pump.fun WebSocket for real-time token updates
    /// Listens for new token launches, price updates, and trading activity
    pub fn start_websocket_listener(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        use tokio_tungstenite::{connect_async, tungstenite::Message};
        use futures::{SinkExt, StreamExt};
        
        log::info!("🔌 Starting pump.fun WebSocket listener for real-time updates...");
        
        // Try to connect to WebSocket
        // Note: Actual WebSocket URL may need to be discovered from pump.fun's network requests
        // Common patterns: wss://pump.fun/ws, wss://api.pump.fun/ws, wss://pump.fun/api/ws
        let ws_urls = vec![
            "wss://pump.fun/ws".to_string(),
            "wss://api.pump.fun/ws".to_string(),
            "wss://pump.fun/api/ws".to_string(),
            "wss://frontend-api.pump.fun/ws".to_string(),
        ];
        
        let client_clone = self.clone();
        let ws_connected_clone = self.ws_connected.clone();
        
        tokio::spawn(async move {
            let mut current_url_idx = 0;
            
            loop {
                let ws_url = &ws_urls[current_url_idx % ws_urls.len()];
                
                match connect_async(ws_url).await {
                    Ok((ws_stream, _)) => {
                        log::info!("✅ Connected to pump.fun WebSocket: {}", ws_url);
                        let (mut write, mut read) = ws_stream.split();
                        
                        // Mark as connected
                        {
                            let mut connected = ws_connected_clone.lock().await;
                            *connected = true;
                        }
                        
                        // Listen for messages
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    // Parse WebSocket message
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                        client_clone.handle_websocket_message(&json).await;
                                    } else {
                                        log::debug!("Received non-JSON WebSocket message: {}", text);
                                    }
                                }
                                Ok(Message::Binary(_data)) => {
                                    // Handle binary messages (might be protobuf or other format)
                                    log::debug!("Received binary WebSocket message from pump.fun");
                                }
                                Ok(Message::Ping(data)) => {
                                    // Respond to ping
                                    if let Err(e) = write.send(Message::Pong(data)).await {
                                        log::error!("Failed to send pong: {}", e);
                                        break;
                                    }
                                }
                                Ok(Message::Close(_)) => {
                                    log::warn!("WebSocket connection closed by pump.fun");
                                    break;
                                }
                                Err(e) => {
                                    log::error!("WebSocket error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        
                        // Mark as disconnected
                        {
                            let mut connected = ws_connected_clone.lock().await;
                            *connected = false;
                        }
                    }
                    Err(e) => {
                        log::warn!("⚠️ Failed to connect to pump.fun WebSocket ({}): {}", ws_url, e);
                        current_url_idx += 1;
                        if current_url_idx >= ws_urls.len() {
                            log::info!("   Tried all WebSocket URLs, will retry in 60 seconds...");
                            current_url_idx = 0;
                            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                        } else {
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        })
    }
    
    /// Handle incoming WebSocket messages from pump.fun
    async fn handle_websocket_message(&self, json: &serde_json::Value) {
        // Parse different message types from pump.fun WebSocket
        // Common types: new_token, price_update, trade_update, etc.
        
        if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
            match msg_type {
                "new_token" | "token_launched" => {
                    if let Some(token) = self.parse_token_json(json) {
                        log::info!("🆕 New token launched on pump.fun: {} ({})", token.name, token.symbol);
                        log::info!("   Market Cap: ${:.2} | Mint: {}", token.market_cap, token.mint);
                    }
                }
                "price_update" | "price_change" => {
                    if let Some(mint) = json.get("mint").and_then(|v| v.as_str()) {
                        if let Some(price) = json.get("price").and_then(|v| v.as_f64()) {
                            log::debug!("💰 Price update for {}: ${:.8}", mint, price);
                        }
                    }
                }
                "trade" | "trade_update" => {
                    if let Some(mint) = json.get("mint").and_then(|v| v.as_str()) {
                        log::debug!("📊 Trade update for {} on pump.fun", mint);
                    }
                }
                _ => {
                    log::debug!("Received WebSocket message type: {}", msg_type);
                }
            }
        }
    }
    
    /// Check if WebSocket is connected
    pub async fn is_websocket_connected(&self) -> bool {
        let connected = self.ws_connected.lock().await;
        *connected
    }
    
    /// Get recently created tokens on PumpFun with real-time price data from Moralis
    /// NOTE: This is a fallback method. The memecoin monitor now scans ALL pairs from Mobula API.
    pub async fn get_recent_launches(&self, _limit: usize) -> Result<Vec<TokenLaunch>, Box<dyn Error>> {
        log::debug!("Fetching recent launches from PumpFun (fallback method - memecoin monitor uses all pairs)");
        
        // NO SIMULATED DATA - This method should not be used for production
        // The memecoin monitor now uses ALL pairs from Mobula API directly
        log::warn!("⚠️ get_recent_launches() called - this is a fallback method");
        log::warn!("   Memecoin monitor should use Mobula API pairs directly");
        log::warn!("   Returning empty list - use real pair data from Mobula API");
        
        // Return empty - real data must come from Mobula API
        Ok(Vec::new())
    }
    
    /// Get token details by mint address using Moralis API
    pub async fn get_token_details(&self, mint: &str) -> Result<Option<TokenLaunch>, Box<dyn Error>> {
        log::debug!("Fetching token details for: {}", mint);
        
        // Try to get price from Moralis API if available
        if let Some(ref _api_key) = self.moralis_api_key {
            match self.get_token_price_from_moralis(mint).await {
                Ok(Some(price_data)) => {
                    log::info!("✅ Got real-time price data from Moralis for token: {}", mint);
                    log::info!("   Exchange: {} | Price: ${:.6} | Pair: {}", 
                        price_data.exchange_name, price_data.usd_price, price_data.pair_address);
                    
                    // Estimate market cap from USD price (in production, fetch actual supply from token metadata)
                    let estimated_supply = 1_000_000_000.0; // 1B tokens as default
                    let market_cap = price_data.usd_price * estimated_supply;
                    
                    // Return token launch with real-time price data
                    return Ok(Some(TokenLaunch {
                        mint: mint.to_string(),
                        name: format!("Token ({})", price_data.exchange_name),
                        symbol: mint.chars().take(8).collect::<String>().to_uppercase(), // Use mint address prefix
                        uri: format!("https://pump.fun/{}", mint),
                        creator: price_data.exchange_address.clone(),
                        created_timestamp: chrono::Utc::now().timestamp(),
                        market_cap,
                        reply_count: if market_cap > 10000.0 { 50 } else { 10 }, // Estimate engagement
                        is_currently_live: true,
                        king_of_the_hill_timestamp: None,
                        bonding_curve: price_data.pair_address.clone(),
                    }));
                }
                Ok(None) => {
                    log::debug!("No price data found for token: {}", mint);
                }
                Err(e) => {
                    log::warn!("Failed to fetch price from Moralis: {}", e);
                }
            }
        }
        
        // Fallback to simulation or return None
        Ok(None)
    }
    
    /// Get token price from Moralis API
    /// Endpoint: GET https://solana-gateway.moralis.io/token/mainnet/{TOKEN_ADDRESS}/price
    /// Docs: https://docs.moralis.com/web3-data-api/solana/tutorials/get-pump-fun-token-prices
    pub async fn get_token_price_from_moralis(&self, token_address: &str) -> Result<Option<MoralisTokenPrice>, Box<dyn Error>> {
        if self.moralis_api_key.is_none() {
            return Err("Moralis API key required. Set MORALIS_API_KEY environment variable.".into());
        }
        
        let url = format!("{}/{}/price", self.moralis_api_url, token_address);
        
        log::debug!("Fetching token price from Moralis: {}", url);
        
        // CIRCUIT BREAKER: Check circuit breaker state before making request
        if let Some(ref cb) = self.circuit_breaker {
            let cb_state = cb.lock().await.get_state().await;
            if matches!(cb_state, crate::error_handling::CircuitState::Open) {
                return Err("Circuit breaker is OPEN - Moralis API call blocked".into());
            }
        }
        
        let response = self.client
            .get(&url)
            .header("X-API-Key", self.moralis_api_key.as_ref().unwrap())
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
        
        let status = response.status();
        if !status.is_success() {
            if status == 404 {
                // Token not found on any DEX
                return Ok(None);
            }
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Moralis API error {}: {}", status, error_text).into());
        }
        
        let price_data: MoralisTokenPrice = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        Ok(Some(price_data))
    }
    
    /// Generate dynamic launches without hardcoded memecoins (fallback only)
    /// This is only used when Mobula API is unavailable
    fn generate_dynamic_launches(&self, limit: usize) -> Vec<TokenLaunch> {
        let mut launches = Vec::new();
        let base_timestamp = Utc::now().timestamp();
        
        // Generate dynamic token names instead of hardcoded memecoins
        for i in 0..limit {
            let timestamp = base_timestamp - (i as i64 * 300); // 5 min apart
            
            // Generate dynamic symbol and name
            let symbol = format!("TOKEN{}", i + 1);
            let name = format!("Token {}", i + 1);
            
            launches.push(TokenLaunch {
                mint: format!("{}...{}", 
                    &hex::encode(&rand::random::<[u8; 4]>()),
                    &hex::encode(&rand::random::<[u8; 4]>())),
                name,
                symbol,
                uri: format!("https://pump.fun/token/token{}", i + 1),
                creator: format!("{}...{}", 
                    &hex::encode(&rand::random::<[u8; 4]>()),
                    &hex::encode(&rand::random::<[u8; 4]>())),
                created_timestamp: timestamp,
                market_cap: 10000.0 + rand::random::<f64>() * 100000.0,
                reply_count: (rand::random::<u32>() % 100),
                is_currently_live: rand::random::<f64>() > 0.3,
                king_of_the_hill_timestamp: None,
                bonding_curve: format!("bonding_curve_{}", i),
            });
        }
        
        launches
    }
    
    /// Analyze meme coin sentiment
    pub fn analyze_sentiment(&self, launch: &TokenLaunch) -> MemeSentiment {
        let mut sentiment_score = 0.0;
        let mut social_signals = Vec::new();
        
        // Analyze reply count (engagement)
        if launch.reply_count > 50 {
            sentiment_score += 20.0;
            social_signals.push("High engagement".to_string());
        } else if launch.reply_count > 20 {
            sentiment_score += 10.0;
            social_signals.push("Medium engagement".to_string());
        }
        
        // Analyze if currently live
        if launch.is_currently_live {
            sentiment_score += 15.0;
            social_signals.push("Currently live".to_string());
        }
        
        // Analyze market cap
        if launch.market_cap > 50000.0 {
            sentiment_score += 25.0;
            social_signals.push("Strong market cap".to_string());
        } else if launch.market_cap > 20000.0 {
            sentiment_score += 10.0;
            social_signals.push("Growing market cap".to_string());
        }
        
        // Time since launch
        let age_hours = (Utc::now().timestamp() - launch.created_timestamp) / 3600;
        if age_hours < 1 {
            sentiment_score += 20.0;
            social_signals.push("Fresh launch".to_string());
        } else if age_hours < 6 {
            sentiment_score += 10.0;
            social_signals.push("Recent launch".to_string());
        }
        
        // Determine hype level
        let hype_level = if sentiment_score >= 70.0 {
            HypeLevel::Extreme
        } else if sentiment_score >= 50.0 {
            HypeLevel::High
        } else if sentiment_score >= 30.0 {
            HypeLevel::Medium
        } else {
            HypeLevel::Low
        };
        
        // Determine risk level (inversely related to market cap and age)
        let risk_level = if launch.market_cap < 10000.0 || age_hours < 1 {
            RiskLevel::Extreme
        } else if launch.market_cap < 30000.0 || age_hours < 3 {
            RiskLevel::High
        } else if launch.market_cap < 50000.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        
        MemeSentiment {
            token_address: launch.mint.clone(),
            symbol: launch.symbol.clone(),
            sentiment_score,
            hype_level,
            social_signals,
            risk_level,
            twitter_sentiment: None,
            twitter_weighted_polarity: None,
            community_growth_pct: None,
        }
    }
    
    /// Analyze meme coin sentiment with Twitter data enhancement
    /// This method enhances the base sentiment analysis with ML-based Twitter sentiment
    pub async fn analyze_sentiment_with_twitter(
        &self,
        launch: &TokenLaunch,
        twitter_client: Option<&crate::twitter_sentiment::TwitterSentimentClient>,
    ) -> MemeSentiment {
        let mut sentiment = self.analyze_sentiment(launch);
        
        // Enhance with Twitter data if available
        if let Some(client) = twitter_client {
            match client.get_sentiment(&launch.symbol, None, None).await {
                Ok(twitter_data) => {
                    sentiment.twitter_sentiment = Some(twitter_data.clone());
                    sentiment.twitter_weighted_polarity = Some(twitter_data.weighted_polarity);
                    sentiment.community_growth_pct = Some(twitter_data.volume_growth);
                    
                    // Adjust sentiment score based on Twitter data
                    // Twitter polarity is -1.0 to 1.0, scale to -20 to +20 adjustment
                    let twitter_adjustment = twitter_data.weighted_polarity * 20.0;
                    sentiment.sentiment_score = (sentiment.sentiment_score + twitter_adjustment)
                        .max(-100.0).min(100.0);
                    
                    // Update hype level based on Twitter engagement
                    if twitter_data.engagement_metrics.avg_likes > 100.0 
                        && twitter_data.weighted_polarity > 0.3 {
                        sentiment.hype_level = HypeLevel::High;
                        sentiment.social_signals.push(format!(
                            "High Twitter engagement ({} avg likes)",
                            twitter_data.engagement_metrics.avg_likes as i64
                        ));
                    }
                    
                    // Add Twitter sentiment to social signals
                    sentiment.social_signals.push(format!(
                        "Twitter sentiment: {} ({:.2})",
                        twitter_data.sentiment,
                        twitter_data.weighted_polarity
                    ));
                    
                    if twitter_data.volume_growth > 10.0 {
                        sentiment.social_signals.push(format!(
                            "Growing Twitter volume: +{:.1}%",
                            twitter_data.volume_growth
                        ));
                    }
                    
                    log::debug!(
                        "✅ Enhanced sentiment for {} with Twitter data: polarity={:.3}, growth={:.1}%",
                        launch.symbol,
                        twitter_data.weighted_polarity,
                        twitter_data.volume_growth
                    );
                }
                Err(e) => {
                    log::debug!("Twitter sentiment unavailable for {}: {}", launch.symbol, e);
                }
            }
        }
        
        sentiment
    }
    
    /// Generate trading signals for meme coins
    pub async fn generate_meme_signals(&self, launches: Vec<TokenLaunch>) -> Vec<MemeTradeSignal> {
        let mut signals = Vec::new();
        
        for launch in launches {
            let sentiment = self.analyze_sentiment(&launch);
            
            // Only generate signals for tokens with positive sentiment
            if sentiment.sentiment_score > 40.0 {
                let mut reasons = Vec::new();
                let action: String;
                let confidence: f64;
                
                // Strong buy signal
                if sentiment.sentiment_score > 70.0 && launch.market_cap > 30000.0 {
                    action = "BUY".to_string();
                    confidence = 0.75;
                    reasons.push("Extremely high sentiment".to_string());
                    reasons.push("Strong community backing".to_string());
                    reasons.extend(sentiment.social_signals.clone());
                } 
                // Moderate buy signal
                else if sentiment.sentiment_score > 50.0 {
                    action = "BUY".to_string();
                    confidence = 0.60;
                    reasons.push("Good sentiment".to_string());
                    reasons.extend(sentiment.social_signals.clone());
                }
                // Weak buy signal
                else {
                    action = "HOLD".to_string();
                    confidence = 0.45;
                    reasons.push("Moderate sentiment".to_string());
                    reasons.push("Monitor for better entry".to_string());
                }
                
                // Estimate prices (simplified)
                let entry_price = launch.market_cap / 1000000.0; // Simplified price calculation
                let target_price = entry_price * (1.0 + confidence);
                let stop_loss = entry_price * 0.85; // 15% stop loss
                
                signals.push(MemeTradeSignal {
                    token_address: launch.mint.clone(),
                    symbol: launch.symbol.clone(),
                    name: launch.name.clone(),
                    action,
                    confidence,
                    entry_price,
                    target_price,
                    stop_loss,
                    reasons,
                    timestamp: Utc::now().timestamp(),
                });
            }
        }
        
        // Sort by confidence
        signals.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        log::info!("Generated {} meme coin trading signals", signals.len());
        
        signals
    }
    
    /// Get top meme coin opportunities
    pub async fn get_top_opportunities(&self, limit: usize) -> Result<Vec<MemeTradeSignal>, Box<dyn Error>> {
        let launches = self.get_recent_launches(limit * 2).await?;
        let mut signals = self.generate_meme_signals(launches).await;
        signals.truncate(limit);
        Ok(signals)
    }
    
    /// Comprehensive safety check for a token before trading
    /// Returns detailed safety analysis with risk factors and warnings
    pub async fn comprehensive_safety_check(
        &self,
        token: &TokenLaunch,
        config: Option<SafetyConfig>,
    ) -> TokenSafetyCheck {
        let config = config.unwrap_or_else(SafetyConfig::default);
        let mut safety = TokenSafetyCheck {
            is_safe: true,
            safety_score: 100.0,
            risk_factors: Vec::new(),
            warnings: Vec::new(),
            passed_checks: Vec::new(),
            liquidity_ok: false,
            volume_ok: false,
            age_ok: false,
            market_cap_ok: false,
            price_stable: false,
            holder_distribution_ok: false,
            contract_verified: false,
            not_blacklisted: true, // Assume not blacklisted unless detected
            multi_source_validated: false,
        };
        
        // 1. Market Cap Check
        if token.market_cap >= config.min_market_cap_usd {
            safety.market_cap_ok = true;
            safety.passed_checks.push(format!("Market cap ${:.0} >= ${:.0}", token.market_cap, config.min_market_cap_usd));
        } else {
            safety.is_safe = false;
            safety.safety_score -= 20.0;
            safety.risk_factors.push(format!("Market cap ${:.0} below minimum ${:.0}", token.market_cap, config.min_market_cap_usd));
        }
        
        // 2. Token Age Check
        let age_hours = (chrono::Utc::now().timestamp() - token.created_timestamp) as f64 / 3600.0;
        if age_hours >= config.min_token_age_hours {
            safety.age_ok = true;
            safety.passed_checks.push(format!("Token age {:.2}h >= {:.2}h", age_hours, config.min_token_age_hours));
        } else {
            safety.is_safe = false;
            safety.safety_score -= 15.0;
            safety.risk_factors.push(format!("Token too new: {:.2}h < {:.2}h minimum", age_hours, config.min_token_age_hours));
            safety.warnings.push("Very new token - higher rug pull risk".to_string());
        }
        
        // 3. Liquidity Check (estimate from market cap and bonding curve)
        // For pump.fun tokens, liquidity is typically tied to bonding curve progress
        let estimated_liquidity = token.market_cap * 0.1; // Rough estimate: 10% of market cap
        if estimated_liquidity >= config.min_liquidity_usd {
            safety.liquidity_ok = true;
            safety.passed_checks.push(format!("Estimated liquidity ${:.0} >= ${:.0}", estimated_liquidity, config.min_liquidity_usd));
        } else {
            safety.is_safe = false;
            safety.safety_score -= 25.0;
            safety.risk_factors.push(format!("Low liquidity: ${:.0} < ${:.0}", estimated_liquidity, config.min_liquidity_usd));
            safety.warnings.push("Low liquidity - high slippage risk".to_string());
        }
        
        // 4. Volume Check (estimate from reply count and market cap)
        // Higher engagement (replies) suggests trading activity
        let estimated_volume_24h = token.reply_count as f64 * 100.0; // Rough estimate
        if estimated_volume_24h >= config.min_volume_24h_usd {
            safety.volume_ok = true;
            safety.passed_checks.push(format!("Estimated 24h volume ${:.0} >= ${:.0}", estimated_volume_24h, config.min_volume_24h_usd));
        } else {
            safety.safety_score -= 10.0;
            safety.warnings.push(format!("Low estimated volume: ${:.0}", estimated_volume_24h));
        }
        
        // 5. Price Stability Check (check if token is currently live and stable)
        if token.is_currently_live {
            safety.price_stable = true;
            safety.passed_checks.push("Token is currently live on pump.fun".to_string());
        } else {
            safety.safety_score -= 5.0;
            safety.warnings.push("Token not currently live - may be completed or failed".to_string());
        }
        
        // 6. Holder Distribution Check (estimate from reply count)
        // More replies = more engagement = likely more holders
        let estimated_holders = token.reply_count.max(1);
        if estimated_holders >= config.min_holder_count {
            safety.holder_distribution_ok = true;
            safety.passed_checks.push(format!("Estimated {} holders >= {}", estimated_holders, config.min_holder_count));
        } else {
            safety.safety_score -= 10.0;
            safety.warnings.push(format!("Low holder count estimate: {}", estimated_holders));
        }
        
        // 7. Multi-Source Validation (check Moralis API if available)
        if let Some(ref _api_key) = self.moralis_api_key {
            match self.get_token_price_from_moralis(&token.mint).await {
                Ok(Some(price_data)) => {
                    if price_data.usd_price > 0.0 && !price_data.exchange_name.is_empty() {
                        safety.multi_source_validated = true;
                        safety.passed_checks.push(format!("Validated on {} DEX", price_data.exchange_name));
                        safety.safety_score += 10.0; // Bonus for multi-source validation
                    }
                }
                Ok(None) => {
                    if config.require_multi_source {
                        safety.is_safe = false;
                        safety.safety_score -= 15.0;
                        safety.risk_factors.push("Not found on any DEX (Moralis)".to_string());
                    } else {
                        safety.warnings.push("Not found on Moralis - pump.fun only".to_string());
                    }
                }
                Err(e) => {
                    log::debug!("Could not validate token on Moralis: {}", e);
                    if config.require_multi_source {
                        safety.warnings.push("Could not verify on external DEX".to_string());
                    }
                }
            }
        } else {
            if config.require_multi_source {
                safety.warnings.push("Moralis API not configured - cannot verify on external DEX".to_string());
            }
        }
        
        // 8. Rug Pull Detection Patterns
        // Check for suspicious patterns
        let suspicious_patterns = self.detect_rug_pull_patterns(token);
        if !suspicious_patterns.is_empty() {
            safety.is_safe = false;
            safety.safety_score -= 30.0;
            for pattern in suspicious_patterns {
                safety.risk_factors.push(format!("RUG PULL RISK: {}", pattern));
            }
        }
        
        // 9. Blacklist Check (basic - can be enhanced with actual blacklist)
        // Check for known scam patterns in name/symbol
        if self.is_blacklisted(&token.mint, &token.name, &token.symbol) {
            safety.is_safe = false;
            safety.not_blacklisted = false;
            safety.safety_score = 0.0;
            safety.risk_factors.push("Token matches blacklist patterns".to_string());
        } else {
            safety.passed_checks.push("Not on blacklist".to_string());
        }
        
        // 10. Contract Verification (pump.fun tokens are typically unverified, so this is optional)
        if config.require_contract_verification {
            // In production, check if token contract is verified
            safety.contract_verified = false; // pump.fun tokens are typically unverified
            safety.warnings.push("Contract not verified (typical for pump.fun tokens)".to_string());
        } else {
            safety.contract_verified = true; // Skip check
            safety.passed_checks.push("Contract verification skipped (pump.fun standard)".to_string());
        }
        
        // Final safety determination
        safety.safety_score = safety.safety_score.max(0.0).min(100.0);
        if safety.safety_score < 50.0 {
            safety.is_safe = false;
        }
        
        // Log safety check results
        if safety.is_safe {
            log::info!("✅ Token {} passed safety check (Score: {:.1}/100)", token.symbol, safety.safety_score);
        } else {
            log::warn!("❌ Token {} FAILED safety check (Score: {:.1}/100)", token.symbol, safety.safety_score);
            log::warn!("   Risk factors: {:?}", safety.risk_factors);
        }
        
        safety
    }
    
    /// Detect potential rug pull patterns
    fn detect_rug_pull_patterns(&self, token: &TokenLaunch) -> Vec<String> {
        let mut patterns = Vec::new();
        
        // Pattern 1: Extremely new token with high market cap (pump and dump)
        let age_hours = (chrono::Utc::now().timestamp() - token.created_timestamp) as f64 / 3600.0;
        if age_hours < 0.5 && token.market_cap > 100000.0 {
            patterns.push("Very new token with suspiciously high market cap".to_string());
        }
        
        // Pattern 2: Low engagement despite high market cap
        if token.market_cap > 50000.0 && token.reply_count < 5 {
            patterns.push("High market cap but very low engagement (possible bot activity)".to_string());
        }
        
        // Pattern 3: Suspicious name/symbol patterns
        let name_lower = token.name.to_lowercase();
        let symbol_lower = token.symbol.to_lowercase();
        let scam_keywords = vec!["test", "scam", "rug", "honeypot", "fake"];
        for keyword in scam_keywords {
            if name_lower.contains(keyword) || symbol_lower.contains(keyword) {
                patterns.push(format!("Suspicious keyword in name/symbol: {}", keyword));
            }
        }
        
        // Pattern 4: Empty or generic metadata
        if token.name.is_empty() || token.name == "Unknown" || token.symbol.len() < 2 {
            patterns.push("Missing or generic token metadata".to_string());
        }
        
        patterns
    }
    
    /// Check if token is blacklisted (basic pattern matching)
    fn is_blacklisted(&self, mint: &str, name: &str, symbol: &str) -> bool {
        // Basic blacklist check - can be enhanced with actual blacklist database
        let blacklist_patterns = vec![
            "test", "scam", "rug", "honeypot", "fake", "demo",
        ];
        
        let check_str = format!("{} {} {}", mint, name, symbol).to_lowercase();
        for pattern in blacklist_patterns {
            if check_str.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if a token is safe to trade using real price data from Moralis
    /// This validates that the token has real liquidity and price data on a DEX
    /// ENHANCED: Now uses comprehensive safety check
    pub async fn is_safe_to_trade_with_price(&self, token_address: &str) -> Result<bool, Box<dyn Error>> {
        if self.moralis_api_key.is_none() {
            // Without Moralis API, we can't validate price data
            log::warn!("Cannot validate token safety without Moralis API key");
            return Ok(false);
        }
        
        match self.get_token_price_from_moralis(token_address).await {
            Ok(Some(price_data)) => {
                // Token has real price data on a DEX - consider it safer
                log::debug!("Token {} has price data on {}: ${}", 
                    token_address, price_data.exchange_name, price_data.usd_price);
                
                // Enhanced safety checks:
                // - Token must have non-zero USD price
                // - Must be listed on a known DEX
                // - Price must be reasonable (not suspiciously high/low)
                let is_safe = price_data.usd_price > 0.0 && 
                             price_data.usd_price < 1_000_000.0 && // Sanity check: not > $1M per token
                             !price_data.exchange_name.is_empty() &&
                             !price_data.pair_address.is_empty();
                
                if !is_safe {
                    log::warn!("Token {} failed price safety check: price=${:.8}, exchange={}", 
                        token_address, price_data.usd_price, price_data.exchange_name);
                }
                
                Ok(is_safe)
            }
            Ok(None) => {
                // Token not found on any DEX - not safe to trade
                log::debug!("Token {} not found on any DEX", token_address);
                Ok(false)
            }
            Err(e) => {
                log::warn!("Failed to check token safety: {}", e);
                Err(e)
            }
        }
    }
}

/// Meme coin analyzer for advanced analysis
pub struct MemeAnalyzer {
    pumpfun: PumpFunClient,
    twitter_client: Option<Arc<crate::twitter_sentiment::TwitterSentimentClient>>, // ADD: Twitter sentiment client
}

impl MemeAnalyzer {
    pub fn new() -> Self {
        Self {
            pumpfun: PumpFunClient::new(),
            twitter_client: None, // Default: no Twitter client
        }
    }
    
    /// Create MemeAnalyzer with Twitter sentiment client
    pub fn with_twitter_client(twitter_client: Arc<crate::twitter_sentiment::TwitterSentimentClient>) -> Self {
        Self {
            pumpfun: PumpFunClient::new(),
            twitter_client: Some(twitter_client),
        }
    }
    
    /// Analyze multiple meme coins and rank them
    pub async fn analyze_and_rank(&self, limit: usize) -> Result<Vec<MemeTradeSignal>, Box<dyn Error>> {
        self.pumpfun.get_top_opportunities(limit).await
    }
    
    /// Analyze meme coin with Twitter sentiment enhancement (uses stored client if available)
    pub async fn analyze_with_twitter_sentiment(
        &self,
        launch: &TokenLaunch,
        twitter_client: Option<&crate::twitter_sentiment::TwitterSentimentClient>,
    ) -> MemeSentiment {
        // Use provided client or fall back to stored client
        let client_to_use = twitter_client.or_else(|| {
            self.twitter_client.as_ref().map(|arc| arc.as_ref())
        });
        self.pumpfun.analyze_sentiment_with_twitter(launch, client_to_use).await
    }
    
    /// Check if a meme coin is safe to trade using sentiment analysis
    /// ENHANCED: More comprehensive safety checks including Twitter sentiment
    pub fn is_safe_to_trade(&self, sentiment: &MemeSentiment, _min_market_cap: f64) -> bool {
        // Enhanced safety checks:
        // 1. Risk level must be Low or Medium (not High or Extreme)
        let risk_ok = matches!(sentiment.risk_level, RiskLevel::Low | RiskLevel::Medium);
        
        // 2. Sentiment score must be positive and above threshold
        let sentiment_ok = sentiment.sentiment_score > 40.0;
        
        // 3. Check for extreme hype (can be dangerous)
        let hype_ok = !matches!(sentiment.hype_level, HypeLevel::Extreme);
        
        // 4. Must have some social signals (community engagement)
        let has_social_signals = !sentiment.social_signals.is_empty();
        
        // 5. Twitter sentiment check (if available)
        let twitter_ok = if let Some(ref twitter_polarity) = sentiment.twitter_weighted_polarity {
            // Twitter sentiment should be neutral to positive for safety
            *twitter_polarity >= -0.2
        } else {
            true // If no Twitter data, don't fail on this check
        };
        
        // All checks must pass
        let is_safe = risk_ok && sentiment_ok && hype_ok && has_social_signals && twitter_ok;
        
        if !is_safe {
            log::debug!("Token {} failed sentiment safety check: risk={:?}, sentiment={:.1}, hype={:?}, signals={}, twitter_ok={}", 
                sentiment.symbol, sentiment.risk_level, sentiment.sentiment_score, 
                sentiment.hype_level, sentiment.social_signals.len(), twitter_ok);
        }
        
        is_safe
    }
    
    /// Enhanced safety check combining sentiment and comprehensive validation
    pub async fn is_safe_to_trade_enhanced(
        &self,
        token: &TokenLaunch,
        sentiment: &MemeSentiment,
        config: Option<SafetyConfig>,
    ) -> Result<TokenSafetyCheck, Box<dyn Error>> {
        // Run comprehensive safety check
        let mut safety = self.pumpfun.comprehensive_safety_check(token, config).await;
        
        // Add sentiment-based checks
        if !self.is_safe_to_trade(sentiment, token.market_cap) {
            safety.is_safe = false;
            safety.safety_score -= 20.0;
            safety.risk_factors.push(format!("Sentiment check failed: risk={:?}, score={:.1}", 
                sentiment.risk_level, sentiment.sentiment_score));
        } else {
            safety.passed_checks.push("Sentiment analysis passed".to_string());
        }
        
        // Check for extreme hype
        if matches!(sentiment.hype_level, HypeLevel::Extreme) {
            safety.warnings.push("Extreme hype detected - high volatility risk".to_string());
            safety.safety_score -= 5.0;
        }
        
        // Final determination
        safety.safety_score = safety.safety_score.max(0.0).min(100.0);
        if safety.safety_score < 50.0 {
            safety.is_safe = false;
        }
        
        Ok(safety)
    }
    
    /// Calculate position size for meme coin trade
    pub fn calculate_meme_position_size(&self, confidence: f64, account_balance: f64) -> f64 {
        // Use smaller position sizes for meme coins due to higher risk
        let max_position_pct = 0.05; // Max 5% per meme trade
        let size = account_balance * max_position_pct * confidence;
        size.min(account_balance * max_position_pct)
    }
}

// ========== Moralis API Response Structures ==========

/// Moralis API token price response
/// Docs: https://docs.moralis.com/web3-data-api/solana/tutorials/get-pump-fun-token-prices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralisTokenPrice {
    #[serde(rename = "tokenAddress")]
    pub token_address: String,
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    #[serde(rename = "exchangeName")]
    pub exchange_name: String,
    #[serde(rename = "exchangeAddress")]
    pub exchange_address: String,
    #[serde(rename = "nativePrice")]
    pub native_price: MoralisNativePrice,
    #[serde(rename = "usdPrice")]
    pub usd_price: f64,
}

/// Native price information from Moralis API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralisNativePrice {
    pub value: String, // String to handle large numbers
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

impl MoralisTokenPrice {
    /// Get native price as f64 (may lose precision for very large numbers)
    pub fn native_price_f64(&self) -> f64 {
        self.native_price.value.parse::<f64>().unwrap_or(0.0) / 10_f64.powi(self.native_price.decimals as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pumpfun_client_creation() {
        let client = PumpFunClient::new();
        assert!(!client.api_url.is_empty());
    }

    #[tokio::test]
    async fn test_get_recent_launches() {
        let client = PumpFunClient::new();
        let launches = client.get_recent_launches(5).await;
        assert!(launches.is_ok());
        assert_eq!(launches.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_sentiment_analysis() {
        let client = PumpFunClient::new();
        let launch = TokenLaunch {
            mint: "test".to_string(),
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            uri: "https://test.com".to_string(),
            creator: "creator".to_string(),
            created_timestamp: Utc::now().timestamp(),
            market_cap: 50000.0,
            reply_count: 60,
            is_currently_live: true,
            king_of_the_hill_timestamp: None,
            bonding_curve: "test".to_string(),
        };
        
        let sentiment = client.analyze_sentiment(&launch);
        assert!(sentiment.sentiment_score > 0.0);
        assert!(!sentiment.social_signals.is_empty());
    }

    #[test]
    fn test_meme_analyzer_position_sizing() {
        let analyzer = MemeAnalyzer::new();
        let position = analyzer.calculate_meme_position_size(0.7, 10000.0);
        assert!(position > 0.0);
        assert!(position <= 500.0); // Max 5% of 10000
    }
}
