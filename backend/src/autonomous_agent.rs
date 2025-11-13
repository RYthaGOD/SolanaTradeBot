use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use serde::Serialize;
use chrono::Utc;

use crate::switchboard_oracle::{SwitchboardClient, OracleFeed};
use crate::dex_screener::{DexScreenerClient, TradingOpportunity};
use crate::pumpfun::{PumpFunClient, MemeTradeSignal};
use crate::trading_engine::{TradingEngine, TradingSignal, TradeAction};
use crate::risk_management::RiskManager;

/// Autonomous trading agent decision
#[derive(Debug, Clone, Serialize)]
pub struct AgentDecision {
    pub timestamp: i64,
    pub decision_type: DecisionType,
    pub symbol: String,
    pub action: String,
    pub confidence: f64,
    pub data_sources: Vec<String>,
    pub reasoning: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum DecisionType {
    Trade,
    Hold,
    Monitor,
}

/// Autonomous trading agent that integrates all data sources
pub struct AutonomousAgent {
    oracle_client: Arc<SwitchboardClient>,
    dex_client: Arc<DexScreenerClient>,
    pumpfun_client: Arc<PumpFunClient>,
    trading_engine: Arc<Mutex<TradingEngine>>,
    risk_manager: Arc<Mutex<RiskManager>>,
    min_confidence: f64,
    check_interval_secs: u64,
}

impl AutonomousAgent {
    pub fn new(
        rpc_url: String,
        trading_engine: Arc<Mutex<TradingEngine>>,
        risk_manager: Arc<Mutex<RiskManager>>,
    ) -> Self {
        Self {
            oracle_client: Arc::new(SwitchboardClient::new(rpc_url)),
            dex_client: Arc::new(DexScreenerClient::new()),
            pumpfun_client: Arc::new(PumpFunClient::new()),
            trading_engine,
            risk_manager,
            min_confidence: 0.6, // 60% minimum confidence
            check_interval_secs: 60, // Check every minute
        }
    }
    
    /// Main autonomous trading loop
    pub async fn run(&self) {
        log::info!("🤖 Starting Autonomous Trading Agent...");
        log::info!("📊 Monitoring Switchboard Oracle, DEX Screener, and PumpFun");
        log::info!("🎯 Minimum confidence threshold: {}", self.min_confidence);
        
        loop {
            match self.analyze_and_trade().await {
                Ok(decision) => {
                    log::info!("✅ Agent decision: {:?}", decision);
                }
                Err(e) => {
                    log::error!("❌ Agent error: {}", e);
                }
            }
            
            // Wait before next analysis cycle
            tokio::time::sleep(tokio::time::Duration::from_secs(self.check_interval_secs)).await;
        }
    }
    
    /// Analyze all data sources and make trading decisions
    pub async fn analyze_and_trade(&self) -> Result<AgentDecision, Box<dyn std::error::Error>> {
        log::debug!("🔍 Analyzing market data from all sources...");
        
        // Gather data from all sources
        let oracle_feeds = self.fetch_oracle_data().await?;
        let dex_opportunities = self.fetch_dex_opportunities().await?;
        let meme_signals = self.fetch_meme_signals().await?;
        
        // Analyze and create composite decision
        let decision = self.make_composite_decision(
            oracle_feeds,
            dex_opportunities,
            meme_signals,
        ).await?;
        
        // Execute trade if decision confidence is high enough
        if decision.confidence >= self.min_confidence && decision.action != "HOLD" {
            self.execute_trade(&decision).await?;
        }
        
        Ok(decision)
    }
    
    /// Fetch live data from Switchboard Oracle
    async fn fetch_oracle_data(&self) -> Result<HashMap<String, OracleFeed>, Box<dyn std::error::Error>> {
        let symbols = vec!["SOL/USD".to_string(), "BTC/USD".to_string(), "ETH/USD".to_string()];
        let feeds = self.oracle_client.fetch_multiple_feeds(&symbols).await?;
        
        let mut feed_map = HashMap::new();
        for feed in feeds {
            feed_map.insert(feed.symbol.clone(), feed);
        }
        
        log::debug!("📡 Fetched {} oracle feeds", feed_map.len());
        Ok(feed_map)
    }
    
    /// Fetch trading opportunities from DEX Screener
    async fn fetch_dex_opportunities(&self) -> Result<Vec<TradingOpportunity>, Box<dyn std::error::Error>> {
        let opportunities = self.dex_client.get_top_opportunities(5).await?;
        log::debug!("🔍 Found {} DEX opportunities", opportunities.len());
        Ok(opportunities)
    }
    
    /// Fetch meme coin signals from PumpFun
    async fn fetch_meme_signals(&self) -> Result<Vec<MemeTradeSignal>, Box<dyn std::error::Error>> {
        let signals = self.pumpfun_client.get_top_opportunities(5).await?;
        log::debug!("🎯 Generated {} meme signals", signals.len());
        Ok(signals)
    }
    
    /// Make a composite trading decision based on all data sources
    async fn make_composite_decision(
        &self,
        oracle_feeds: HashMap<String, OracleFeed>,
        dex_opportunities: Vec<TradingOpportunity>,
        meme_signals: Vec<MemeTradeSignal>,
    ) -> Result<AgentDecision, Box<dyn std::error::Error>> {
        let mut data_sources = Vec::new();
        let mut reasoning = Vec::new();
        let mut best_action = "HOLD".to_string();
        let mut best_symbol = "NONE".to_string();
        let mut total_confidence = 0.0;
        let mut signal_count = 0;
        
        // Analyze oracle data for price trends
        for (symbol, feed) in &oracle_feeds {
            data_sources.push(format!("Switchboard Oracle: {}", symbol));
            reasoning.push(format!("{} at ${:.2} (confidence: {:.2}%)", 
                symbol, feed.price, feed.confidence * 100.0));
        }
        
        // Analyze DEX opportunities
        if !dex_opportunities.is_empty() {
            let top_opp = &dex_opportunities[0];
            
            if top_opp.opportunity_score > 70.0 {
                data_sources.push(format!("DEX Screener: {}", top_opp.token_symbol));
                reasoning.push(format!("High opportunity score: {:.1} for {}", 
                    top_opp.opportunity_score, top_opp.token_symbol));
                reasoning.extend(top_opp.signals.clone());
                
                best_action = "BUY".to_string();
                best_symbol = top_opp.token_symbol.clone();
                total_confidence += top_opp.opportunity_score / 100.0;
                signal_count += 1;
            }
        }
        
        // Analyze meme coin signals
        if !meme_signals.is_empty() {
            let top_meme = &meme_signals[0];
            
            if top_meme.confidence > 0.7 {
                data_sources.push(format!("PumpFun: {}", top_meme.symbol));
                reasoning.push(format!("Strong meme signal: {} confidence {:.2}", 
                    top_meme.symbol, top_meme.confidence));
                reasoning.extend(top_meme.reasons.clone());
                
                // Meme signals can override if confidence is higher
                if top_meme.confidence > total_confidence / signal_count.max(1) as f64 {
                    best_action = top_meme.action.clone();
                    best_symbol = top_meme.symbol.clone();
                    total_confidence = top_meme.confidence;
                    signal_count = 1;
                } else {
                    total_confidence += top_meme.confidence;
                    signal_count += 1;
                }
            }
        }
        
        // Calculate final confidence
        let final_confidence = if signal_count > 0 {
            total_confidence / signal_count as f64
        } else {
            0.0
        };
        
        let decision_type = if final_confidence >= self.min_confidence && best_action != "HOLD" {
            DecisionType::Trade
        } else if !data_sources.is_empty() {
            DecisionType::Monitor
        } else {
            DecisionType::Hold
        };
        
        if reasoning.is_empty() {
            reasoning.push("No strong signals detected".to_string());
        }
        
        Ok(AgentDecision {
            timestamp: Utc::now().timestamp(),
            decision_type,
            symbol: best_symbol,
            action: best_action,
            confidence: final_confidence,
            data_sources,
            reasoning,
        })
    }
    
    /// Execute a trade based on agent decision
    async fn execute_trade(&self, decision: &AgentDecision) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("🚀 Executing trade: {} {} (confidence: {:.2})", 
            decision.action, decision.symbol, decision.confidence);
        
        let mut engine = self.trading_engine.lock().await;
        let risk_manager = self.risk_manager.lock().await;
        
        // Get current market price (simplified - would use oracle data in production)
        let price = 100.0; // Placeholder
        
        // Calculate position size based on confidence and risk management
        let position_size = self.calculate_position_size(
            decision.confidence,
            price,
            engine.current_balance,
        );
        
        // Validate trade with risk manager
        let is_valid = risk_manager.validate_trade(
            &decision.symbol,
            position_size,
            price,
            decision.confidence,
        );
        
        if !is_valid {
            log::warn!("⚠️ Trade rejected by risk manager");
            return Ok(());
        }
        
        // Create and record trading signal
        let action = match decision.action.as_str() {
            "BUY" => TradeAction::Buy,
            "SELL" => TradeAction::Sell,
            _ => TradeAction::Hold,
        };
        
        let signal = TradingSignal {
            id: uuid::Uuid::new_v4().to_string(),
            action,
            symbol: decision.symbol.clone(),
            price,
            confidence: decision.confidence,
            size: position_size,
            stop_loss: price * 0.95,
            take_profit: price * 1.10,
            timestamp: Utc::now().timestamp(),
        };
        
        engine.trade_history.push(signal.clone());
        
        // Update portfolio
        match decision.action.as_str() {
            "BUY" => {
                if engine.current_balance >= position_size * price {
                    engine.current_balance -= position_size * price;
                    *engine.portfolio.entry(decision.symbol.clone()).or_insert(0.0) += position_size;
                    log::info!("✅ Bought {} {} at ${:.2}", position_size, decision.symbol, price);
                }
            }
            "SELL" => {
                if let Some(position) = engine.portfolio.get_mut(&decision.symbol) {
                    if *position >= position_size {
                        *position -= position_size;
                        engine.current_balance += position_size * price;
                        log::info!("✅ Sold {} {} at ${:.2}", position_size, decision.symbol, price);
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Calculate position size based on confidence and available capital
    fn calculate_position_size(&self, confidence: f64, price: f64, balance: f64) -> f64 {
        let max_position_pct = 0.1; // Max 10% of capital per trade
        let confidence_adjusted_pct = max_position_pct * confidence;
        let position_value = balance * confidence_adjusted_pct;
        position_value / price
    }
    
    /// Get agent statistics
    pub async fn get_stats(&self) -> HashMap<String, String> {
        let engine = self.trading_engine.lock().await;
        
        let mut stats = HashMap::new();
        stats.insert("status".to_string(), "active".to_string());
        stats.insert("min_confidence".to_string(), format!("{:.2}", self.min_confidence));
        stats.insert("check_interval".to_string(), format!("{} seconds", self.check_interval_secs));
        stats.insert("total_trades".to_string(), engine.trade_history.len().to_string());
        stats.insert("current_balance".to_string(), format!("${:.2}", engine.current_balance));
        stats.insert("active_positions".to_string(), engine.portfolio.len().to_string());
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_position_size() {
        let risk_manager = Arc::new(Mutex::new(RiskManager::new(10000.0, 0.1)));
        let engine = Arc::new(Mutex::new(TradingEngine::new(risk_manager.clone())));
        let agent = AutonomousAgent::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            engine,
            risk_manager,
        );
        
        let position_size = agent.calculate_position_size(0.7, 100.0, 10000.0);
        assert!(position_size > 0.0);
        assert!(position_size <= 10.0); // Max 10% of balance / price
    }

    #[tokio::test]
    async fn test_agent_stats() {
        let risk_manager = Arc::new(Mutex::new(RiskManager::new(10000.0, 0.1)));
        let engine = Arc::new(Mutex::new(TradingEngine::new(risk_manager.clone())));
        let agent = AutonomousAgent::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            engine,
            risk_manager,
        );
        
        let stats = agent.get_stats().await;
        assert!(stats.contains_key("status"));
        assert_eq!(stats.get("status").unwrap(), "active");
    }
}
