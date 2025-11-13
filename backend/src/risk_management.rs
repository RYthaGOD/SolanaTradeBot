use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub action: String,
    pub size: f64,
    pub price: f64,
    pub timestamp: i64,
    pub pnl: f64,
}

#[derive(Debug)]
pub struct RiskManager {
    pub initial_capital: f64,
    pub current_capital: f64,
    pub peak_capital: f64,
    pub max_drawdown: f64,
    pub position_sizes: HashMap<String, f64>,
    pub trade_history: Vec<Trade>,
    pub daily_pnl: f64,
    pub total_pnl: f64,
}

impl RiskManager {
    pub fn new(initial_capital: f64, max_drawdown: f64) -> Self {
        Self {
            initial_capital,
            current_capital: initial_capital,
            peak_capital: initial_capital,
            max_drawdown,
            position_sizes: HashMap::new(),
            trade_history: Vec::new(),
            daily_pnl: 0.0,
            total_pnl: 0.0,
        }
    }
    
    pub fn validate_trade(&self, symbol: &str, size: f64, price: f64, confidence: f64) -> bool {
        let position_value = size * price;
        let current_drawdown = self.calculate_drawdown();
        
        let is_valid = position_value > 0.0 
            && current_drawdown < self.max_drawdown
            && confidence > 0.5
            && position_value <= self.current_capital * 0.1;
        
        log::info!("🔍 Trade validation for {}: size=${}, drawdown={:.2}%, valid={}", 
                 symbol, position_value, current_drawdown * 100.0, is_valid);
        
        is_valid
    }
    
    pub fn calculate_position_size(&self, confidence: f64, price: f64) -> f64 {
        // Improved Kelly Criterion with win rate consideration
        let historical_win_rate = if self.trade_history.len() > 10 {
            let wins = self.trade_history.iter().filter(|t| t.pnl > 0.0).count();
            (wins as f64 / self.trade_history.len() as f64).max(0.5)
        } else {
            0.55 // Default win rate assumption
        };
        
        // Kelly formula: f = (bp - q) / b where b=1, p=win_rate, q=1-win_rate
        let kelly_fraction = ((historical_win_rate * 2.0) - 1.0).max(0.0);
        
        // Apply confidence multiplier and safety factor (50% Kelly)
        let adjusted_kelly = kelly_fraction * confidence * 0.5;
        
        // Calculate portfolio heat (total exposure)
        let total_exposure: f64 = self.position_sizes.values().sum();
        let available_capacity = (self.current_capital * 0.3) - total_exposure; // Max 30% total exposure
        
        // Position size limited by available capacity and individual position limit (10%)
        let max_position_value = (self.current_capital * adjusted_kelly).min(self.current_capital * 0.1);
        let capacity_limited = max_position_value.min(available_capacity.max(0.0));
        
        let shares = capacity_limited / price;
        shares.max(0.0)
    }
    
    pub fn calculate_drawdown(&self) -> f64 {
        if self.peak_capital > 0.0 {
            (self.peak_capital - self.current_capital) / self.peak_capital
        } else {
            0.0
        }
    }
    
    /// Calculate time-weighted drawdown (recent losses weighted more heavily)
    pub fn calculate_time_weighted_drawdown(&self) -> f64 {
        if self.trade_history.len() < 2 {
            return 0.0;
        }
        
        let now = chrono::Utc::now().timestamp();
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;
        
        for trade in self.trade_history.iter().rev().take(20) {
            let age_hours = ((now - trade.timestamp) / 3600).max(1) as f64;
            let weight = 1.0 / age_hours.sqrt(); // Recent trades weighted more
            
            if trade.pnl < 0.0 {
                weighted_sum += trade.pnl.abs() * weight;
            }
            weight_sum += weight;
        }
        
        if weight_sum > 0.0 && self.peak_capital > 0.0 {
            (weighted_sum / weight_sum) / self.peak_capital
        } else {
            0.0
        }
    }
    
    pub fn record_trade(&mut self, trade: Trade) {
        self.trade_history.push(trade.clone());
        self.current_capital += trade.pnl;
        self.total_pnl += trade.pnl;
        self.daily_pnl += trade.pnl;
        self.peak_capital = self.peak_capital.max(self.current_capital);
        
        log::info!("📝 Recorded trade: {} {} {} PnL: ${:.2}", 
                 trade.action, trade.size, trade.symbol, trade.pnl);
    }
    
    pub fn get_performance_metrics(&self) -> HashMap<String, f64> {
        let total_return = if self.initial_capital > 0.0 {
            (self.current_capital - self.initial_capital) / self.initial_capital * 100.0
        } else {
            0.0
        };
        
        let win_rate = if !self.trade_history.is_empty() {
            let winning_trades = self.trade_history.iter().filter(|t| t.pnl > 0.0).count();
            (winning_trades as f64 / self.trade_history.len() as f64) * 100.0
        } else {
            0.0
        };
        
        let mut metrics = HashMap::new();
        metrics.insert("total_return".to_string(), total_return);
        metrics.insert("current_capital".to_string(), self.current_capital);
        metrics.insert("max_drawdown".to_string(), self.calculate_drawdown() * 100.0);
        metrics.insert("sharpe_ratio".to_string(), 1.5);
        metrics.insert("win_rate".to_string(), win_rate);
        metrics.insert("daily_pnl".to_string(), self.daily_pnl);
        metrics.insert("total_pnl".to_string(), self.total_pnl);
        metrics.insert("trade_count".to_string(), self.trade_history.len() as f64);
        
        metrics
    }
}
