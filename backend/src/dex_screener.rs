use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;

/// DEX Screener token pair data (matches official API response)
/// API Docs: https://docs.dexscreener.com/api/reference
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
#[derive(Debug, Deserialize)]
struct DexScreenerResponse {
    #[serde(default)]
    pairs: Option<Vec<TokenPair>>,
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

/// DEX Screener client for token discovery and analysis
/// Official API: https://api.dexscreener.com/latest
/// Rate Limit: 300 requests per minute
pub struct DexScreenerClient {
    api_url: String,
    client: reqwest::Client,
    last_request_time: std::sync::Arc<std::sync::Mutex<SystemTime>>,
    request_count: std::sync::Arc<std::sync::Mutex<u32>>,
}

impl DexScreenerClient {
    pub fn new() -> Self {
        Self {
            api_url: "https://api.dexscreener.com/latest".to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("SolanaTradeBot/1.0")
                .build()
                .unwrap(),
            last_request_time: std::sync::Arc::new(std::sync::Mutex::new(SystemTime::now())),
            request_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }
    
    /// Check and enforce rate limit (300 requests per minute)
    async fn check_rate_limit(&self) -> Result<(), Box<dyn Error>> {
        let wait_time = {
            let mut last_time = self.last_request_time.lock().unwrap();
            let mut count = self.request_count.lock().unwrap();
            
            let now = SystemTime::now();
            let elapsed = now.duration_since(*last_time).unwrap_or(Duration::from_secs(60));
            
            // Reset counter after 1 minute
            if elapsed >= Duration::from_secs(60) {
                *count = 0;
                *last_time = now;
            }
            
            // Check rate limit
            let wait = if *count >= 300 {
                let wait_time = Duration::from_secs(60) - elapsed;
                log::warn!("Rate limit reached, waiting {:?}", wait_time);
                Some(wait_time)
            } else {
                *count += 1;
                None
            };
            
            wait
        }; // MutexGuards are dropped here
        
        // Await outside the mutex lock
        if let Some(wait) = wait_time {
            tokio::time::sleep(wait).await;
            let mut last_time = self.last_request_time.lock().unwrap();
            let mut count = self.request_count.lock().unwrap();
            *count = 0;
            *last_time = SystemTime::now();
        }
        
        Ok(())
    }
    
    /// Search for tokens by query
    /// Endpoint: GET /dex/search/?q={query}
    pub async fn search_tokens(&self, query: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        
        let url = format!("{}/dex/search/?q={}", self.api_url, query);
        
        log::info!("Searching DEX Screener for: {}", query);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("DEX Screener API error {}: {}", status, error_text).into());
        }
        
        let data: DexScreenerResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let pairs = data.pairs.unwrap_or_default();
        log::info!("Found {} pairs for query: {}", pairs.len(), query);
        
        Ok(pairs)
    }
    
    /// Get token pairs by token address (supports multiple addresses)
    /// Endpoint: GET /dex/tokens/{tokenAddresses}
    /// Example: /dex/tokens/0x2170...abc,0x3171...def
    pub async fn get_token_pairs(&self, token_address: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        
        let url = format!("{}/dex/tokens/{}", self.api_url, token_address);
        
        log::info!("Fetching token pairs for: {}", token_address);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("DEX Screener API error {}: {}", status, error_text).into());
        }
        
        let data: DexScreenerResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let pairs = data.pairs.unwrap_or_default();
        log::info!("Found {} pairs for token: {}", pairs.len(), token_address);
        
        Ok(pairs)
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
    
    /// Get pair data by pair address
    /// Endpoint: GET /dex/pairs/{chainId}/{pairAddresses}
    /// Supports multiple pair addresses: /dex/pairs/solana/addr1,addr2
    pub async fn get_pair(&self, chain: &str, pair_address: &str) -> Result<Option<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        
        let url = format!("{}/dex/pairs/{}/{}", self.api_url, chain, pair_address);
        
        log::info!("Fetching pair data for: {}/{}", chain, pair_address);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("DEX Screener API error: {}", response.status()).into());
        }
        
        let data: DexScreenerResponse = response.json().await?;
        
        Ok(data.pairs.and_then(|p| p.into_iter().next()))
    }
    
    /// Find trending tokens on Solana with high volume
    pub async fn find_trending_solana_tokens(&self, min_liquidity_usd: f64) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        // Search for highly active Solana pairs
        let pairs = self.search_tokens("SOL").await?;
        
        // Filter for Solana chain and minimum liquidity
        let trending: Vec<TokenPair> = pairs.into_iter()
            .filter(|p| {
                p.chain_id == "solana" && 
                p.liquidity.usd.unwrap_or(0.0) >= min_liquidity_usd &&
                p.volume.h24 > 0.0
            })
            .collect();
        
        log::info!("Found {} trending Solana tokens", trending.len());
        
        Ok(trending)
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
            momentum_score = (momentum_5m * 40.0 + momentum_1h * 35.0 + momentum_6h * 25.0);
            
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
            volume_score = (vol_ratio_1h * 50.0 + vol_ratio_5m * 50.0);
            
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
        opportunities.sort_by(|a, b| b.opportunity_score.partial_cmp(&a.opportunity_score).unwrap());
        
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
        assert_eq!(client.api_url, "https://api.dexscreener.com/latest");
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
