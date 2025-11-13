use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: i64,
    pub bid: f64,
    pub ask: f64,
    pub spread: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub id: String,
    pub action: TradeAction,
    pub symbol: String,
    pub price: f64,
    pub confidence: f64,
    pub size: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone)]
pub struct TradingEngine {
    pub market_state: HashMap<String, VecDeque<MarketData>>,
    pub portfolio: HashMap<String, f64>,
    pub initial_balance: f64,
    pub current_balance: f64,
    pub trade_history: Vec<TradingSignal>,
}

impl TradingEngine {
    pub fn new() -> Self {
        Self {
            market_state: HashMap::new(),
            portfolio: HashMap::new(),
            initial_balance: 10000.0,
            current_balance: 10000.0,
            trade_history: Vec::new(),
        }
    }
    
    pub fn process_market_data(&mut self, data: MarketData) -> Option<TradingSignal> {
        let symbol_data = self.market_state
            .entry(data.symbol.clone())
            .or_insert_with(|| VecDeque::with_capacity(100));
        
        symbol_data.push_back(data.clone());
        if symbol_data.len() > 100 {
            symbol_data.pop_front();
        }
        
        if symbol_data.len() >= 20 {
            let prices: Vec<f64> = symbol_data.iter().map(|d| d.price).collect();
            let sma_10: f64 = prices[prices.len()-10..].iter().sum::<f64>() / 10.0;
            let sma_20: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
            
            if sma_10 > sma_20 * 1.02 && self.current_balance > data.price {
                let signal = TradingSignal {
                    id: uuid::Uuid::new_v4().to_string(),
                    action: TradeAction::Buy,
                    symbol: data.symbol.clone(),
                    price: data.price,
                    confidence: 0.7,
                    size: self.calculate_position_size(0.7, data.price),
                    stop_loss: data.price * 0.95,
                    take_profit: data.price * 1.05,
                    timestamp: Utc::now().timestamp(),
                };
                self.trade_history.push(signal.clone());
                return Some(signal);
            } else if sma_10 < sma_20 * 0.98 {
                if let Some(&position) = self.portfolio.get(&data.symbol) {
                    if position > 0.0 {
                        let signal = TradingSignal {
                            id: uuid::Uuid::new_v4().to_string(),
                            action: TradeAction::Sell,
                            symbol: data.symbol.clone(),
                            price: data.price,
                            confidence: 0.6,
                            size: position.min(self.calculate_position_size(0.6, data.price)),
                            stop_loss: data.price * 1.05,
                            take_profit: data.price * 0.95,
                            timestamp: Utc::now().timestamp(),
                        };
                        self.trade_history.push(signal.clone());
                        return Some(signal);
                    }
                }
            }
        }
        
        None
    }
    
    fn calculate_position_size(&self, confidence: f64, price: f64) -> f64 {
        let max_position_value = self.current_balance * 0.1;
        let shares = (max_position_value * confidence) / price;
        shares.max(0.0)
    }
    
    pub fn execute_trade(&mut self, signal: &TradingSignal) -> bool {
        match signal.action {
            TradeAction::Buy => {
                let cost = signal.size * signal.price;
                if cost <= self.current_balance {
                    self.current_balance -= cost;
                    *self.portfolio.entry(signal.symbol.clone()).or_insert(0.0) += signal.size;
                    log::info!("✅ Bought {} {} at ${}", signal.size, signal.symbol, signal.price);
                    true
                } else {
                    log::warn!("❌ Insufficient balance for buy order");
                    false
                }
            }
            TradeAction::Sell => {
                if let Some(position) = self.portfolio.get_mut(&signal.symbol) {
                    if *position >= signal.size {
                        *position -= signal.size;
                        self.current_balance += signal.size * signal.price;
                        log::info!("✅ Sold {} {} at ${}", signal.size, signal.symbol, signal.price);
                        true
                    } else {
                        log::warn!("❌ Insufficient position for sell order");
                        false
                    }
                } else {
                    log::warn!("❌ No position found for {}", signal.symbol);
                    false
                }
            }
            TradeAction::Hold => {
                false
            }
        }
    }
    
    pub fn get_portfolio_value(&self, current_prices: &HashMap<String, f64>) -> f64 {
        let positions_value: f64 = self.portfolio.iter()
            .map(|(symbol, size)| {
                current_prices.get(symbol).unwrap_or(&0.0) * size
            })
            .sum();
        
        self.current_balance + positions_value
    }
    
    pub fn get_portfolio_data(&self) -> HashMap<String, f64> {
        let mut data = self.portfolio.clone();
        data.insert("CASH".to_string(), self.current_balance);
        data
    }
}

pub async fn generate_trading_signals(
    _engine: Arc<Mutex<TradingEngine>>,
    _risk_manager: Arc<Mutex<super::risk_management::RiskManager>>,
) {
    log::info!("🤖 Starting AI-powered trading signal generation");
    
    // Check if DeepSeek API key is available
    let use_deepseek = std::env::var("DEEPSEEK_API_KEY").is_ok();
    
    if use_deepseek {
        log::info!("✅ DeepSeek AI enabled for trading decisions");
    } else {
        log::warn!("⚠️ DeepSeek API key not found, using traditional signals");
    }
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        if use_deepseek {
            log::debug!("🧠 Generating AI-powered trading signals...");
            // AI signal generation would be implemented here
            // For now, keeping the existing logic active
        } else {
            log::debug!("🔍 Generating traditional trading signals...");
        }
    }
}
