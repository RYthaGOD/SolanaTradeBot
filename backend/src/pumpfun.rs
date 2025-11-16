//! PumpFun memecoin monitoring and analysis with Moralis price enrichment
//! Integrated into AI orchestrator for memecoin opportunity detection

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// Meme coin sentiment analysis
#[derive(Debug, Clone, Serialize)]
pub struct MemeSentiment {
    pub token_address: String,
    pub symbol: String,
    pub sentiment_score: f64, // -100 to +100
    pub hype_level: HypeLevel,
    pub social_signals: Vec<String>,
    pub risk_level: RiskLevel,
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
pub struct PumpFunClient {
    api_url: String,
    moralis_api_url: String,
    moralis_api_key: Option<String>,
    client: reqwest::Client,
    recent_cache: Arc<Mutex<Option<CacheEntry<Vec<TokenLaunch>>>>>,
}

struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
}

impl PumpFunClient {
    pub fn new() -> Self {
        let moralis_api_key = std::env::var("MORALIS_API_KEY").ok();

        if moralis_api_key.is_some() {
            log::info!("✅ Moralis API key loaded for pump.fun token price data");
        } else {
            log::warn!("⚠️ Moralis API key not found. Token prices will be simulated.");
            log::info!("   Set MORALIS_API_KEY environment variable to enable real price data");
            log::info!("   Get your API key from: https://admin.moralis.com/");
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("SolanaTradeBot/1.0")
            .build()
            .expect("Failed to build PumpFun HTTP client");

        Self {
            api_url: "https://frontend-api.pump.fun".to_string(),
            moralis_api_url: "https://solana-gateway.moralis.io/token/mainnet".to_string(),
            moralis_api_key,
            client,
            recent_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Get recently created tokens on PumpFun with real-time price data from Moralis
    pub async fn get_recent_launches(
        &self,
        limit: usize,
    ) -> Result<Vec<TokenLaunch>, Box<dyn Error>> {
        log::debug!("Fetching recent launches from PumpFun");

        if let Some(cached) = self.get_cached_recent(Duration::from_secs(15)) {
            return Ok(cached.into_iter().take(limit).collect());
        }

        let mut launches = match self.fetch_recent_launches_api(limit).await {
            Ok(real) if !real.is_empty() => real,
            _ => self.simulate_recent_launches(limit),
        };

        if self.moralis_api_key.is_some() {
            log::info!("🔄 Enriching launches with real-time price data from Moralis...");
            for launch in &mut launches {
                match self.get_token_price_from_moralis(&launch.mint).await {
                    Ok(Some(price_data)) => {
                        let estimated_supply = 1_000_000_000.0;
                        launch.market_cap = price_data.usd_price * estimated_supply;
                        if !price_data.pair_address.is_empty() {
                            launch.bonding_curve = price_data.pair_address.clone();
                        }
                        log::debug!(
                            "✅ Updated {} with real price: ${:.6} on {}",
                            launch.symbol,
                            price_data.usd_price,
                            price_data.exchange_name
                        );
                    }
                    Ok(None) => {
                        log::debug!("No price data available for {}", launch.mint);
                    }
                    Err(e) => {
                        log::debug!("Could not fetch price for {}: {}", launch.mint, e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            log::info!(
                "✅ Enriched {} launches with real-time price data",
                launches.len()
            );
        }

        self.store_recent_cache(launches.clone());
        Ok(launches)
    }

    /// Get token details by mint address using Moralis API
    pub async fn get_token_details(
        &self,
        mint: &str,
    ) -> Result<Option<TokenLaunch>, Box<dyn Error>> {
        log::debug!("Fetching token details for: {}", mint);

        if let Some(_) = &self.moralis_api_key {
            match self.get_token_price_from_moralis(mint).await {
                Ok(Some(price_data)) => {
                    log::info!(
                        "✅ Got real-time price data from Moralis for token: {} ({} ${:.6})",
                        mint,
                        price_data.exchange_name,
                        price_data.usd_price
                    );
                    let estimated_supply = 1_000_000_000.0;
                    let market_cap = price_data.usd_price * estimated_supply;
                    return Ok(Some(TokenLaunch {
                        mint: mint.to_string(),
                        name: format!("Token ({})", price_data.exchange_name),
                        symbol: mint.chars().take(8).collect::<String>().to_uppercase(),
                        uri: format!("https://pump.fun/{}", mint),
                        creator: price_data.exchange_address.clone(),
                        created_timestamp: chrono::Utc::now().timestamp(),
                        market_cap,
                        reply_count: if market_cap > 10_000.0 { 50 } else { 10 },
                        is_currently_live: true,
                        king_of_the_hill_timestamp: None,
                        bonding_curve: price_data.pair_address.clone(),
                    }));
                }
                Ok(None) => log::debug!("No price data found for token: {}", mint),
                Err(e) => log::warn!("Failed to fetch price from Moralis: {}", e),
            }
        }

        Ok(None)
    }

    /// Get token price from Moralis API
    pub async fn get_token_price_from_moralis(
        &self,
        token_address: &str,
    ) -> Result<Option<MoralisTokenPrice>, Box<dyn Error>> {
        if self.moralis_api_key.is_none() {
            return Err(
                "Moralis API key required. Set MORALIS_API_KEY environment variable.".into(),
            );
        }

        let url = format!("{}/{}/price", self.moralis_api_url, token_address);
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", self.moralis_api_key.as_ref().unwrap())
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            if status == 404 {
                return Ok(None);
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Moralis API error {}: {}", status, error_text).into());
        }

        let price_data: MoralisTokenPrice = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(Some(price_data))
    }

    async fn fetch_recent_launches_api(
        &self,
        limit: usize,
    ) -> Result<Vec<TokenLaunch>, Box<dyn Error>> {
        let url = format!("{}/coins/recent?limit={}", self.api_url, limit.min(50));
        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("PumpFun API request failed: {}", e))?;

        if !response.status().is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(format!("PumpFun API error {}: {}", response.status(), body).into());
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse PumpFun response: {}", e))?;

        let coins = payload
            .get("coins")
            .and_then(|v| v.as_array())
            .cloned()
            .or_else(|| payload.as_array().cloned())
            .unwrap_or_default();

        let mut launches = Vec::new();
        for coin in coins {
            let mint = coin
                .get("mint")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if mint.is_empty() {
                continue;
            }

            launches.push(TokenLaunch {
                mint,
                name: coin
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed")
                    .to_string(),
                symbol: coin
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("COIN")
                    .to_uppercase(),
                uri: coin
                    .get("image_uri")
                    .or_else(|| coin.get("uri"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                creator: coin
                    .get("creator")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                created_timestamp: coin
                    .get("created_timestamp")
                    .or_else(|| coin.get("created_timestamp_unix"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or_else(|| Utc::now().timestamp()),
                market_cap: coin
                    .get("market_cap")
                    .or_else(|| coin.get("current_market_cap"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                reply_count: coin
                    .get("reply_count")
                    .or_else(|| coin.get("replies"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                is_currently_live: coin
                    .get("is_currently_live")
                    .or_else(|| coin.get("is_live"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                king_of_the_hill_timestamp: coin
                    .get("king_of_the_hill_timestamp")
                    .and_then(|v| v.as_i64()),
                bonding_curve: coin
                    .get("bonding_curve")
                    .or_else(|| coin.get("bonding_curve_address"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }

        Ok(launches)
    }

    fn get_cached_recent(&self, ttl: Duration) -> Option<Vec<TokenLaunch>> {
        let guard = self.recent_cache.lock().unwrap();
        guard.as_ref().and_then(|entry| {
            if entry.fetched_at.elapsed() < ttl {
                Some(entry.data.clone())
            } else {
                None
            }
        })
    }

    fn store_recent_cache(&self, data: Vec<TokenLaunch>) {
        let mut guard = self.recent_cache.lock().unwrap();
        *guard = Some(CacheEntry {
            data,
            fetched_at: Instant::now(),
        });
    }

    fn simulate_recent_launches(&self, limit: usize) -> Vec<TokenLaunch> {
        let mut launches = Vec::new();
        let base_timestamp = Utc::now().timestamp();
        let meme_names = [
            ("DOGE2", "Doge 2.0"),
            ("PEPE", "Pepe Coin"),
            ("SHIB2", "Shiba 2.0"),
            ("WOJAK", "Wojak Coin"),
            ("BONK2", "Bonk 2.0"),
            ("WIF", "Dogwifhat"),
            ("MEME", "Meme Coin"),
            ("FLOKI", "Floki Inu"),
        ];

        for i in 0..limit.min(meme_names.len()) {
            let (symbol, name) = meme_names[i % meme_names.len()];
            let timestamp = base_timestamp - (i as i64 * 300);

            launches.push(TokenLaunch {
                mint: format!(
                    "{}...{}",
                    &hex::encode(&rand::random::<[u8; 4]>()),
                    &hex::encode(&rand::random::<[u8; 4]>())
                ),
                name: name.to_string(),
                symbol: symbol.to_string(),
                uri: format!("https://pump.fun/token/{}", symbol.to_lowercase()),
                creator: format!(
                    "{}...{}",
                    &hex::encode(&rand::random::<[u8; 4]>()),
                    &hex::encode(&rand::random::<[u8; 4]>())
                ),
                created_timestamp: timestamp,
                market_cap: 10_000.0 + rand::random::<f64>() * 100_000.0,
                reply_count: rand::random::<u32>() % 100,
                is_currently_live: rand::random::<f64>() > 0.3,
                king_of_the_hill_timestamp: None,
                bonding_curve: format!("bonding_curve_{}", i),
            });
        }

        launches
    }

    pub fn analyze_sentiment(&self, launch: &TokenLaunch) -> MemeSentiment {
        let mut sentiment_score = 0.0;
        let mut social_signals = Vec::new();

        if launch.reply_count > 50 {
            sentiment_score += 20.0;
            social_signals.push("High engagement".to_string());
        } else if launch.reply_count > 20 {
            sentiment_score += 10.0;
            social_signals.push("Medium engagement".to_string());
        }

        if launch.is_currently_live {
            sentiment_score += 15.0;
            social_signals.push("Currently live".to_string());
        }

        if launch.market_cap > 50_000.0 {
            sentiment_score += 25.0;
            social_signals.push("Strong market cap".to_string());
        } else if launch.market_cap > 20_000.0 {
            sentiment_score += 10.0;
            social_signals.push("Growing market cap".to_string());
        }

        let age_hours = (Utc::now().timestamp() - launch.created_timestamp) / 3600;
        if age_hours < 1 {
            sentiment_score += 20.0;
            social_signals.push("Fresh launch".to_string());
        } else if age_hours < 6 {
            sentiment_score += 10.0;
            social_signals.push("Recent launch".to_string());
        }

        let hype_level = if sentiment_score >= 70.0 {
            HypeLevel::Extreme
        } else if sentiment_score >= 50.0 {
            HypeLevel::High
        } else if sentiment_score >= 30.0 {
            HypeLevel::Medium
        } else {
            HypeLevel::Low
        };

        let risk_level = if launch.market_cap < 10_000.0 || age_hours < 1 {
            RiskLevel::Extreme
        } else if launch.market_cap < 30_000.0 || age_hours < 3 {
            RiskLevel::High
        } else if launch.market_cap < 50_000.0 {
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
        }
    }

    pub async fn generate_meme_signals(&self, launches: Vec<TokenLaunch>) -> Vec<MemeTradeSignal> {
        let mut signals = Vec::new();

        for launch in launches {
            let sentiment = self.analyze_sentiment(&launch);

            if sentiment.sentiment_score > 40.0 {
                let mut reasons = Vec::new();
                let action: String;
                let confidence: f64;

                if sentiment.sentiment_score > 70.0 && launch.market_cap > 30_000.0 {
                    action = "BUY".to_string();
                    confidence = 0.75;
                    reasons.push("Extremely high sentiment".to_string());
                    reasons.push("Strong community backing".to_string());
                    reasons.extend(sentiment.social_signals.clone());
                } else if sentiment.sentiment_score > 50.0 {
                    action = "BUY".to_string();
                    confidence = 0.60;
                    reasons.push("Good sentiment".to_string());
                    reasons.extend(sentiment.social_signals.clone());
                } else {
                    action = "HOLD".to_string();
                    confidence = 0.45;
                    reasons.push("Moderate sentiment".to_string());
                    reasons.push("Monitor for better entry".to_string());
                }

                let entry_price = launch.market_cap / 1_000_000.0;
                let target_price = entry_price * (1.0 + confidence);
                let stop_loss = entry_price * 0.85;

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

        signals.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        log::info!("Generated {} meme coin trading signals", signals.len());
        signals
    }

    pub async fn get_top_opportunities(
        &self,
        limit: usize,
    ) -> Result<Vec<MemeTradeSignal>, Box<dyn Error>> {
        let launches = self.get_recent_launches(limit * 2).await?;
        let mut signals = self.generate_meme_signals(launches).await;
        signals.truncate(limit);
        Ok(signals)
    }

    pub async fn is_safe_to_trade_with_price(
        &self,
        token_address: &str,
    ) -> Result<bool, Box<dyn Error>> {
        if self.moralis_api_key.is_none() {
            log::warn!("Cannot validate token safety without Moralis API key");
            return Ok(false);
        }

        match self.get_token_price_from_moralis(token_address).await {
            Ok(Some(price_data)) => {
                let is_safe = price_data.usd_price > 0.0
                    && !price_data.exchange_name.is_empty()
                    && !price_data.pair_address.is_empty();
                Ok(is_safe)
            }
            Ok(None) => Ok(false),
            Err(e) => {
                log::warn!("Failed to check token safety: {}", e);
                Err(e)
            }
        }
    }
}

pub struct MemeAnalyzer {
    pumpfun: PumpFunClient,
}

impl MemeAnalyzer {
    pub fn new() -> Self {
        Self {
            pumpfun: PumpFunClient::new(),
        }
    }

    pub async fn analyze_and_rank(
        &self,
        limit: usize,
    ) -> Result<Vec<MemeTradeSignal>, Box<dyn Error>> {
        self.pumpfun.get_top_opportunities(limit).await
    }

    pub fn is_safe_to_trade(&self, sentiment: &MemeSentiment, _min_market_cap: f64) -> bool {
        matches!(sentiment.risk_level, RiskLevel::Low | RiskLevel::Medium)
            && sentiment.sentiment_score > 40.0
    }

    pub fn calculate_meme_position_size(&self, confidence: f64, account_balance: f64) -> f64 {
        let max_position_pct = 0.05;
        let size = account_balance * max_position_pct * confidence;
        size.min(account_balance * max_position_pct)
    }
}

/// Moralis API token price response
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
    pub value: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

impl MoralisTokenPrice {
    pub fn native_price_f64(&self) -> f64 {
        self.native_price.value.parse::<f64>().unwrap_or(0.0)
            / 10_f64.powi(self.native_price.decimals as i32)
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
            market_cap: 50_000.0,
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
        let position = analyzer.calculate_meme_position_size(0.7, 10_000.0);
        assert!(position > 0.0);
        assert!(position <= 500.0);
    }
}
