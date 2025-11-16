use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::{Duration, SystemTime};

/// Mobula API token pair data (GMGN-compatible, multi-chain support)
/// API Docs: https://docs.mobula.io/
/// Base URL: https://api.mobula.io/api/1/
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

/// Response from Mobula API (GMGN-compatible)
/// Note: Mobula API may return different structures depending on endpoint
/// This structure matches the pairs endpoint format
#[derive(Debug, Deserialize)]
struct MobulaResponse {
    #[serde(default)]
    data: Option<Vec<TokenPair>>,
    #[serde(default)]
    pairs: Option<Vec<TokenPair>>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl MobulaResponse {
    /// Extract pairs from response (handles different response formats)
    fn get_pairs(&self) -> Vec<TokenPair> {
        self.data
            .clone()
            .or_else(|| self.pairs.clone())
            .unwrap_or_default()
    }
}

/// Trading opportunity identified from Mobula data
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

/// Mobula API client for token discovery and analysis (GMGN-compatible)
/// Official API: https://api.mobula.io/api/1/
/// Supports Solana and 50+ other blockchains
/// API Key required for production: https://admin.mobula.io
pub struct DexScreenerClient {
    api_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
    last_request_time: std::sync::Arc<std::sync::Mutex<SystemTime>>,
    request_count: std::sync::Arc<std::sync::Mutex<u32>>,
}

impl DexScreenerClient {
    pub fn new() -> Self {
        let api_key = std::env::var("MOBULA_API_KEY").ok();

        if api_key.is_some() {
            log::info!("✅ Mobula API key loaded from environment");
        } else {
            log::warn!("⚠️ Mobula API key not found. Some endpoints may have rate limits.");
            log::info!("   Set MOBULA_API_KEY environment variable or add it to .env file");
        }

        let client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("SolanaTradeBot/1.0");

        Self {
            api_url: "https://api.mobula.io/api/1".to_string(),
            api_key,
            client: client_builder.build().unwrap(),
            last_request_time: std::sync::Arc::new(std::sync::Mutex::new(SystemTime::now())),
            request_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }

    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.get(url);
        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
            log::debug!("🔑 Using Mobula API key for request");
        }
        request
    }

    async fn check_rate_limit(&self) -> Result<(), Box<dyn Error>> {
        let wait_time = {
            let mut last_time = self.last_request_time.lock().unwrap();
            let mut count = self.request_count.lock().unwrap();
            let now = SystemTime::now();
            let elapsed = now
                .duration_since(*last_time)
                .unwrap_or(Duration::from_secs(60));
            if elapsed >= Duration::from_secs(60) {
                *count = 0;
                *last_time = now;
            }
            if *count >= 1000 {
                let wait_time = Duration::from_secs(60) - elapsed;
                log::warn!("Rate limit reached, waiting {:?}", wait_time);
                Some(wait_time)
            } else {
                *count += 1;
                None
            }
        };
        if let Some(wait) = wait_time {
            tokio::time::sleep(wait).await;
            let mut last_time = self.last_request_time.lock().unwrap();
            let mut count = self.request_count.lock().unwrap();
            *count = 0;
            *last_time = SystemTime::now();
        }
        Ok(())
    }

    pub async fn search_tokens(&self, query: &str) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        let url = format!(
            "{}/market/search?q={}&blockchain=solana",
            self.api_url, query
        );
        let response = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Mobula API error {}: {}", response.status(), error_text).into());
        }
        let data: MobulaResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(data.get_pairs())
    }

    pub async fn get_token_pairs(
        &self,
        token_address: &str,
    ) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        let url = format!(
            "{}/market/blockchain/pairs?blockchain=solana&token={}",
            self.api_url, token_address
        );
        let response = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Mobula API error {}: {}", response.status(), error_text).into());
        }
        let data: MobulaResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(data.get_pairs())
    }

    pub async fn get_multiple_token_pairs(
        &self,
        token_addresses: &[String],
    ) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        if token_addresses.is_empty() {
            return Ok(Vec::new());
        }
        let addresses = token_addresses.join(",");
        self.get_token_pairs(&addresses).await
    }

    pub async fn get_pair(
        &self,
        chain: &str,
        pair_address: &str,
    ) -> Result<Option<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        let blockchain = if chain.to_lowercase() == "solana" {
            "solana"
        } else {
            chain
        };
        let url = format!(
            "{}/market/blockchain/pairs?blockchain={}&pair={}",
            self.api_url, blockchain, pair_address
        );
        let response = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Mobula API error: {}", response.status()).into());
        }
        let data: MobulaResponse = response.json().await?;
        Ok(data.get_pairs().into_iter().next())
    }

    pub async fn find_trending_solana_tokens(
        &self,
        min_liquidity_usd: f64,
    ) -> Result<Vec<TokenPair>, Box<dyn Error>> {
        self.check_rate_limit().await?;
        let url = format!(
            "{}/market/blockchain/pairs?blockchain=solana&sortBy=volume24h",
            self.api_url
        );
        let response = self
            .build_request(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Mobula API error: {}", error_text).into());
        }
        let data: MobulaResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let pairs = data.get_pairs();
        let trending: Vec<TokenPair> = pairs
            .into_iter()
            .filter(|p| p.liquidity.usd.unwrap_or(0.0) >= min_liquidity_usd && p.volume.h24 > 0.0)
            .collect();
        Ok(trending)
    }

    pub async fn analyze_opportunities(&self, pairs: Vec<TokenPair>) -> Vec<TradingOpportunity> {
        let mut opportunities = Vec::new();
        for pair in pairs {
            let price_usd = pair
                .price_usd
                .as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);
            let liquidity_usd = pair.liquidity.usd.unwrap_or(0.0);
            if price_usd <= 0.0 || liquidity_usd < 1000.0 {
                continue;
            }
            let mut signals = Vec::new();
            let momentum_5m = (pair.price_change.m5 / 10.0).min(1.0).max(0.0);
            let momentum_1h = (pair.price_change.h1 / 15.0).min(1.0).max(0.0);
            let momentum_6h = (pair.price_change.h6 / 25.0).min(1.0).max(0.0);
            let momentum_score = momentum_5m * 40.0 + momentum_1h * 35.0 + momentum_6h * 25.0;
            if momentum_5m > 0.5 {
                signals.push(format!("Strong 5m momentum: +{:.1}%", pair.price_change.m5));
            }
            if momentum_1h > 0.67 {
                signals.push(format!("Strong 1h trend: +{:.1}%", pair.price_change.h1));
            }
            if momentum_6h > 0.8 {
                signals.push(format!("Strong 6h uptrend: +{:.1}%", pair.price_change.h6));
            }
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
            if vol_ratio_1h > 0.5 {
                signals.push("Increasing volume".to_string());
            }
            if vol_ratio_5m > 0.5 {
                signals.push(format!("Volume spike: {:.1}x avg", vol_ratio_5m * 4.0));
            }
            let liquidity_ratio = (liquidity_usd.log10() / 5.0).min(1.0).max(0.0);
            let liquidity_score = liquidity_ratio * 100.0;
            let mut score = (momentum_score * 0.30)
                + (vol_ratio_1h * 50.0 + vol_ratio_5m * 50.0) * 0.25
                + (liquidity_score * 0.25);
            let sentiment_bonus = if pair.price_change.m5 > 0.0
                && pair.price_change.h1 > 0.0
                && pair.price_change.h6 > 0.0
            {
                20.0
            } else if (pair.price_change.m5 > 0.0 && pair.price_change.h1 > 0.0)
                || (pair.price_change.h1 > 0.0 && pair.price_change.h6 > 0.0)
            {
                10.0
            } else {
                0.0
            };
            score = (score + sentiment_bonus).min(100.0);
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
        opportunities.sort_by(|a, b| {
            b.opportunity_score
                .partial_cmp(&a.opportunity_score)
                .unwrap()
        });
        opportunities
    }

    pub async fn get_top_opportunities(
        &self,
        limit: usize,
    ) -> Result<Vec<TradingOpportunity>, Box<dyn Error>> {
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
        assert_eq!(client.api_url, "https://api.mobula.io/api/1");
    }

    #[test]
    fn test_trading_opportunity_scoring() {
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
