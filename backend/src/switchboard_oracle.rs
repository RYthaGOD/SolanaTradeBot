use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

/// Represents a Switchboard Oracle price feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleFeed {
    pub feed_address: String,
    pub symbol: String,
    pub price: f64,
    pub confidence: f64,
    pub timestamp: i64,
    pub slot: u64,
}

/// Response from Switchboard API
#[derive(Debug, Deserialize)]
struct SwitchboardApiResponse {
    #[serde(default)]
    data: Option<SwitchboardData>,
}

#[derive(Debug, Deserialize)]
struct SwitchboardData {
    #[serde(default)]
    result: Option<f64>,
    #[serde(default)]
    std_dev: Option<f64>,
    #[serde(default)]
    slot: Option<u64>,
}

/// Switchboard Oracle client for fetching live price feeds
pub struct SwitchboardClient {
    rpc_url: String,
    client: reqwest::Client,
    feed_addresses: HashMap<String, String>,
}

impl SwitchboardClient {
    /// Create a new Switchboard client
    pub fn new(rpc_url: String) -> Self {
        let mut feed_addresses = HashMap::new();
        
        // Add known Switchboard feed addresses for popular Solana tokens
        // These are mainnet-beta feed addresses
        feed_addresses.insert(
            "SOL/USD".to_string(),
            "GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR".to_string(),
        );
        feed_addresses.insert(
            "BTC/USD".to_string(),
            "8SXvChNYFhRq4EZuZvnhjrB3jJRQCv4k3P4W6hesH3Ee".to_string(),
        );
        feed_addresses.insert(
            "ETH/USD".to_string(),
            "JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB".to_string(),
        );
        feed_addresses.insert(
            "USDC/USD".to_string(),
            "En8hkHLkRe9d9DraYmBTrus518BvmVH448YcvmrFM6Ce".to_string(),
        );
        
        Self {
            rpc_url,
            client: reqwest::Client::new(),
            feed_addresses,
        }
    }
    
    /// Fetch the latest price from a Switchboard feed
    pub async fn fetch_price(&self, symbol: &str) -> Result<OracleFeed, Box<dyn Error>> {
        let feed_address = self.feed_addresses.get(symbol)
            .ok_or_else(|| format!("No feed address found for symbol: {}", symbol))?;
        
        // For now, simulate oracle data since we need actual Solana RPC access
        // In production, this would query the Switchboard program account
        log::debug!("Fetching Switchboard feed for {} at {}", symbol, feed_address);
        
        // Simulate fetching from the oracle
        let simulated_price = self.simulate_oracle_price(symbol).await?;
        
        Ok(OracleFeed {
            feed_address: feed_address.clone(),
            symbol: symbol.to_string(),
            price: simulated_price,
            confidence: 0.01, // 1% confidence interval
            timestamp: chrono::Utc::now().timestamp(),
            slot: 0, // Would be actual slot from blockchain
        })
    }
    
    /// Fetch multiple feeds at once
    pub async fn fetch_multiple_feeds(&self, symbols: &[String]) -> Result<Vec<OracleFeed>, Box<dyn Error>> {
        let mut feeds = Vec::new();
        
        for symbol in symbols {
            match self.fetch_price(symbol).await {
                Ok(feed) => feeds.push(feed),
                Err(e) => log::warn!("Failed to fetch feed for {}: {}", symbol, e),
            }
        }
        
        Ok(feeds)
    }
    
    /// Get all available feed symbols
    pub fn get_available_symbols(&self) -> Vec<String> {
        self.feed_addresses.keys().cloned().collect()
    }
    
    /// Add a custom feed address
    pub fn add_feed(&mut self, symbol: String, feed_address: String) {
        self.feed_addresses.insert(symbol, feed_address);
    }
    
    /// Simulate oracle price for development/testing
    /// In production, this would be replaced with actual on-chain data fetching
    async fn simulate_oracle_price(&self, symbol: &str) -> Result<f64, Box<dyn Error>> {
        // Simulate realistic price movements based on symbol
        let base_price = match symbol {
            "SOL/USD" => 100.0 + (rand::random::<f64>() * 20.0),
            "BTC/USD" => 40000.0 + (rand::random::<f64>() * 5000.0),
            "ETH/USD" => 2500.0 + (rand::random::<f64>() * 300.0),
            "USDC/USD" => 1.0 + (rand::random::<f64>() * 0.01 - 0.005),
            _ => 1.0 + (rand::random::<f64>() * 0.5),
        };
        
        Ok(base_price)
    }
    
    /// Check if oracle data is fresh (within acceptable time window)
    pub fn is_data_fresh(feed: &OracleFeed, max_age_seconds: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - feed.timestamp <= max_age_seconds
    }
    
    /// Calculate price change percentage
    pub fn calculate_price_change(old_price: f64, new_price: f64) -> f64 {
        ((new_price - old_price) / old_price) * 100.0
    }
}

/// Oracle feed aggregator for cross-checking multiple sources
pub struct OracleAggregator {
    switchboard: SwitchboardClient,
}

impl OracleAggregator {
    pub fn new(rpc_url: String) -> Self {
        Self {
            switchboard: SwitchboardClient::new(rpc_url),
        }
    }
    
    /// Get aggregated price from multiple oracle sources
    pub async fn get_aggregated_price(&self, symbol: &str) -> Result<f64, Box<dyn Error>> {
        // Currently only using Switchboard, but could aggregate from multiple sources
        let feed = self.switchboard.fetch_price(symbol).await?;
        Ok(feed.price)
    }
    
    /// Get price with confidence interval
    pub async fn get_price_with_confidence(&self, symbol: &str) -> Result<(f64, f64), Box<dyn Error>> {
        let feed = self.switchboard.fetch_price(symbol).await?;
        Ok((feed.price, feed.confidence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_switchboard_client_creation() {
        let client = SwitchboardClient::new("https://api.mainnet-beta.solana.com".to_string());
        assert!(!client.feed_addresses.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_price() {
        let client = SwitchboardClient::new("https://api.mainnet-beta.solana.com".to_string());
        let result = client.fetch_price("SOL/USD").await;
        assert!(result.is_ok());
        let feed = result.unwrap();
        assert_eq!(feed.symbol, "SOL/USD");
        assert!(feed.price > 0.0);
    }

    #[tokio::test]
    async fn test_get_available_symbols() {
        let client = SwitchboardClient::new("https://api.mainnet-beta.solana.com".to_string());
        let symbols = client.get_available_symbols();
        assert!(!symbols.is_empty());
        assert!(symbols.contains(&"SOL/USD".to_string()));
    }

    #[test]
    fn test_is_data_fresh() {
        let feed = OracleFeed {
            feed_address: "test".to_string(),
            symbol: "SOL/USD".to_string(),
            price: 100.0,
            confidence: 0.01,
            timestamp: chrono::Utc::now().timestamp(),
            slot: 0,
        };
        assert!(SwitchboardClient::is_data_fresh(&feed, 60));
    }

    #[test]
    fn test_calculate_price_change() {
        let change = SwitchboardClient::calculate_price_change(100.0, 110.0);
        assert!((change - 10.0).abs() < 0.01);
    }
}
