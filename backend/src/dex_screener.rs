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
