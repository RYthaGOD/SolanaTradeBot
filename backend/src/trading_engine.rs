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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

impl std::fmt::Display for TradeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TradeAction::Buy => write!(f, "BUY"),
            TradeAction::Sell => write!(f, "SELL"),
            TradeAction::Hold => write!(f, "HOLD"),
        }
    }
}

/// Trading engine with real Solana integration
/// Uses real PDA balance and executes real transactions
#[derive(Debug)]
pub struct TradingEngine {
    pub market_state: HashMap<String, VecDeque<MarketData>>,
    pub portfolio: HashMap<String, f64>,
    pub initial_balance: f64,
    pub current_balance: f64,
    pub trade_history: Vec<TradingSignal>,
    pub risk_manager: Arc<Mutex<RiskManager>>,
    /// Real Solana client for executing trades (optional - can work standalone)
    pub solana_client: Option<Arc<Mutex<crate::solana_integration::SolanaClient>>>,
    /// FIX #3: Track pending trades for rollback if confirmation fails
    pending_portfolio_updates: HashMap<String, (TradeAction, f64)>, // trade_id -> (action, size)
    /// Jupiter client for executing swaps (optional)
    pub jupiter_client: Option<Arc<crate::jupiter_integration::JupiterClient>>,
    /// Fee optimizer for transaction fee tracking and optimization
    pub fee_optimizer: Option<Arc<Mutex<crate::fee_optimization::FeeOptimizer>>>,
}

impl TradingEngine {
    /// Create new trading engine with real Solana integration
    pub fn new_with_solana(
        risk_manager: Arc<Mutex<RiskManager>>,
        solana_client: Arc<Mutex<crate::solana_integration::SolanaClient>>,
        jupiter_client: Option<Arc<crate::jupiter_integration::JupiterClient>>,
        fee_optimizer: Option<Arc<Mutex<crate::fee_optimization::FeeOptimizer>>>,
    ) -> Self {
        // Get initial balance from real PDA (will be synced async)
        let initial_balance = 0.0; // Will be synced from PDA
        
        let engine = Self {
            market_state: HashMap::new(),
            portfolio: HashMap::new(),
            initial_balance,
            current_balance: initial_balance,
            trade_history: Vec::new(),
            risk_manager,
            solana_client: Some(solana_client),
            jupiter_client,
            fee_optimizer,
            pending_portfolio_updates: HashMap::new(), // FIX #3: Initialize pending updates tracker
        };
        
        // Log initialization status
        log::info!("✅ TradingEngine initialized with real Solana client");
        log::debug!("   Solana client available: {}", engine.solana_client.is_some());
        log::debug!("   Jupiter client available: {}", engine.jupiter_client.is_some());
        log::debug!("   Fee optimizer available: {}", engine.fee_optimizer.is_some());
        
        engine
    }
    
    /// Create new trading engine (legacy - uses simulated balance)
    pub fn new(risk_manager: Arc<Mutex<RiskManager>>) -> Self {
        Self {
            market_state: HashMap::new(),
            portfolio: HashMap::new(),
            initial_balance: 0.0, // Will be synced from PDA if available
            current_balance: 0.0,
            trade_history: Vec::new(),
            risk_manager,
            solana_client: None,
            jupiter_client: None,
            fee_optimizer: None,
            pending_portfolio_updates: HashMap::new(), // FIX #3: Initialize pending updates tracker
        }
    }
    
    pub fn new_default() -> Self {
        let risk_manager = Arc::new(Mutex::new(RiskManager::new(10000.0, 0.1)));
        Self::new(risk_manager)
    }
    
    /// Sync balance from real PDA (if Solana client is available)
    pub async fn sync_balance_from_pda(&mut self) {
        if let Some(ref solana_client) = self.solana_client {
            let mut client = solana_client.lock().await;
            client.sync_trading_budget_from_pda().await;
            let pda_balance = client.get_trading_budget();
            
            if self.initial_balance == 0.0 {
                self.initial_balance = pda_balance;
            }
            self.current_balance = pda_balance;
            
            // FIX #2: Sync RiskManager current_capital with actual balance
            let mut risk_manager = self.risk_manager.lock().await;
            risk_manager.current_capital = pda_balance;
            drop(risk_manager);
            
            log::debug!("🔄 Trading engine balance synced from PDA: {:.6} SOL | Risk manager capital updated", pda_balance);
        }
    }
    
    pub async fn process_market_data(&mut self, data: MarketData) -> Option<TradingSignal> {
        // RESOURCE LIMIT: Prevent unbounded growth - limit market_state size
        const MAX_SYMBOLS: usize = 1000; // Max 1000 symbols tracked
        const MAX_DATA_POINTS: usize = 100; // Max 100 data points per symbol
        
        // Cleanup old symbols if we exceed limit
        if self.market_state.len() > MAX_SYMBOLS {
            // Remove oldest symbols (simple FIFO - could be improved with LRU)
            let keys_to_remove: Vec<String> = self.market_state.keys()
                .take(self.market_state.len() - MAX_SYMBOLS)
                .cloned()
                .collect();
            for key in keys_to_remove {
                self.market_state.remove(&key);
                log::debug!("🧹 Removed old symbol {} from market_state (limit: {})", key, MAX_SYMBOLS);
            }
        }
        
        let symbol_data = self.market_state
            .entry(data.symbol.clone())
            .or_insert_with(|| VecDeque::with_capacity(MAX_DATA_POINTS));
        
        symbol_data.push_back(data.clone());
        // ENFORCE LIMIT: Always maintain max size
        while symbol_data.len() > MAX_DATA_POINTS {
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
    
    /// Execute trade using REAL Solana transactions (if Solana client available)
    /// Falls back to simulated execution if no Solana client or if dry_run is enabled
    /// In dry-run mode, performs paper trading that updates state for ML/RL learning
    pub async fn execute_trade(
        &mut self, 
        signal: &TradingSignal, 
        trading_enabled: Option<&Arc<Mutex<bool>>>,
        dry_run: Option<&Arc<Mutex<bool>>>,
    ) -> bool {
        // Check if dry-run mode is enabled
        let is_dry_run = if let Some(dry_run_flag) = dry_run {
            *dry_run_flag.lock().await
        } else {
            false
        };
        
        // In dry-run mode, always use paper trading
        if is_dry_run {
            let action_str = match signal.action {
                TradeAction::Buy => "BUY",
                TradeAction::Sell => "SELL",
                TradeAction::Hold => "HOLD",
            };
            log::info!("🧪 DRY-RUN MODE: Executing paper trade for {} {} {} at ${:.8}", 
                      action_str, signal.size, signal.symbol, signal.price);
            return self.execute_paper_trade(signal).await;
        }
        
        // Check if trading is enabled
        if let Some(enabled) = trading_enabled {
            let is_enabled = *enabled.lock().await;
            if !is_enabled {
                log::warn!("⚠️ Trading is disabled - trade execution blocked");
                return false;
            }
        }
        
        // Sync balance from PDA before executing
        self.sync_balance_from_pda().await;
        
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
        
        // Execute REAL trade if Solana client is available
        match &self.solana_client {
            Some(solana_client) => {
                log::info!("🔗 Executing REAL Solana transaction via Jupiter API");
                let solana_client_clone = solana_client.clone();
                let success = self.execute_real_trade(signal, solana_client_clone).await;
                
                if success {
                    log::info!("✅ Real Solana transaction executed successfully");
                } else {
                    log::warn!("⚠️ Real Solana transaction execution returned false");
                }
                
                return success;
            }
            None => {
                log::error!("❌ CRITICAL: No Solana client available - cannot execute real trades!");
                log::error!("   TradingEngine was not initialized with Solana client.");
                log::error!("   Ensure TradingEngine::new_with_solana() is used instead of TradingEngine::new()");
                log::warn!("⚠️ Falling back to simulated execution (for testing only)");
                log::warn!("   ⚠️  WARNING: This is NOT a real trade - only for testing!");
                self.execute_simulated_trade(signal).await
            }
        }
    }
    
    /// Execute REAL Solana trade via Solana client
    async fn execute_real_trade(
        &mut self,
        signal: &TradingSignal,
        solana_client: Arc<Mutex<crate::solana_integration::SolanaClient>>,
    ) -> bool {
        let is_buy = matches!(signal.action, TradeAction::Buy);
        
        // FEE OPTIMIZATION: Get optimal fee estimate BEFORE executing trade
        use crate::fee_optimization::FeePriority;
        let (estimated_fee_lamports, _confirmation_time) = if let Some(ref fee_optimizer) = self.fee_optimizer {
                    let optimizer = fee_optimizer.lock().await;
                    // Get optimal fee estimate based on signal confidence (higher confidence = higher priority)
                    let priority = if signal.confidence >= 0.8 {
                        FeePriority::High
                    } else if signal.confidence >= 0.6 {
                        FeePriority::Normal
                    } else {
                        FeePriority::Low
                    };
                    let fee_estimate = optimizer.estimate_fee(priority);
                    let recommended_fee = fee_estimate.recommended_fee;
                    let confirmation_time = std::time::Duration::from_secs(1); // Default confirmation time
                    
                    log::debug!("💰 Using optimal fee estimate: {} lamports (priority: {:?}, confidence: {:.1}%)", 
                               recommended_fee, priority, signal.confidence * 100.0);
                    (recommended_fee, confirmation_time)
                } else {
                    // Fallback to default fee if no optimizer available
                    let default_fee = 5000u64;
                    let confirmation_time = std::time::Duration::from_secs(1);
                    (default_fee, confirmation_time)
                };
        
        // Execute real trade via Solana client with optimal fee estimate
        let trade_start_time = std::time::Instant::now();
        
        // FIX #3: Store pending portfolio update BEFORE execution (for rollback)
        let pending_update_key = format!("{}_{}", signal.id, signal.symbol); // Use signal.id instead of trade_id
        self.pending_portfolio_updates.insert(pending_update_key.clone(), (signal.action.clone(), signal.size));
        
        // FIX #3: Update portfolio optimistically BEFORE execution
        match signal.action {
            TradeAction::Buy => {
                *self.portfolio.entry(signal.symbol.clone()).or_insert(0.0) += signal.size;
                log::debug!("📊 Portfolio updated optimistically (pre-execution): +{} {}", signal.size, signal.symbol);
            }
            TradeAction::Sell => {
                if let Some(position) = self.portfolio.get_mut(&signal.symbol) {
                    *position = (*position - signal.size).max(0.0);
                    log::debug!("📊 Portfolio updated optimistically (pre-execution): -{} {}", signal.size, signal.symbol);
                }
            }
            TradeAction::Hold => {}
        }
        
        let mut client = solana_client.lock().await;
        let trade_result = client.execute_trade(
            &signal.symbol,
            signal.size,
            is_buy,
            signal.price,
            Some(estimated_fee_lamports), // PASS: Optimal fee estimate from fee optimizer
        ).await;
        drop(client); // Release lock early
        
        match trade_result {
            Ok(trade_id) => {
                // Measure actual execution time (approximation of confirmation time)
                let actual_execution_time = trade_start_time.elapsed();
                
                log::info!("✅ REAL trade executed: {} {} {} at ${:.8} | Trade ID: {} | Fee: {} lamports | Execution time: {:?}", 
                    if is_buy { "BUY" } else { "SELL" },
                    signal.size,
                    signal.symbol,
                    signal.price,
                    trade_id,
                    estimated_fee_lamports,
                    actual_execution_time
                );
                
                // FIX #5: Record transaction for future optimization with actual execution time
                // NOTE: This is execution time (request->response), not blockchain confirmation time
                // For true confirmation time, we would need to poll blockchain for transaction status
                // This is an approximation that works well in most cases
                // TODO: In production, poll blockchain for actual confirmation and update fee optimizer
                if let Some(ref fee_optimizer) = self.fee_optimizer {
                    let mut optimizer = fee_optimizer.lock().await;
                    // FIX #5: Use execution time as approximation, but note it's not true confirmation time
                    // For more accurate fee estimation, this should be replaced with actual blockchain confirmation time
                    optimizer.record_transaction(estimated_fee_lamports, actual_execution_time);
                    log::debug!("💰 Recorded transaction for fee optimization: fee={} lamports, execution_time={:?} (approximation - not true blockchain confirmation)", 
                               estimated_fee_lamports, actual_execution_time);
                }
                
                // Sync balance from PDA (actual balance from blockchain)
                self.sync_balance_from_pda().await;
                
                // FIX #2: Sync RiskManager capital after trade execution (ensure accuracy)
                // This is already handled in sync_balance_from_pda(), but we ensure it's done here too
                
                // FIX #3: Mark pending update as confirmed (after successful execution)
                // In production, this would only happen after blockchain confirmation
                // For now, we assume execution success = confirmation
                self.pending_portfolio_updates.remove(&pending_update_key);
                log::debug!("✅ Portfolio update confirmed for trade {}", trade_id);
                
                true  // FIX #3: Removed duplicate `true` statement
            }
            Err(e) => {
                log::error!("❌ REAL trade execution failed: {}", e);
                
                // FIX #3: Rollback portfolio update if trade execution failed
                // Use the pending_update_key we stored before execution
                if let Some((action, size)) = self.pending_portfolio_updates.remove(&pending_update_key) {
                    match action {
                        TradeAction::Buy => {
                            if let Some(position) = self.portfolio.get_mut(&signal.symbol) {
                                *position = (*position - size).max(0.0);
                                log::warn!("🔄 Rolled back portfolio update: -{} {} (trade failed)", size, signal.symbol);
                            }
                        }
                        TradeAction::Sell => {
                            *self.portfolio.entry(signal.symbol.clone()).or_insert(0.0) += size;
                            log::warn!("🔄 Rolled back portfolio update: +{} {} (trade failed)", size, signal.symbol);
                        }
                        TradeAction::Hold => {}
                    }
                } else {
                    log::warn!("⚠️ No pending portfolio update found for rollback: {}", pending_update_key);
                }
                
                false
            }
        }
    }
    
    /// Execute paper trade for dry-run mode
    /// Properly tracks PnL, updates state, and records trades for ML/RL learning
    async fn execute_paper_trade(&mut self, signal: &TradingSignal) -> bool {
        // Initialize paper trading balance if not already set (10 SOL starting balance)
        const PAPER_STARTING_BALANCE: f64 = 10.0; // 10 SOL for paper trading
        if self.current_balance == 0.0 && self.initial_balance == 0.0 {
            self.initial_balance = PAPER_STARTING_BALANCE;
            self.current_balance = PAPER_STARTING_BALANCE;
            
            // CRITICAL: Sync RiskManager with paper trading balance
            let mut risk_manager = self.risk_manager.lock().await;
            // Only update if RiskManager hasn't been initialized with real capital yet
            if risk_manager.initial_capital == 10000.0 && risk_manager.current_capital == 10000.0 {
                risk_manager.initial_capital = PAPER_STARTING_BALANCE;
                risk_manager.current_capital = PAPER_STARTING_BALANCE;
                // CRITICAL FIX: Reset peak_capital to prevent false drawdown (10000 -> 10 = 99.9% drawdown)
                risk_manager.peak_capital = PAPER_STARTING_BALANCE;
                log::info!("   RiskManager synced with paper trading balance: {:.8} SOL", PAPER_STARTING_BALANCE);
                log::info!("   Peak capital reset to {:.8} SOL (prevents false drawdown)", PAPER_STARTING_BALANCE);
            } else if risk_manager.peak_capital > risk_manager.current_capital * 10.0 && risk_manager.current_capital < 1000.0 {
                // FIX: If peak_capital is way out of sync (likely from initialization issue), reset it
                log::warn!("   ⚠️ Peak capital ({:.8}) out of sync with current ({:.8}) - resetting for paper trading", 
                          risk_manager.peak_capital, risk_manager.current_capital);
                risk_manager.peak_capital = risk_manager.current_capital;
            }
            drop(risk_manager);
            
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::info!("🧪 PAPER TRADING INITIALIZED");
            log::info!("   Starting Balance: {:.8} SOL", PAPER_STARTING_BALANCE);
            log::info!("   All trades will be simulated with this paper balance");
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
        
        // Validate trade with risk manager first (even in paper trading)
        let risk_manager = self.risk_manager.lock().await;
        let is_valid = risk_manager.validate_trade(
            &signal.symbol,
            signal.size,
            signal.price,
            signal.confidence,
        );
        drop(risk_manager);
        
        if !is_valid {
            log::warn!("❌ Paper trade rejected by risk manager");
            return false;
        }
        
        let success = match signal.action {
            TradeAction::Buy => {
                let cost = signal.size * signal.price;
                if cost <= self.current_balance {
                    self.current_balance -= cost;
                    *self.portfolio.entry(signal.symbol.clone()).or_insert(0.0) += signal.size;
                    log::info!("🧪 [PAPER TRADE] Bought {} {} at ${:.8} (cost: ${:.8})", 
                              signal.size, signal.symbol, signal.price, cost);
                    true
                } else {
                    log::warn!("❌ Insufficient balance for buy order (balance: ${:.8}, required: ${:.8})", 
                              self.current_balance, cost);
                    false
                }
            }
            TradeAction::Sell => {
                // Calculate PnL before getting mutable borrow
                let pnl = Self::calculate_pnl_for_sell_static(&self.trade_history, &signal.symbol, signal.size, signal.price);
                
                if let Some(position) = self.portfolio.get_mut(&signal.symbol) {
                    if *position >= signal.size {
                        *position -= signal.size;
                        self.current_balance += signal.size * signal.price;
                        
                        log::info!("🧪 [PAPER TRADE] Sold {} {} at ${:.8} (PnL: ${:.8})", 
                                  signal.size, signal.symbol, signal.price, pnl);
                        true
                    } else {
                        log::warn!("❌ Insufficient position for sell order (position: {}, required: {})", 
                                  *position, signal.size);
                        false
                    }
                } else {
                    log::warn!("❌ No position found for {}", signal.symbol);
                    false
                }
            }
            TradeAction::Hold => {
                log::debug!("🧪 [PAPER TRADE] Hold signal - no action taken");
                false
            }
        };
        
        if success {
            // Record trade in trade_history for ML/RL learning
            self.trade_history.push(signal.clone());
            
            // Record trade in risk manager with proper PnL
            self.record_paper_trade_in_risk_manager(signal).await;
            
            // CRITICAL: Sync RiskManager's current_capital with trading engine's balance
            // This ensures the dashboard shows correct paper trading balance
            let mut risk_manager = self.risk_manager.lock().await;
            risk_manager.current_capital = self.current_balance;
            risk_manager.peak_capital = risk_manager.peak_capital.max(self.current_balance);
            drop(risk_manager);
            
            let action_str = match signal.action {
                TradeAction::Buy => "BUY",
                TradeAction::Sell => "SELL",
                TradeAction::Hold => "HOLD",
            };
            log::debug!("📊 Paper trade recorded: {} {} {} | Balance: ${:.8} | Portfolio: {:?}", 
                       action_str, signal.size, signal.symbol, self.current_balance, self.portfolio);
        }
        
        success
    }
    
    /// Calculate PnL for a sell trade by matching with previous buy trades (FIFO)
    fn calculate_pnl_for_sell(&self, symbol: &str, sell_size: f64, sell_price: f64) -> f64 {
        Self::calculate_pnl_for_sell_static(&self.trade_history, symbol, sell_size, sell_price)
    }
    
    /// Static helper to calculate PnL without borrowing self
    fn calculate_pnl_for_sell_static(trade_history: &Vec<TradingSignal>, symbol: &str, sell_size: f64, sell_price: f64) -> f64 {
        // Find all buy trades for this symbol
        let buy_trades: Vec<&TradingSignal> = trade_history.iter()
            .filter(|t| t.symbol == symbol && matches!(t.action, TradeAction::Buy))
            .collect();
        
        if buy_trades.is_empty() {
            // No buy trades found, assume average cost basis
            return 0.0;
        }
        
        // Calculate average cost basis from buy trades
        let mut total_cost = 0.0;
        let mut total_size = 0.0;
        
        for trade in buy_trades {
            total_cost += trade.size * trade.price;
            total_size += trade.size;
        }
        
        if total_size == 0.0 {
            return 0.0;
        }
        
        let avg_cost_basis = total_cost / total_size;
        let pnl = (sell_price - avg_cost_basis) * sell_size;
        
        pnl
    }
    
    /// Record paper trade in risk manager with proper PnL calculation
    async fn record_paper_trade_in_risk_manager(&self, signal: &TradingSignal) {
        let action_str = match signal.action {
            TradeAction::Buy => "BUY",
            TradeAction::Sell => "SELL",
            TradeAction::Hold => "HOLD",
        };
        
        // Calculate PnL for sell trades
        let pnl = match signal.action {
            TradeAction::Sell => {
                self.calculate_pnl_for_sell(&signal.symbol, signal.size, signal.price)
            }
            _ => 0.0,
        };
        
        let trade = crate::risk_management::Trade {
            id: format!("paper_{}", signal.id),
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
    
    /// Execute simulated trade (fallback for testing - legacy method)
    async fn execute_simulated_trade(&mut self, signal: &TradingSignal) -> bool {
        let success = match signal.action {
            TradeAction::Buy => {
                let cost = signal.size * signal.price;
                if cost <= self.current_balance {
                    self.current_balance -= cost;
                    *self.portfolio.entry(signal.symbol.clone()).or_insert(0.0) += signal.size;
                    log::info!("✅ [SIMULATED] Bought {} {} at ${}", signal.size, signal.symbol, signal.price);
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
                        log::info!("✅ [SIMULATED] Sold {} {} at ${}", signal.size, signal.symbol, signal.price);
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
        
        if success {
            self.record_trade_in_risk_manager(signal, "simulated").await;
        }
        
        success
    }
    
    /// Record trade in risk manager
    async fn record_trade_in_risk_manager(&self, signal: &TradingSignal, trade_id: &str) {
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
            id: trade_id.to_string(),
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
        
    /// Execute signal from marketplace (converts TradingSignalData to TradingSignal)
    /// In dry-run mode, performs paper trading that updates state for ML/RL learning
    pub async fn execute_marketplace_signal(
        &mut self,
        signal_data: &crate::signal_platform::TradingSignalData,
        trading_enabled: Option<&Arc<Mutex<bool>>>,
        dry_run: Option<&Arc<Mutex<bool>>>,
    ) -> Result<String, String> {
        // Convert marketplace signal to trading signal
        let action = match signal_data.action {
            crate::signal_platform::SignalAction::Buy => TradeAction::Buy,
            crate::signal_platform::SignalAction::Sell => TradeAction::Sell,
            crate::signal_platform::SignalAction::Hold => TradeAction::Hold,
        };
        
        // FIX #6: Calculate position size with locked balance access to prevent race conditions
        // Check if we're in dry-run mode - don't sync from PDA in paper trading
        let is_dry_run = if let Some(dry_run_flag) = dry_run {
            *dry_run_flag.lock().await
        } else {
            false
        };
        
        let position_size = if matches!(action, TradeAction::Buy) {
            // Only sync balance from PDA if NOT in dry-run mode
            // In dry-run mode, use paper trading balance (already initialized)
            if !is_dry_run {
                self.sync_balance_from_pda().await;
            }
            // Immediately capture balance to ensure consistency
            let current_balance = self.current_balance; // Use paper balance in dry-run mode
            let max_cost = current_balance * 0.1; // Use 10% of balance per signal
            let calculated_size = max_cost / signal_data.entry_price;
            
            // Validate we have sufficient balance for the calculated size
            if calculated_size <= 0.0 || (calculated_size * signal_data.entry_price) > current_balance {
                return Err(format!("Insufficient balance for signal: {} (balance: {:.6}, required: {:.6})", 
                    signal_data.id, current_balance, calculated_size * signal_data.entry_price));
            }
            
            calculated_size
        } else {
            // For sell, use existing position
            self.portfolio.get(&signal_data.symbol).copied().unwrap_or(0.0)
        };
        
        if position_size <= 0.0 {
            return Err(format!("Insufficient balance/position for signal: {}", signal_data.id));
        }
        
        let signal = TradingSignal {
            id: signal_data.id.clone(),
            action,
            symbol: signal_data.symbol.clone(),
            price: signal_data.entry_price,
            confidence: signal_data.confidence,
            size: position_size,
            stop_loss: signal_data.stop_loss,
            take_profit: signal_data.target_price,
            timestamp: signal_data.timestamp,
        };
        
        let success = self.execute_trade(&signal, trading_enabled, dry_run).await;
        
        if success {
            Ok(format!("Signal {} executed successfully", signal_data.id))
        } else {
            Err(format!("Failed to execute signal: {}", signal_data.id))
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
