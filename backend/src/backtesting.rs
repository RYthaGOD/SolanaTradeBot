//! Backtesting Framework
//! Replays historical market data to test trading strategies
//! Provides comprehensive performance metrics and reporting

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use crate::trading_engine::{TradingEngine, TradingSignal, TradeAction, MarketData};
use crate::risk_management::RiskManager;
use uuid::Uuid;

/// Historical market data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub timestamp: i64,
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub bid: f64,
    pub ask: f64,
    pub spread: f64,
}

/// Backtest configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub initial_balance: f64,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub max_drawdown: f64,
    pub commission_rate: f64, // e.g., 0.001 for 0.1%
    pub slippage: f64, // e.g., 0.0005 for 0.05%
    pub min_confidence: f64,
    pub max_position_size_pct: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_balance: 10000.0,
            start_date: Utc::now() - chrono::Duration::days(30),
            end_date: Utc::now(),
            max_drawdown: 0.2, // 20% max drawdown
            commission_rate: 0.001, // 0.1% commission
            slippage: 0.0005, // 0.05% slippage
            min_confidence: 0.6,
            max_position_size_pct: 0.1, // Max 10% per position
        }
    }
}

/// Backtest results with comprehensive metrics
#[derive(Debug, Clone, Serialize)]
pub struct BacktestResults {
    pub config: BacktestConfig,
    pub initial_balance: f64,
    pub final_balance: f64,
    pub total_return: f64,
    pub total_return_pct: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub total_fees: f64,
    pub total_slippage: f64,
    pub daily_returns: Vec<f64>,
    pub equity_curve: Vec<(i64, f64)>,
    pub trade_history: Vec<BacktestTrade>,
    pub symbol_performance: HashMap<String, SymbolPerformance>,
}

/// Individual backtest trade record
#[derive(Debug, Clone, Serialize)]
pub struct BacktestTrade {
    pub id: String,
    pub symbol: String,
    pub action: String,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub size: f64,
    pub entry_timestamp: i64,
    pub exit_timestamp: Option<i64>,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub fees: f64,
    pub slippage: f64,
    pub confidence: f64,
    pub duration_seconds: Option<i64>,
}

/// Symbol-specific performance metrics
#[derive(Debug, Clone, Serialize)]
pub struct SymbolPerformance {
    pub symbol: String,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub best_trade: f64,
    pub worst_trade: f64,
}

/// Backtesting engine
pub struct BacktestEngine {
    config: BacktestConfig,
    trading_engine: Arc<Mutex<TradingEngine>>,
    risk_manager: Arc<Mutex<RiskManager>>,
    current_balance: f64,
    equity_curve: Vec<(i64, f64)>,
    trades: Vec<BacktestTrade>,
    open_positions: HashMap<String, BacktestTrade>, // symbol -> trade
    daily_returns: Vec<f64>,
    last_balance_snapshot: f64,
    last_snapshot_time: i64,
}

impl BacktestEngine {
    /// Create a new backtest engine
    pub fn new(config: BacktestConfig) -> Self {
        let risk_manager = Arc::new(Mutex::new(RiskManager::new(
            config.initial_balance,
            config.max_drawdown,
        )));
        
        let trading_engine = Arc::new(Mutex::new(TradingEngine::new(risk_manager.clone())));
        
        // Initialize trading engine balance
        let mut engine = trading_engine.blocking_lock();
        engine.initial_balance = config.initial_balance;
        engine.current_balance = config.initial_balance;
        drop(engine);
        
        Self {
            config: config.clone(),
            trading_engine,
            risk_manager,
            current_balance: config.initial_balance,
            equity_curve: vec![(Utc::now().timestamp(), config.initial_balance)],
            trades: Vec::new(),
            open_positions: HashMap::new(),
            daily_returns: Vec::new(),
            last_balance_snapshot: config.initial_balance,
            last_snapshot_time: Utc::now().timestamp(),
        }
    }
    
    /// Run backtest on historical data
    pub async fn run(&mut self, historical_data: Vec<HistoricalDataPoint>) -> BacktestResults {
        log::info!("🚀 Starting backtest...");
        log::info!("   Initial Balance: ${:.2}", self.config.initial_balance);
        log::info!("   Period: {} to {}", self.config.start_date, self.config.end_date);
        log::info!("   Data Points: {}", historical_data.len());
        
        // Sort data by timestamp
        let mut sorted_data = historical_data;
        sorted_data.sort_by_key(|d| d.timestamp);
        
        // Filter data to backtest period
        let filtered_data: Vec<HistoricalDataPoint> = sorted_data
            .into_iter()
            .filter(|d| {
                let dt = DateTime::from_timestamp(d.timestamp, 0).unwrap_or_default();
                dt >= self.config.start_date && dt <= self.config.end_date
            })
            .collect();
        
        log::info!("   Filtered Data Points: {}", filtered_data.len());
        
        // Process each data point
        for data_point in filtered_data {
            self.process_data_point(data_point).await;
        }
        
        // Close any remaining open positions
        self.close_all_positions(Utc::now().timestamp()).await;
        
        // Calculate final metrics
        self.calculate_results().await
    }
    
    /// Process a single historical data point
    async fn process_data_point(&mut self, data_point: HistoricalDataPoint) {
        // Convert to MarketData
        let market_data = MarketData {
            symbol: data_point.symbol.clone(),
            price: data_point.price,
            volume: data_point.volume,
            timestamp: data_point.timestamp,
            bid: data_point.bid,
            ask: data_point.ask,
            spread: data_point.spread,
        };
        
        // Update trading engine with market data
        let mut engine = self.trading_engine.lock().await;
        let signal = engine.process_market_data(market_data).await;
        drop(engine);
        
        // Execute signal if generated
        if let Some(signal) = signal {
            if signal.confidence >= self.config.min_confidence {
                self.execute_signal(signal, data_point.timestamp).await;
            }
        }
        
        // Update equity curve (snapshot daily)
        let current_day = data_point.timestamp / 86400; // Days since epoch
        let last_day = self.last_snapshot_time / 86400;
        
        if current_day > last_day {
            self.equity_curve.push((data_point.timestamp, self.current_balance));
            
            // Calculate daily return
            if self.last_balance_snapshot > 0.0 {
                let daily_return = (self.current_balance - self.last_balance_snapshot) / self.last_balance_snapshot;
                self.daily_returns.push(daily_return);
            }
            
            self.last_balance_snapshot = self.current_balance;
            self.last_snapshot_time = data_point.timestamp;
        }
        
        // Check for stop loss / take profit on open positions
        self.check_exit_conditions(data_point).await;
    }
    
    /// Execute a trading signal in backtest
    async fn execute_signal(&mut self, signal: TradingSignal, timestamp: i64) {
        let symbol = signal.symbol.clone();
        
        match signal.action {
            TradeAction::Buy => {
                // Check if we already have a position
                if self.open_positions.contains_key(&symbol) {
                    return; // Don't double up
                }
                
                // Calculate fees and slippage
                let slippage_cost = signal.size * signal.price * self.config.slippage;
                let commission = signal.size * signal.price * self.config.commission_rate;
                let total_cost = (signal.size * signal.price) + slippage_cost + commission;
                
                // Check if we have enough balance
                if total_cost > self.current_balance {
                    log::debug!("Insufficient balance for {}: need ${:.2}, have ${:.2}", 
                               symbol, total_cost, self.current_balance);
                    return;
                }
                
                // Execute trade
                self.current_balance -= total_cost;
                
                let trade = BacktestTrade {
                    id: signal.id.clone(),
                    symbol: symbol.clone(),
                    action: "BUY".to_string(),
                    entry_price: signal.price * (1.0 + self.config.slippage), // Apply slippage
                    exit_price: None,
                    size: signal.size,
                    entry_timestamp: timestamp,
                    exit_timestamp: None,
                    pnl: 0.0,
                    pnl_pct: 0.0,
                    fees: commission,
                    slippage: slippage_cost,
                    confidence: signal.confidence,
                    duration_seconds: None,
                };
                
                self.open_positions.insert(symbol, trade);
                
                log::debug!("📈 BUY {} @ ${:.8} | Size: {} | Cost: ${:.2} | Balance: ${:.2}", 
                           signal.symbol, signal.price, signal.size, total_cost, self.current_balance);
            }
            
            TradeAction::Sell => {
                // Check if we have a position to sell
                if let Some(mut trade) = self.open_positions.remove(&symbol) {
                    // Calculate fees and slippage
                    let exit_price = signal.price * (1.0 - self.config.slippage); // Apply slippage
                    let revenue = trade.size * exit_price;
                    let commission = revenue * self.config.commission_rate;
                    let slippage_cost = trade.size * signal.price * self.config.slippage;
                    let net_revenue = revenue - commission - slippage_cost;
                    
                    // Update balance
                    self.current_balance += net_revenue;
                    
                    // Calculate P&L
                    let entry_cost = trade.size * trade.entry_price + trade.fees + trade.slippage;
                    let pnl = net_revenue - entry_cost;
                    let pnl_pct = (pnl / entry_cost) * 100.0;
                    
                    // Update trade record
                    trade.exit_price = Some(exit_price);
                    trade.exit_timestamp = Some(timestamp);
                    trade.pnl = pnl;
                    trade.pnl_pct = pnl_pct;
                    trade.fees += commission;
                    trade.slippage += slippage_cost;
                    trade.duration_seconds = Some(timestamp - trade.entry_timestamp);
                    
                    self.trades.push(trade);
                    
                    log::debug!("📉 SELL {} @ ${:.8} | P&L: ${:.2} ({:.2}%) | Balance: ${:.2}", 
                               symbol, exit_price, pnl, pnl_pct, self.current_balance);
                }
            }
            
            TradeAction::Hold => {
                // No action
            }
        }
    }
    
    /// Check exit conditions (stop loss, take profit) for open positions
    async fn check_exit_conditions(&mut self, data_point: HistoricalDataPoint) {
        if let Some(trade) = self.open_positions.get_mut(&data_point.symbol) {
            let current_price = data_point.price;
            let entry_price = trade.entry_price;
            
            // Check stop loss (5% loss)
            if current_price <= entry_price * 0.95 {
                log::debug!("🛑 Stop loss triggered for {} @ ${:.8}", data_point.symbol, current_price);
                let signal = TradingSignal {
                    id: Uuid::new_v4().to_string(),
                    action: TradeAction::Sell,
                    symbol: data_point.symbol.clone(),
                    price: current_price,
                    confidence: 1.0,
                    size: trade.size,
                    stop_loss: current_price,
                    take_profit: current_price,
                    timestamp: data_point.timestamp,
                };
                self.execute_signal(signal, data_point.timestamp).await;
            }
            // Check take profit (5% gain)
            else if current_price >= entry_price * 1.05 {
                log::debug!("🎯 Take profit triggered for {} @ ${:.8}", data_point.symbol, current_price);
                let signal = TradingSignal {
                    id: Uuid::new_v4().to_string(),
                    action: TradeAction::Sell,
                    symbol: data_point.symbol.clone(),
                    price: current_price,
                    confidence: 1.0,
                    size: trade.size,
                    stop_loss: current_price,
                    take_profit: current_price,
                    timestamp: data_point.timestamp,
                };
                self.execute_signal(signal, data_point.timestamp).await;
            }
        }
    }
    
    /// Close all open positions at end of backtest
    async fn close_all_positions(&mut self, timestamp: i64) {
        let symbols: Vec<String> = self.open_positions.keys().cloned().collect();
        
        for symbol in symbols {
            if let Some(trade) = self.open_positions.get(&symbol) {
                // Use entry price as exit (conservative)
                let exit_price = trade.entry_price;
                
                let signal = TradingSignal {
                    id: Uuid::new_v4().to_string(),
                    action: TradeAction::Sell,
                    symbol: symbol.clone(),
                    price: exit_price,
                    confidence: 1.0,
                    size: trade.size,
                    stop_loss: exit_price,
                    take_profit: exit_price,
                    timestamp,
                };
                self.execute_signal(signal, timestamp).await;
            }
        }
    }
    
    /// Calculate comprehensive backtest results
    async fn calculate_results(&self) -> BacktestResults {
        let total_trades = self.trades.len();
        let winning_trades = self.trades.iter().filter(|t| t.pnl > 0.0).count();
        let losing_trades = total_trades - winning_trades;
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64
        } else {
            0.0
        };
        
        let _total_pnl: f64 = self.trades.iter().map(|t| t.pnl).sum();
        let total_fees: f64 = self.trades.iter().map(|t| t.fees).sum();
        let total_slippage: f64 = self.trades.iter().map(|t| t.slippage).sum();
        
        let winning_pnl: f64 = self.trades.iter()
            .filter(|t| t.pnl > 0.0)
            .map(|t| t.pnl)
            .sum();
        let losing_pnl: f64 = self.trades.iter()
            .filter(|t| t.pnl < 0.0)
            .map(|t| t.pnl.abs())
            .sum();
        
        let avg_win = if winning_trades > 0 {
            winning_pnl / winning_trades as f64
        } else {
            0.0
        };
        
        let avg_loss = if losing_trades > 0 {
            losing_pnl / losing_trades as f64
        } else {
            0.0
        };
        
        let profit_factor = if losing_pnl > 0.0 {
            winning_pnl / losing_pnl
        } else if winning_pnl > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        
        // Calculate drawdown
        let mut max_drawdown = 0.0;
        let mut peak = self.config.initial_balance;
        for (_, balance) in &self.equity_curve {
            if *balance > peak {
                peak = *balance;
            }
            let drawdown = (peak - balance) / peak;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        // Calculate Sharpe ratio (simplified)
        let avg_return = if !self.daily_returns.is_empty() {
            self.daily_returns.iter().sum::<f64>() / self.daily_returns.len() as f64
        } else {
            0.0
        };
        
        let return_std = if self.daily_returns.len() > 1 {
            let variance: f64 = self.daily_returns.iter()
                .map(|r| (r - avg_return).powi(2))
                .sum::<f64>() / (self.daily_returns.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        let sharpe_ratio = if return_std > 0.0 {
            (avg_return * 252.0) / (return_std * (252.0_f64).sqrt()) // Annualized
        } else {
            0.0
        };
        
        // Calculate Sortino ratio (only downside deviation)
        let downside_returns: Vec<f64> = self.daily_returns.iter()
            .filter(|r| **r < 0.0)
            .cloned()
            .collect();
        
        let downside_std = if downside_returns.len() > 1 {
            let variance: f64 = downside_returns.iter()
                .map(|r| (r - avg_return).powi(2))
                .sum::<f64>() / downside_returns.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        let sortino_ratio = if downside_std > 0.0 {
            (avg_return * 252.0) / (downside_std * (252.0_f64).sqrt())
        } else {
            0.0
        };
        
        // Calculate symbol performance
        let mut symbol_performance: HashMap<String, SymbolPerformance> = HashMap::new();
        
        for trade in &self.trades {
            let perf = symbol_performance.entry(trade.symbol.clone())
                .or_insert_with(|| SymbolPerformance {
                    symbol: trade.symbol.clone(),
                    total_trades: 0,
                    winning_trades: 0,
                    win_rate: 0.0,
                    total_pnl: 0.0,
                    avg_pnl: 0.0,
                    best_trade: f64::NEG_INFINITY,
                    worst_trade: f64::INFINITY,
                });
            
            perf.total_trades += 1;
            if trade.pnl > 0.0 {
                perf.winning_trades += 1;
            }
            perf.total_pnl += trade.pnl;
            perf.best_trade = perf.best_trade.max(trade.pnl);
            perf.worst_trade = perf.worst_trade.min(trade.pnl);
        }
        
        // Calculate averages
        for perf in symbol_performance.values_mut() {
            perf.win_rate = if perf.total_trades > 0 {
                perf.winning_trades as f64 / perf.total_trades as f64
            } else {
                0.0
            };
            perf.avg_pnl = if perf.total_trades > 0 {
                perf.total_pnl / perf.total_trades as f64
            } else {
                0.0
            };
        }
        
        let total_return = self.current_balance - self.config.initial_balance;
        let total_return_pct = (total_return / self.config.initial_balance) * 100.0;
        
        BacktestResults {
            config: self.config.clone(),
            initial_balance: self.config.initial_balance,
            final_balance: self.current_balance,
            total_return,
            total_return_pct,
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown,
            max_drawdown_pct: max_drawdown * 100.0,
            sharpe_ratio,
            sortino_ratio,
            total_fees,
            total_slippage,
            daily_returns: self.daily_returns.clone(),
            equity_curve: self.equity_curve.clone(),
            trade_history: self.trades.clone(),
            symbol_performance,
        }
    }
}

/// Generate sample historical data for testing
pub fn generate_sample_data(
    symbol: String,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    initial_price: f64,
) -> Vec<HistoricalDataPoint> {
    let mut data = Vec::new();
    let mut current_price = initial_price;
    let mut timestamp = start_date.timestamp();
    let end_timestamp = end_date.timestamp();
    
    // Generate hourly data points
    while timestamp <= end_timestamp {
        // Random walk with slight upward bias
        let change = (rand::random::<f64>() - 0.45) * 0.02; // -0.02 to +0.02, slight upward bias
        current_price *= 1.0 + change;
        current_price = current_price.max(0.01); // Floor price
        
        let spread = current_price * 0.001; // 0.1% spread
        let bid = current_price - spread / 2.0;
        let ask = current_price + spread / 2.0;
        
        data.push(HistoricalDataPoint {
            timestamp,
            symbol: symbol.clone(),
            price: current_price,
            volume: rand::random::<f64>() * 1000000.0,
            bid,
            ask,
            spread,
        });
        
        timestamp += 3600; // Next hour
    }
    
    data
}

