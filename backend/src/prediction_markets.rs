use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use chrono::Utc;

/// Represents a prediction market on Solana
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionMarket {
    pub market_id: String,
    pub question: String,
    pub category: MarketCategory,
    pub outcomes: Vec<MarketOutcome>,
    pub liquidity: f64,
    pub volume_24h: f64,
    pub end_date: i64,
    pub resolution_date: Option<i64>,
    pub status: MarketStatus,
    pub fee_bps: u16, // Fee in basis points (1% = 100 bps)
}

/// Categories of prediction markets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarketCategory {
    Crypto,
    Politics,
    Sports,
    Entertainment,
    Economics,
    Science,
    Other(String),
}

/// Individual outcome in a prediction market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOutcome {
    pub outcome_id: String,
    pub name: String,
    pub price: f64,        // Probability as price (0.0 to 1.0)
    pub shares: f64,       // Total shares outstanding
    pub volume: f64,       // Trading volume
    pub last_price: f64,   // Previous price for change calculation
    pub bid: Option<f64>,  // Best bid price
    pub ask: Option<f64>,  // Best ask price
}

/// Market status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarketStatus {
    Active,
    Closed,
    Resolved,
    Cancelled,
}

/// Trading signal for prediction markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionSignal {
    pub signal_id: String,
    pub market_id: String,
    pub outcome_id: String,
    pub action: SignalAction,
    pub target_price: f64,
    pub confidence: f64,
    pub expected_value: f64,
    pub kelly_fraction: f64,
    pub timestamp: i64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalAction {
    BuyYes,
    SellYes,
    BuyNo,
    SellNo,
    Hold,
}

/// Prediction market client for Solana-based protocols
pub struct PredictionMarketClient {
    markets: HashMap<String, PredictionMarket>,
    use_real_data: bool,
}

impl PredictionMarketClient {
    /// Create a new prediction market client
    pub fn new(use_real_data: bool) -> Self {
        let mut client = Self {
            markets: HashMap::new(),
            use_real_data,
        };
        
        // Initialize with simulated markets
        if !use_real_data {
            client.initialize_simulated_markets();
        }
        
        client
    }

    /// Initialize simulated markets for development/testing
    fn initialize_simulated_markets(&mut self) {
        // Crypto market
        self.markets.insert(
            "market_btc_100k".to_string(),
            PredictionMarket {
                market_id: "market_btc_100k".to_string(),
                question: "Will Bitcoin reach $100,000 by end of 2025?".to_string(),
                category: MarketCategory::Crypto,
                outcomes: vec![
                    MarketOutcome {
                        outcome_id: "yes_btc_100k".to_string(),
                        name: "Yes".to_string(),
                        price: 0.65,
                        shares: 50000.0,
                        volume: 125000.0,
                        last_price: 0.63,
                        bid: Some(0.64),
                        ask: Some(0.66),
                    },
                    MarketOutcome {
                        outcome_id: "no_btc_100k".to_string(),
                        name: "No".to_string(),
                        price: 0.35,
                        shares: 50000.0,
                        volume: 75000.0,
                        last_price: 0.37,
                        bid: Some(0.34),
                        ask: Some(0.36),
                    },
                ],
                liquidity: 100000.0,
                volume_24h: 50000.0,
                end_date: 1735689600, // End of 2025
                resolution_date: None,
                status: MarketStatus::Active,
                fee_bps: 200, // 2% fee
            },
        );

        // SOL price market
        self.markets.insert(
            "market_sol_500".to_string(),
            PredictionMarket {
                market_id: "market_sol_500".to_string(),
                question: "Will Solana reach $500 in 2025?".to_string(),
                category: MarketCategory::Crypto,
                outcomes: vec![
                    MarketOutcome {
                        outcome_id: "yes_sol_500".to_string(),
                        name: "Yes".to_string(),
                        price: 0.42,
                        shares: 30000.0,
                        volume: 60000.0,
                        last_price: 0.40,
                        bid: Some(0.41),
                        ask: Some(0.43),
                    },
                    MarketOutcome {
                        outcome_id: "no_sol_500".to_string(),
                        name: "No".to_string(),
                        price: 0.58,
                        shares: 30000.0,
                        volume: 70000.0,
                        last_price: 0.60,
                        bid: Some(0.57),
                        ask: Some(0.59),
                    },
                ],
                liquidity: 60000.0,
                volume_24h: 35000.0,
                end_date: 1735689600,
                resolution_date: None,
                status: MarketStatus::Active,
                fee_bps: 200,
            },
        );

        // ETH market
        self.markets.insert(
            "market_eth_10k".to_string(),
            PredictionMarket {
                market_id: "market_eth_10k".to_string(),
                question: "Will Ethereum reach $10,000 by end of 2025?".to_string(),
                category: MarketCategory::Crypto,
                outcomes: vec![
                    MarketOutcome {
                        outcome_id: "yes_eth_10k".to_string(),
                        name: "Yes".to_string(),
                        price: 0.55,
                        shares: 40000.0,
                        volume: 85000.0,
                        last_price: 0.54,
                        bid: Some(0.54),
                        ask: Some(0.56),
                    },
                    MarketOutcome {
                        outcome_id: "no_eth_10k".to_string(),
                        name: "No".to_string(),
                        price: 0.45,
                        shares: 40000.0,
                        volume: 65000.0,
                        last_price: 0.46,
                        bid: Some(0.44),
                        ask: Some(0.46),
                    },
                ],
                liquidity: 80000.0,
                volume_24h: 42000.0,
                end_date: 1735689600,
                resolution_date: None,
                status: MarketStatus::Active,
                fee_bps: 200,
            },
        );
    }

    /// Fetch all active markets
    pub async fn get_active_markets(&self) -> Vec<PredictionMarket> {
        self.markets
            .values()
            .filter(|m| m.status == MarketStatus::Active)
            .cloned()
            .collect()
    }

    /// Get a specific market by ID
    pub async fn get_market(&self, market_id: &str) -> Option<PredictionMarket> {
        self.markets.get(market_id).cloned()
    }

    /// Get markets by category
    pub async fn get_markets_by_category(&self, category: &MarketCategory) -> Vec<PredictionMarket> {
        self.markets
            .values()
            .filter(|m| &m.category == category && m.status == MarketStatus::Active)
            .cloned()
            .collect()
    }

    /// Analyze market for trading opportunities using Expected Value (EV) strategy
    pub async fn analyze_market(&self, market_id: &str) -> Result<Vec<PredictionSignal>, Box<dyn Error>> {
        let market = self.markets.get(market_id)
            .ok_or("Market not found")?;

        if market.status != MarketStatus::Active {
            return Ok(Vec::new());
        }

        let mut signals = Vec::new();

        for outcome in &market.outcomes {
            // Calculate expected value based on price vs implied probability
            let implied_prob = outcome.price;
            
            // Estimate true probability (simplified - in production, use ML/analysis)
            let true_prob = self.estimate_true_probability(market, outcome);
            
            // Calculate expected value: EV = (true_prob * payout) - (1 - true_prob) * loss
            // For binary markets: payout = 1/price if correct, loss = price if wrong
            let ev = if true_prob > implied_prob {
                // Positive EV - should buy
                let payout = 1.0 / implied_prob;
                let expected_return = (true_prob * payout) - ((1.0 - true_prob) * implied_prob);
                expected_return
            } else {
                // Negative EV - should sell or avoid
                let expected_return = true_prob - implied_prob;
                expected_return
            };

            // Generate signal if EV is significant
            if ev.abs() > 0.05 { // 5% edge threshold
                let confidence = (ev.abs() / 0.2).min(1.0); // Scale confidence
                
                // Calculate Kelly fraction for position sizing
                let win_prob = true_prob;
                let odds = if implied_prob > 0.0 { 1.0 / implied_prob } else { 0.0 };
                let kelly_fraction = if odds > 0.0 {
                    ((win_prob * odds - (1.0 - win_prob)) / odds).max(0.0).min(0.25) // Cap at 25%
                } else {
                    0.0
                };

                let action = if ev > 0.0 {
                    SignalAction::BuyYes
                } else {
                    SignalAction::SellYes
                };

                let reasoning = format!(
                    "EV: {:.2}%, Implied prob: {:.1}%, Estimated true prob: {:.1}%, Kelly: {:.1}%",
                    ev * 100.0,
                    implied_prob * 100.0,
                    true_prob * 100.0,
                    kelly_fraction * 100.0
                );

                signals.push(PredictionSignal {
                    signal_id: uuid::Uuid::new_v4().to_string(),
                    market_id: market_id.to_string(),
                    outcome_id: outcome.outcome_id.clone(),
                    action,
                    target_price: outcome.price,
                    confidence,
                    expected_value: ev,
                    kelly_fraction,
                    timestamp: Utc::now().timestamp(),
                    reasoning,
                });
            }
        }

        Ok(signals)
    }

    /// Estimate true probability (simplified - in production use ML/data analysis)
    fn estimate_true_probability(&self, market: &PredictionMarket, outcome: &MarketOutcome) -> f64 {
        // Use market dynamics to adjust probability
        let price_momentum = (outcome.price - outcome.last_price) / outcome.last_price.max(0.01);
        
        // Volume-weighted adjustment
        let volume_ratio = outcome.volume / market.volume_24h.max(1.0);
        
        // Liquidity adjustment (higher liquidity = more efficient pricing)
        let liquidity_factor = (market.liquidity / 100000.0).min(1.0);
        
        // Simple model: adjust implied probability based on momentum and volume
        let base_prob = outcome.price;
        let momentum_adjustment = price_momentum * 0.1 * (1.0 - liquidity_factor);
        let volume_adjustment = (volume_ratio - 0.5) * 0.05;
        
        let adjusted_prob = base_prob + momentum_adjustment + volume_adjustment;
        adjusted_prob.max(0.01).min(0.99)
    }

    /// Get market statistics
    pub async fn get_market_stats(&self) -> HashMap<String, serde_json::Value> {
        let active_markets = self.get_active_markets().await;
        
        let mut stats = HashMap::new();
        stats.insert("total_markets".to_string(), serde_json::json!(self.markets.len()));
        stats.insert("active_markets".to_string(), serde_json::json!(active_markets.len()));
        
        let total_liquidity: f64 = active_markets.iter().map(|m| m.liquidity).sum();
        stats.insert("total_liquidity".to_string(), serde_json::json!(total_liquidity));
        
        let total_volume: f64 = active_markets.iter().map(|m| m.volume_24h).sum();
        stats.insert("volume_24h".to_string(), serde_json::json!(total_volume));
        
        stats
    }

    /// Execute a prediction market trade (simulated)
    pub async fn execute_trade(
        &mut self,
        market_id: &str,
        outcome_id: &str,
        action: &SignalAction,
        amount: f64,
    ) -> Result<String, Box<dyn Error>> {
        let market = self.markets.get(market_id)
            .ok_or("Market not found")?;

        if market.status != MarketStatus::Active {
            return Err("Market is not active".into());
        }

        // Find the outcome
        let outcome = market.outcomes.iter()
            .find(|o| o.outcome_id == outcome_id)
            .ok_or("Outcome not found")?;

        // Calculate trade details
        let price = match action {
            SignalAction::BuyYes | SignalAction::BuyNo => {
                outcome.ask.unwrap_or(outcome.price * 1.01)
            }
            SignalAction::SellYes | SignalAction::SellNo => {
                outcome.bid.unwrap_or(outcome.price * 0.99)
            }
            SignalAction::Hold => {
                return Err("Cannot execute HOLD action".into());
            }
        };

        let shares = amount / price;
        let fee = amount * (market.fee_bps as f64 / 10000.0);
        let total_cost = amount + fee;

        let trade_id = uuid::Uuid::new_v4().to_string();
        
        log::info!(
            "📊 Prediction market trade executed: {} {} shares of {} at {:.4} (total: ${:.2})",
            match action {
                SignalAction::BuyYes | SignalAction::BuyNo => "BOUGHT",
                SignalAction::SellYes | SignalAction::SellNo => "SOLD",
                _ => "HELD",
            },
            shares,
            outcome.name,
            price,
            total_cost
        );

        Ok(trade_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = PredictionMarketClient::new(false);
        assert_eq!(client.markets.len(), 3);
    }

    #[tokio::test]
    async fn test_get_active_markets() {
        let client = PredictionMarketClient::new(false);
        let markets = client.get_active_markets().await;
        assert_eq!(markets.len(), 3);
        assert!(markets.iter().all(|m| m.status == MarketStatus::Active));
    }

    #[tokio::test]
    async fn test_get_market_by_id() {
        let client = PredictionMarketClient::new(false);
        let market = client.get_market("market_btc_100k").await;
        assert!(market.is_some());
        let market = market.unwrap();
        assert_eq!(market.market_id, "market_btc_100k");
        assert_eq!(market.outcomes.len(), 2);
    }

    #[tokio::test]
    async fn test_get_markets_by_category() {
        let client = PredictionMarketClient::new(false);
        let crypto_markets = client.get_markets_by_category(&MarketCategory::Crypto).await;
        assert_eq!(crypto_markets.len(), 3);
    }

    #[tokio::test]
    async fn test_analyze_market() {
        let client = PredictionMarketClient::new(false);
        let signals = client.analyze_market("market_btc_100k").await;
        assert!(signals.is_ok());
    }

    #[tokio::test]
    async fn test_market_stats() {
        let client = PredictionMarketClient::new(false);
        let stats = client.get_market_stats().await;
        assert!(stats.contains_key("total_markets"));
        assert!(stats.contains_key("active_markets"));
        assert!(stats.contains_key("total_liquidity"));
    }

    #[tokio::test]
    async fn test_execute_trade() {
        let mut client = PredictionMarketClient::new(false);
        let result = client.execute_trade(
            "market_btc_100k",
            "yes_btc_100k",
            &SignalAction::BuyYes,
            100.0,
        ).await;
        assert!(result.is_ok());
    }
}
