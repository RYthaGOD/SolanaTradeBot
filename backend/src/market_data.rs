use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;

/// Real-time market data provider using Pyth Network HTTP API
/// NO FALLBACKS - Real data only, system fails if oracles unavailable
pub struct MarketDataProvider {
    price_cache: Arc<RwLock<HashMap<String, PriceData>>>,
    pyth_price_ids: HashMap<String, String>,
    http_client: reqwest::Client,
}

/// Price data structure
#[derive(Debug, Clone)]
pub struct PriceData {
    pub symbol: String,
    pub price: f64,
    pub confidence: f64,
    pub timestamp: i64,
    pub source: PriceSource,
    pub volume_24h: Option<f64>,
}

/// Price data source - NO SIMULATED DATA
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriceSource {
    PythHTTP,
    SwitchboardHTTP,
    Cache,
}

impl std::fmt::Display for PriceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PriceSource::PythHTTP => write!(f, "Pyth-HTTP"),
            PriceSource::SwitchboardHTTP => write!(f, "Switchboard-HTTP"),
            PriceSource::Cache => write!(f, "Cache"),
        }
    }
}

// Pyth HTTP API response structures (matching official Hermes API schema)
#[derive(Debug, Deserialize)]
struct PythHTTPResponse {
    parsed: Option<Vec<PythParsedPrice>>,
}

#[derive(Debug, Deserialize)]
struct PythParsedPrice {
    id: String,
    price: PythPriceInfo,
}

#[derive(Debug, Deserialize)]
struct PythPriceInfo {
    price: String,
    conf: String,
    expo: i32,
    publish_time: i64,
}

impl MarketDataProvider {
    /// Create new market data provider - REAL DATA ONLY
    pub fn new() -> Self {
        log::info!("📊 Initializing Market Data Provider (REAL DATA ONLY - NO FALLBACKS)");
        
        // Pyth price feed IDs (from https://pyth.network/developers/price-feed-ids)
        let mut pyth_price_ids = HashMap::new();
        
        // Mainnet Pyth price feed IDs
        pyth_price_ids.insert(
            "SOL/USD".to_string(),
            "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d".to_string()
        );
        pyth_price_ids.insert(
            "BTC/USD".to_string(),
            "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string()
        );
        pyth_price_ids.insert(
            "ETH/USD".to_string(),
            "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string()
        );
        pyth_price_ids.insert(
            "USDC/USD".to_string(),
            "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a".to_string()
        );
        
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            pyth_price_ids,
            http_client,
        }
    }
    
    /// Get price for a symbol - REAL DATA ONLY, NO FALLBACKS
    /// System will fail if real data is unavailable
    pub async fn get_price(&self, symbol: &str) -> Result<PriceData> {
        // Check cache first
        {
            let cache = self.price_cache.read().await;
            if let Some(cached_price) = cache.get(symbol) {
                let age = chrono::Utc::now().timestamp() - cached_price.timestamp;
                if age < 10 {
                    // Cache is fresh (less than 10 seconds old)
                    log::debug!("📊 Using cached price for {}: ${:.2}", symbol, cached_price.price);
                    return Ok(cached_price.clone());
                }
            }
        }
        
        // Fetch fresh price from Pyth HTTP API - NO FALLBACKS
        let price_data = self.fetch_pyth_http(symbol).await
            .map_err(|e| anyhow::anyhow!("CRITICAL: Failed to fetch real price data for {}: {}. No fallbacks available.", symbol, e))?;
        
        // Update cache
        let mut cache = self.price_cache.write().await;
        cache.insert(symbol.to_string(), price_data.clone());
        super::monitoring::MARKET_DATA_UPDATES.inc();
        
        Ok(price_data)
    }
    
    /// Fetch price from Pyth HTTP API (Hermes endpoint)
    async fn fetch_pyth_http(&self, symbol: &str) -> Result<PriceData> {
        let price_id = self.pyth_price_ids.get(symbol)
            .ok_or_else(|| anyhow::anyhow!("No Pyth price feed ID for {}", symbol))?;
        
        log::debug!("📊 Fetching Pyth HTTP price for {} (ID: {})", symbol, price_id);
        
        // Use Pyth Hermes API v2 (official HTTP endpoint)
        // Format: https://hermes.pyth.network/v2/updates/price/latest?ids[]={id}&parsed=true
        let url = format!(
            "https://hermes.pyth.network/v2/updates/price/latest?ids[]={}&parsed=true",
            price_id
        );
        
        log::debug!("🌐 Requesting: {}", url);
        
        let response = self.http_client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                log::error!("❌ HTTP request failed for {}: {}", symbol, e);
                anyhow::anyhow!("Network error: {}. Ensure internet access to hermes.pyth.network", e)
            })?;
        
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());
            log::error!("❌ Pyth API error for {}: {} - {}", symbol, status, error_body);
            return Err(anyhow::anyhow!("Pyth API returned error {}: {}", status, error_body));
        }
        
        let response_text = response.text().await
            .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))?;
        
        log::debug!("📦 Response body: {}", response_text);
        
        let pyth_response: PythHTTPResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                log::error!("❌ JSON parse error for {}: {} | Response: {}", symbol, e, response_text);
                anyhow::anyhow!("Failed to parse Pyth response: {}", e)
            })?;
        
        let parsed_prices = pyth_response.parsed
            .ok_or_else(|| anyhow::anyhow!("No 'parsed' field in Pyth response"))?;
        
        let parsed_price = parsed_prices.first()
            .ok_or_else(|| anyhow::anyhow!("Empty parsed prices array in response"))?;
        
        // Parse price and confidence (hex strings from Pyth)
        let price_raw: i64 = parsed_price.price.price.parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse price value '{}': {}", parsed_price.price.price, e))?;
        let conf_raw: u64 = parsed_price.price.conf.parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse confidence value '{}': {}", parsed_price.price.conf, e))?;
        
        // Apply exponent to convert to decimal
        let price = (price_raw as f64) * 10_f64.powi(parsed_price.price.expo);
        let confidence = (conf_raw as f64) * 10_f64.powi(parsed_price.price.expo);
        
        // Validate price is reasonable
        if !price.is_finite() || price <= 0.0 {
            return Err(anyhow::anyhow!("Invalid price value: {}", price));
        }
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Check staleness (60 seconds max)
        let age = current_time - parsed_price.price.publish_time;
        if age > 60 {
            log::warn!("⚠️ Price data is stale for {}: {} seconds old", symbol, age);
            return Err(anyhow::anyhow!("Price data too stale: {} seconds old (max 60s)", age));
        }
        
        log::info!("✅ Pyth HTTP price for {}: ${:.2} ±${:.4} (age: {}s)", symbol, price, confidence, age);
        
        Ok(PriceData {
            symbol: symbol.to_string(),
            price,
            confidence,
            timestamp: parsed_price.price.publish_time,
            source: PriceSource::PythHTTP,
            volume_24h: None,
        })
    }
    

    
    /// Get prices for multiple symbols
    pub async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<PriceData>> {
        let mut prices = Vec::new();
        
        for symbol in symbols {
            match self.get_price(symbol).await {
                Ok(price_data) => prices.push(price_data),
                Err(e) => {
                    log::warn!("⚠️ Failed to get price for {}: {}", symbol, e);
                }
            }
        }
        
        Ok(prices)
    }
    
    /// Validate price data
    pub fn validate_price(&self, price_data: &PriceData, max_confidence_pct: f64) -> bool {
        // Check if confidence interval is acceptable
        let confidence_pct = (price_data.confidence / price_data.price) * 100.0;
        
        if confidence_pct > max_confidence_pct {
            log::warn!("⚠️ Price confidence too wide for {}: {:.2}% (max: {:.2}%)",
                price_data.symbol, confidence_pct, max_confidence_pct);
            return false;
        }
        
        // Check if price is recent (within 60 seconds)
        let age = chrono::Utc::now().timestamp() - price_data.timestamp;
        if age > 60 {
            log::warn!("⚠️ Price data too old for {}: {}s", price_data.symbol, age);
            return false;
        }
        
        // Check if price is reasonable (not zero or negative)
        if price_data.price <= 0.0 {
            log::warn!("⚠️ Invalid price for {}: {}", price_data.symbol, price_data.price);
            return false;
        }
        
        true
    }
    
    /// Clear price cache
    pub async fn clear_cache(&self) {
        let mut cache = self.price_cache.write().await;
        cache.clear();
        log::info!("🗑️ Price cache cleared");
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.price_cache.read().await;
        
        let mut stats = CacheStats {
            total_entries: cache.len(),
            pyth_prices: 0,
            switchboard_prices: 0,
            oldest_timestamp: i64::MAX,
            newest_timestamp: 0,
        };
        
        for price_data in cache.values() {
            match price_data.source {
                PriceSource::PythHTTP => stats.pyth_prices += 1,
                PriceSource::SwitchboardHTTP => stats.switchboard_prices += 1,
                _ => {}
            }
            
            stats.oldest_timestamp = stats.oldest_timestamp.min(price_data.timestamp);
            stats.newest_timestamp = stats.newest_timestamp.max(price_data.timestamp);
        }
        
        stats
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub pyth_prices: usize,
    pub switchboard_prices: usize,
    pub oldest_timestamp: i64,
    pub newest_timestamp: i64,
}

/// Start WebSocket price feed (placeholder for future implementation)
pub async fn start_websocket_feed(
    _provider: Arc<MarketDataProvider>,
    _symbols: Vec<String>,
) -> Result<()> {
    log::info!("🔌 WebSocket price feed support planned for future release");
    log::info!("📊 Currently using polling-based price updates");
    
    // TODO: Implement WebSocket connections to Pyth/Switchboard
    // This would provide real-time price updates instead of polling
    
    Ok(())
}

/// Price feed manager for continuous updates
pub struct PriceFeedManager {
    provider: Arc<MarketDataProvider>,
    symbols: Vec<String>,
    update_interval_secs: u64,
}

impl PriceFeedManager {
    pub fn new(
        provider: Arc<MarketDataProvider>,
        symbols: Vec<String>,
        update_interval_secs: u64,
    ) -> Self {
        Self {
            provider,
            symbols,
            update_interval_secs,
        }
    }
    
    /// Start continuous price updates
    pub async fn start(&self) {
        log::info!("📊 Starting price feed manager for {} symbols", self.symbols.len());
        log::info!("⏱️ Update interval: {}s", self.update_interval_secs);
        
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(self.update_interval_secs)
        );
        
        loop {
            interval.tick().await;
            
            let symbols_refs: Vec<&str> = self.symbols.iter().map(|s| s.as_str()).collect();
            match self.provider.get_prices(&symbols_refs).await {
                Ok(prices) => {
                    for price in prices {
                        log::debug!("📊 Updated price: {} = ${:.2} ({})", 
                            price.symbol, price.price, format!("{:?}", price.source));
                    }
                }
                Err(e) => {
                    log::error!("❌ Failed to update prices: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::commitment_config::CommitmentConfig;
    
    #[tokio::test]
    async fn test_simulated_price() {
        let rpc = Arc::new(tokio::sync::Mutex::new(
            super::super::solana_rpc::SolanaRpcClient::new(
                vec!["http://localhost:8899".to_string()],
                true,
                CommitmentConfig::confirmed(),
            )
        ));
        
        let provider = MarketDataProvider::new(rpc, false);
        
        let price = provider.get_price("SOL/USD").await.unwrap();
        assert!(price.price > 0.0);
        assert_eq!(price.source, PriceSource::Simulated);
    }
    
    #[tokio::test]
    async fn test_price_validation() {
        let rpc = Arc::new(tokio::sync::Mutex::new(
            super::super::solana_rpc::SolanaRpcClient::new(
                vec!["http://localhost:8899".to_string()],
                true,
                CommitmentConfig::confirmed(),
            )
        ));
        
        let provider = MarketDataProvider::new(rpc, false);
        
        let price = provider.get_price("SOL/USD").await.unwrap();
        
        // Should be valid with 1% max confidence
        assert!(provider.validate_price(&price, 1.0));
    }
    
    #[tokio::test]
    async fn test_cache() {
        let rpc = Arc::new(tokio::sync::Mutex::new(
            super::super::solana_rpc::SolanaRpcClient::new(
                vec!["http://localhost:8899".to_string()],
                true,
                CommitmentConfig::confirmed(),
            )
        ));
        
        let provider = MarketDataProvider::new(rpc, false);
        
        // First call - cache miss
        let price1 = provider.get_price("SOL/USD").await.unwrap();
        
        // Second call - cache hit
        let price2 = provider.get_price("SOL/USD").await.unwrap();
        
        // Prices should be very close (same cached value)
        assert!((price1.price - price2.price).abs() < 0.01);
    }
}
