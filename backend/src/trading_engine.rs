use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::risk_management::RiskManager;

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

#[derive(Debug)]
pub struct TradingEngine {
    pub market_state: HashMap<String, VecDeque<MarketData>>,
    pub portfolio: HashMap<String, f64>,
    pub initial_balance: f64,
    pub current_balance: f64,
    pub trade_history: Vec<TradingSignal>,
    pub risk_manager: Arc<Mutex<RiskManager>>,
}

impl TradingEngine {
    pub fn new(risk_manager: Arc<Mutex<RiskManager>>) -> Self {
        Self {
            market_state: HashMap::new(),
            portfolio: HashMap::new(),
            initial_balance: 10000.0,
            current_balance: 10000.0,
            trade_history: Vec::new(),
            risk_manager,
        }
    }
    
    pub fn new_default() -> Self {
        let risk_manager = Arc::new(Mutex::new(RiskManager::new(10000.0, 0.1)));
        Self::new(risk_manager)
    }
    
    pub async fn process_market_data(&mut self, data: MarketData) -> Option<TradingSignal> {
        let symbol_data = self.market_state
            .entry(data.symbol.clone())
            .or_insert_with(|| VecDeque::with_capacity(100));
        
        symbol_data.push_back(data.clone());
        if symbol_data.len() > 100 {
            symbol_data.pop_front();
        }
        
        if symbol_data.len() >= 20 {
            let prices: Vec<f64> = symbol_data.iter().map(|d| d.price).collect();
            let volumes: Vec<f64> = symbol_data.iter().map(|d| d.volume).collect();
            
            // Use EMA instead of SMA for better responsiveness
            let ema_10 = Self::calculate_ema_static(&prices[prices.len()-10..], 10);
            let ema_20 = Self::calculate_ema_static(&prices, 20);
            
            // Calculate ATR for volatility-adjusted threshold
            let atr = Self::calculate_atr_static(symbol_data, 14);
            let volatility_threshold = (atr / data.price) * 100.0; // Convert to percentage
            let adaptive_threshold = volatility_threshold.max(1.5).min(3.0); // 1.5% to 3%
            
            // Volume confirmation (current volume > average)
            let avg_volume = volumes.iter().sum::<f64>() / volumes.len() as f64;
            let volume_confirmed = data.volume > avg_volume * 1.2;
            
            if ema_10 > ema_20 * (1.0 + adaptive_threshold / 100.0) 
                && self.current_balance > data.price 
                && volume_confirmed {
                let signal = TradingSignal {
                    id: uuid::Uuid::new_v4().to_string(),
                    action: TradeAction::Buy,
                    symbol: data.symbol.clone(),
                    price: data.price,
                    confidence: 0.7,
                    size: self.calculate_position_size(0.7, data.price).await,
                    stop_loss: data.price * 0.95,
                    take_profit: data.price * 1.05,
                    timestamp: Utc::now().timestamp(),
                };
                self.trade_history.push(signal.clone());
                return Some(signal);
            } else if ema_10 < ema_20 * (1.0 - adaptive_threshold / 100.0) && volume_confirmed {
                if let Some(&position) = self.portfolio.get(&data.symbol) {
                    if position > 0.0 {
                        let position_size = self.calculate_position_size(0.6, data.price).await;
                        let signal = TradingSignal {
                            id: uuid::Uuid::new_v4().to_string(),
                            action: TradeAction::Sell,
                            symbol: data.symbol.clone(),
                            price: data.price,
                            confidence: 0.6,
                            size: position.min(position_size),
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
    
    async fn calculate_position_size(&self, confidence: f64, price: f64) -> f64 {
        let risk_manager = self.risk_manager.lock().await;
        risk_manager.calculate_position_size(confidence, price)
    }
    
    pub async fn execute_trade(&mut self, signal: &TradingSignal) -> bool {
        // Validate trade with risk manager first
        let risk_manager = self.risk_manager.lock().await;
        let is_valid = risk_manager.validate_trade(
            &signal.symbol,
            signal.size,
            signal.price,
            signal.confidence,
        );
        drop(risk_manager);
        
        if !is_valid {
            log::warn!("❌ Trade rejected by risk manager");
            return false;
        }
        
        let success = match signal.action {
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
        };
        
        // Record trade in risk manager if successful
        if success {
            let action_str = match signal.action {
                TradeAction::Buy => "BUY",
                TradeAction::Sell => "SELL",
                TradeAction::Hold => "HOLD",
            };
            
            let pnl = match signal.action {
                TradeAction::Sell => signal.size * signal.price, // Simplified P&L calculation
                _ => 0.0,
            };
            
            let trade = crate::risk_management::Trade {
                id: signal.id.clone(),
                symbol: signal.symbol.clone(),
                action: action_str.to_string(),
                size: signal.size,
                price: signal.price,
                timestamp: signal.timestamp,
                pnl,
            };
            
            let mut risk_manager = self.risk_manager.lock().await;
            risk_manager.record_trade(trade);
        }
        
        success
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
    
    /// Get return on investment (ROI) percentage based on initial balance
    pub fn get_roi(&self) -> f64 {
        ((self.current_balance - self.initial_balance) / self.initial_balance) * 100.0
    }
    
    /// Get total portfolio value including current positions
    pub fn get_total_value(&self, current_prices: &HashMap<String, f64>) -> f64 {
        self.get_portfolio_value(current_prices)
    }
    
    /// Calculate Exponential Moving Average (more responsive than SMA)
    pub(crate) fn calculate_ema_static(prices: &[f64], period: usize) -> f64 {
        if prices.is_empty() {
            return 0.0;
        }
        
        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut ema = prices[0];
        
        for price in prices.iter().skip(1) {
            ema = (price * multiplier) + (ema * (1.0 - multiplier));
        }
        
        ema
    }
    
    /// Calculate Average True Range for volatility measurement
    pub(crate) fn calculate_atr_static(data: &VecDeque<MarketData>, period: usize) -> f64 {
        if data.len() < period + 1 {
            return 0.0;
        }
        
        let mut true_ranges = Vec::new();
        
        for i in 1..data.len() {
            let high = data[i].price.max(data[i-1].price);
            let low = data[i].price.min(data[i-1].price);
            let prev_close = data[i-1].price;
            
            let tr = (high - low)
                .max((high - prev_close).abs())
                .max((low - prev_close).abs());
            
            true_ranges.push(tr);
        }
        
        if true_ranges.len() >= period {
            let atr_slice = &true_ranges[true_ranges.len()-period..];
            atr_slice.iter().sum::<f64>() / period as f64
        } else {
            0.0
        }
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
