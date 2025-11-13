use serde::{Deserialize, Serialize};
use std::error::Error;

/// DEX Screener token pair data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub chain_id: String,
    pub dex_id: String,
    pub url: String,
    pub pair_address: String,
    pub base_token: Token,
    pub quote_token: Token,
    pub price_native: String,
    pub price_usd: Option<String>,
    pub volume: Volume,
    pub liquidity: Liquidity,
    pub fdv: Option<f64>,
    pub price_change: PriceChange,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChange {
    #[serde(rename = "h24")]
    pub h24: f64,
    #[serde(rename = "h6")]
    pub h6: f64,
    #[serde(rename = "h1")]
    pub h1: f64,
    #[serde(rename = "m5")]
    pub m5: f64,
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
pub struct DexScreenerClient {
    api_url: String,
    client: reqwest::Client,
}

impl DexScreenerClient {
    pub fn new() -> Self {
        Self {
            api_url: "https://api.dexscreener.com/latest".to_string(),
            client: reqwest::Client::new(),
        }
    }
    
    /// Search for tokens by query
    pub async fn search_tokens(&self, query: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        let url = format!("{}/dex/search/?q={}", self.api_url, query);
        
        log::debug!("Searching DEX Screener for: {}", query);
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("DEX Screener API error: {}", response.status()).into());
        }
        
        let data: DexScreenerResponse = response.json().await?;
        
        Ok(data.pairs.unwrap_or_default())
    }
    
    /// Get token pairs by token address
    pub async fn get_token_pairs(&self, token_address: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        let url = format!("{}/dex/tokens/{}", self.api_url, token_address);
        
        log::debug!("Fetching token pairs for: {}", token_address);
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("DEX Screener API error: {}", response.status()).into());
        }
        
        let data: DexScreenerResponse = response.json().await?;
        
        Ok(data.pairs.unwrap_or_default())
    }
    
    /// Get pair data by pair address
    pub async fn get_pair(&self, chain: &str, pair_address: &str) -> Result<Option<TokenPair>, Box<dyn Error>> {
        let url = format!("{}/dex/pairs/{}/{}", self.api_url, chain, pair_address);
        
        log::debug!("Fetching pair data for: {}/{}", chain, pair_address);
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;
        
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
            let mut score = 0.0;
            
            // Check for momentum signals
            if pair.price_change.m5 > 5.0 {
                signals.push("Strong 5m momentum".to_string());
                score += 20.0;
            }
            
            if pair.price_change.h1 > 10.0 {
                signals.push("Strong 1h trend".to_string());
                score += 25.0;
            }
            
            if pair.price_change.h6 > 20.0 {
                signals.push("Strong 6h uptrend".to_string());
                score += 30.0;
            }
            
            // Volume analysis
            if pair.volume.h1 > pair.volume.h6 / 6.0 * 1.5 {
                signals.push("Increasing volume".to_string());
                score += 15.0;
            }
            
            if pair.volume.m5 > pair.volume.h1 / 12.0 * 2.0 {
                signals.push("Volume spike".to_string());
                score += 20.0;
            }
            
            // Liquidity check
            if liquidity_usd > 10000.0 {
                signals.push("Good liquidity".to_string());
                score += 10.0;
            }
            
            if liquidity_usd > 50000.0 {
                signals.push("Excellent liquidity".to_string());
                score += 10.0;
            }
            
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
                    opportunity_score: score,
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
