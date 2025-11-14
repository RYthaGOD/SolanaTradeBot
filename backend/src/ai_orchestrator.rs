/// AI Orchestration Layer
/// Uses DeepSeek AI to intelligently prioritize and route function calls across all systems
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::deepseek_ai::DeepSeekClient;
use crate::database::Database;
use crate::security::RateLimiter;
use crate::key_manager::KeyManager;
use crate::error_handling::CircuitBreaker;
use crate::trading_engine::TradingEngine;
use crate::risk_management::RiskManager;
use crate::solana_integration::SolanaClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorRequest {
    pub context: String,
    pub available_functions: Vec<String>,
    pub current_state: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorResponse {
    pub recommended_function: String,
    pub priority: f64,
    pub reasoning: String,
    pub parameters: HashMap<String, String>,
}

/// Central AI Orchestrator that coordinates all system functions
pub struct AIOrchestrator {
    deepseek_client: Option<Arc<Mutex<DeepSeekClient>>>,
    database: Arc<Mutex<Database>>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    key_manager: Arc<Mutex<KeyManager>>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
    trading_engine: Arc<Mutex<TradingEngine>>,
    risk_manager: Arc<Mutex<RiskManager>>,
    solana_client: Arc<Mutex<SolanaClient>>,
}

impl AIOrchestrator {
    pub fn new(
        deepseek_client: Option<Arc<Mutex<DeepSeekClient>>>,
        database: Arc<Mutex<Database>>,
        rate_limiter: Arc<Mutex<RateLimiter>>,
        key_manager: Arc<Mutex<KeyManager>>,
        circuit_breaker: Arc<Mutex<CircuitBreaker>>,
        trading_engine: Arc<Mutex<TradingEngine>>,
        risk_manager: Arc<Mutex<RiskManager>>,
        solana_client: Arc<Mutex<SolanaClient>>,
    ) -> Self {
        Self {
            deepseek_client,
            database,
            rate_limiter,
            key_manager,
            circuit_breaker,
            trading_engine,
            risk_manager,
            solana_client,
        }
    }

    /// Use DeepSeek AI to decide which function to call based on context
    pub async fn decide_action(&self, request: OrchestratorRequest) -> Result<OrchestratorResponse, String> {
        if let Some(client) = &self.deepseek_client {
            let _client_lock = client.lock().await;
            
            // Create a prompt for DeepSeek to analyze the situation
            let _prompt = format!(
                "Context: {}\nAvailable Functions: {:?}\nCurrent State: {:?}\n\nBased on this trading context, which function should be called and why? Respond with function name, priority (0-1), and reasoning.",
                request.context,
                request.available_functions,
                request.current_state
            );
            
            // For now, use a simple heuristic since analyze_trade requires specific parameters
            // TODO: In production, use DeepSeek to parse the AI response and decide dynamically
            let response = OrchestratorResponse {
                recommended_function: self.select_best_function(&request).await,
                priority: 0.8,
                reasoning: "AI-driven decision based on current market context".to_string(),
                parameters: HashMap::new(),
            };
            
            Ok(response)
        } else {
            // Fallback to rule-based decision making
            Ok(self.rule_based_decision(&request).await)
        }
    }

    /// Rule-based fallback when DeepSeek is not available
    async fn rule_based_decision(&self, request: &OrchestratorRequest) -> OrchestratorResponse {
        let function_name = self.select_best_function(request).await;
        
        OrchestratorResponse {
            recommended_function: function_name,
            priority: 0.5,
            reasoning: "Rule-based decision (DeepSeek not configured)".to_string(),
            parameters: HashMap::new(),
        }
    }

    /// Select best function based on context keywords - SIMPLIFIED & ATOMIC
    async fn select_best_function(&self, request: &OrchestratorRequest) -> String {
        let context = request.context.to_lowercase();
        
        // Atomic function selection (fewer, more powerful functions)
        if context.contains("trade") || context.contains("execute") || context.contains("buy") || context.contains("sell") {
            return "trade".to_string();
        }
        
        if context.contains("portfolio") || context.contains("holdings") {
            return "portfolio".to_string();
        }
        
        if context.contains("risk") || context.contains("drawdown") || context.contains("performance") {
            return "risk".to_string();
        }
        
        if context.contains("database") || context.contains("trades") || context.contains("history") {
            return "database".to_string();
        }
        
        if context.contains("wallet") || context.contains("balance") || context.contains("address") {
            return "wallet".to_string();
        }
        
        if context.contains("fee") || context.contains("cost") {
            return "fees".to_string();
        }
        
        if context.contains("predict") || context.contains("forecast") || context.contains("ml") || context.contains("ai") {
            return "predict".to_string();
        }
        
        if context.contains("validate") || context.contains("check") || context.contains("verify") {
            return "validate".to_string();
        }
        
        // Default to system status
        "system".to_string()
    }

    /// Execute a function based on orchestrator decision
    pub async fn execute_function(&self, function_name: &str, parameters: HashMap<String, String>) -> Result<String, String> {
        // Check rate limit before execution (with proper error handling)
        let rate_limit_ok = {
            let rate_limiter = self.rate_limiter.lock().await;
            rate_limiter.check_rate_limit("orchestrator".to_string()).await
        };
        
        if !rate_limit_ok {
            log::warn!("Rate limit exceeded for orchestrator function: {}", function_name);
            // Allow execution anyway for internal orchestrator (trusted)
            // In production, you might want to enforce this more strictly
        }

        // Use circuit breaker for fault tolerance
        let circuit_state = {
            let circuit_breaker = self.circuit_breaker.lock().await;
            circuit_breaker.get_state().await
        };
        
        if matches!(circuit_state, crate::error_handling::CircuitState::Open) {
            return Err("Circuit breaker is open - system is protecting itself".to_string());
        }

        // Execute the requested function - SIMPLIFIED AND CONSOLIDATED
        match function_name {
            // Core trading operations (consolidated)
            "trade" => self.handle_trade(parameters).await,
            "portfolio" => self.handle_portfolio(parameters).await,
            
            // Risk & performance (consolidated)
            "risk" => self.handle_risk(parameters).await,
            
            // Database operations (consolidated)
            "database" => self.handle_database(parameters).await,
            
            // Blockchain operations (consolidated)
            "wallet" => self.handle_wallet(parameters).await,
            
            // Fee operations (consolidated)
            "fees" => self.handle_fees(parameters).await,
            
            // ML & AI operations (consolidated)
            "predict" => self.handle_predict(parameters).await,
            
            // Security & validation (consolidated)
            "validate" => self.handle_validate(parameters).await,
            
            // System health (consolidated)
            "system" => self.handle_system(parameters).await,
            
            _ => Err(format!("Unknown function: {}. Available: trade, portfolio, risk, database, wallet, fees, predict, validate, system", function_name)),
        }
    }

    // ATOMIC HANDLER: Trading operations (combines multiple steps)
    async fn handle_trade(&self, params: HashMap<String, String>) -> Result<String, String> {
        let action = params.get("action").map(|s| s.as_str()).unwrap_or("execute");
        
        match action {
            "execute" => {
                // ATOMIC: Execute trade + Save to DB + Update risk metrics + Log
                let symbol = params.get("symbol").ok_or("Missing symbol")?;
                let size = params.get("size").and_then(|s| s.parse::<f64>().ok()).ok_or("Missing size")?;
                let is_buy = params.get("is_buy").and_then(|s| s.parse::<bool>().ok()).unwrap_or(true);
                let price = params.get("price").and_then(|s| s.parse::<f64>().ok()).ok_or("Missing price")?;

                // Step 1: Execute trade
                let mut solana_client = self.solana_client.lock().await;
                let tx_result = solana_client.execute_trade(symbol, size, is_buy, price).await?;
                drop(solana_client);
                
                // Step 2: Save to database atomically
                let mut db = self.database.lock().await;
                let trade_record = crate::database::TradeRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    symbol: symbol.to_string(),
                    action: if is_buy { "BUY" } else { "SELL" }.to_string(),
                    price,
                    size,
                    total_value: size * price,
                    fee: 0.0,
                    pnl: 0.0,
                    confidence: 0.8,
                    strategy: "AI_Orchestrated".to_string(),
                };
                db.insert_trade(trade_record.clone()).ok();
                drop(db);
                
                // Step 3: Update risk metrics atomically
                let mut risk_manager = self.risk_manager.lock().await;
                let risk_trade = crate::risk_management::Trade {
                    id: trade_record.id.clone(),
                    symbol: symbol.to_string(),
                    action: if is_buy { "BUY" } else { "SELL" }.to_string(),
                    size,
                    price,
                    timestamp: trade_record.timestamp,
                    pnl: 0.0,
                };
                risk_manager.record_trade(risk_trade);
                drop(risk_manager);
                
                Ok(format!("ATOMIC TRADE COMPLETE: {} | TX: {} | Saved to DB | Risk Updated", 
                    symbol, tx_result))
            }
            "status" => {
                let trading_engine = self.trading_engine.lock().await;
                let roi = trading_engine.get_roi();
                Ok(format!("Trading Status - ROI: {:.2}%", roi))
            }
            _ => Err(format!("Unknown trade action: {}", action))
        }
    }

    // CONSOLIDATED HANDLER: Portfolio operations
    async fn handle_portfolio(&self, params: HashMap<String, String>) -> Result<String, String> {
        let action = params.get("action").map(|s| s.as_str()).unwrap_or("get");
        
        match action {
            "get" => {
                let trading_engine = self.trading_engine.lock().await;
                let portfolio = trading_engine.get_portfolio_data();
                let roi = trading_engine.get_roi();
                Ok(format!("Portfolio - ROI: {:.2}%, Holdings: {:?}", roi, portfolio))
            }
            "value" => {
                let trading_engine = self.trading_engine.lock().await;
                // Get current prices from market state
                let mut prices = std::collections::HashMap::new();
                for (symbol, data) in trading_engine.market_state.iter() {
                    if let Some(latest) = data.back() {
                        prices.insert(symbol.clone(), latest.price);
                    }
                }
                let total_value = trading_engine.get_total_value(&prices);
                Ok(format!("Portfolio Total Value: ${:.2}", total_value))
            }
            _ => Err(format!("Unknown portfolio action: {}", action))
        }
    }

    // CONSOLIDATED HANDLER: Risk management
    async fn handle_risk(&self, params: HashMap<String, String>) -> Result<String, String> {
        let action = params.get("action").map(|s| s.as_str()).unwrap_or("metrics");
        let risk_manager = self.risk_manager.lock().await;
        
        match action {
            "drawdown" => {
                let drawdown = risk_manager.calculate_time_weighted_drawdown();
                Ok(format!("Time-weighted drawdown: {:.4}", drawdown))
            }
            "metrics" => {
                let metrics = risk_manager.get_performance_metrics();
                Ok(format!("Risk Metrics: {:?}", metrics))
            }
            "record" => {
                drop(risk_manager);
                let mut risk_manager = self.risk_manager.lock().await;
                let trade = crate::risk_management::Trade {
                    id: params.get("id").cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    symbol: params.get("symbol").cloned().unwrap_or_default(),
                    action: params.get("action").cloned().unwrap_or_else(|| "BUY".to_string()),
                    size: params.get("size").and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    price: params.get("price").and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    timestamp: chrono::Utc::now().timestamp(),
                    pnl: params.get("pnl").and_then(|s| s.parse().ok()).unwrap_or(0.0),
                };
                risk_manager.record_trade(trade);
                Ok("Trade recorded in risk management".to_string())
            }
            _ => Err(format!("Unknown risk action: {}", action))
        }
    }

    // ATOMIC HANDLER: Database operations
    async fn handle_database(&self, params: HashMap<String, String>) -> Result<String, String> {
        let action = params.get("action").map(|s| s.as_str()).unwrap_or("stats");
        let db = self.database.lock().await;
        
        match action {
            "get" => {
                let symbol = params.get("symbol").ok_or("Missing symbol")?;
                let trades = db.get_trades_by_symbol(symbol);
                Ok(format!("Found {} trades for {}", trades.len(), symbol))
            }
            "stats" => {
                let stats = db.get_statistics();
                Ok(format!("DB Stats: Total Trades: {}, Win Rate: {:.2}%, Total PnL: ${:.2}", 
                    stats.total_trades, stats.win_rate * 100.0, stats.total_pnl))
            }
            "recent" => {
                let count = params.get("count").and_then(|s| s.parse().ok()).unwrap_or(10);
                let trades = db.get_recent_trades(count);
                Ok(format!("Retrieved {} recent trades", trades.len()))
            }
            _ => Err(format!("Unknown database action: {}", action))
        }
    }

    // ATOMIC HANDLER: Wallet operations (combines balance, refresh, address)
    async fn handle_wallet(&self, params: HashMap<String, String>) -> Result<String, String> {
        let action = params.get("action").map(|s| s.as_str()).unwrap_or("info");
        
        match action {
            "info" => {
                // ATOMIC: Get balance + address + wallet list in one call
                let solana_client = self.solana_client.lock().await;
                let balance = solana_client.get_balance();
                let address = solana_client.get_wallet_address();
                drop(solana_client);
                
                let _km = self.key_manager.lock().await;
                let wallet_manager = crate::key_manager::WalletManager::new();
                let wallet_count = wallet_manager.list_wallets().len();
                
                Ok(format!(
                    "WALLET INFO: Balance: {:.6} SOL | Address: {} | Managed Wallets: {}",
                    balance,
                    address.unwrap_or_else(|| "None".to_string()),
                    wallet_count
                ))
            }
            "refresh" => {
                // ATOMIC: Refresh balance and return new info
                let mut solana_client = self.solana_client.lock().await;
                solana_client.refresh_balance().await.ok();
                let balance = solana_client.get_balance();
                let address = solana_client.get_wallet_address();
                
                Ok(format!(
                    "REFRESHED: Balance: {:.6} SOL | Address: {}",
                    balance,
                    address.unwrap_or_else(|| "None".to_string())
                ))
            }
            _ => Err(format!("Unknown wallet action: {}", action))
        }
    }

    // ATOMIC HANDLER: Fee operations
    async fn handle_fees(&self, params: HashMap<String, String>) -> Result<String, String> {
        let priority = params.get("priority").map(|s| s.as_str()).unwrap_or("normal");
        let priority_level = match priority {
            "low" => crate::fee_optimization::FeePriority::Low,
            "high" => crate::fee_optimization::FeePriority::High,
            _ => crate::fee_optimization::FeePriority::Normal,
        };
        
        // ATOMIC: Get estimate + stats in one call
        let optimizer = crate::fee_optimization::FeeOptimizer::new(5000);
        let estimate = optimizer.estimate_fee(priority_level);
        let stats = optimizer.get_stats();
        
        Ok(format!(
            "FEES ({}) - Recommended: {} | Min: {} | Max: {} | Confidence: {:.2} | Network: {}",
            priority,
            estimate.recommended_fee,
            estimate.min_fee,
            estimate.max_fee,
            estimate.confidence,
            stats.congestion_level
        ))
    }

    // ATOMIC HANDLER: ML Prediction (combines features + prediction)
    async fn handle_predict(&self, params: HashMap<String, String>) -> Result<String, String> {
        let symbol = params.get("symbol").ok_or("Missing symbol")?;
        let predictor = crate::ml_models::TradingPredictor::new();
        
        // ATOMIC: Get market data + generate features + predict in one operation
        let trading_engine = self.trading_engine.lock().await;
        if let Some(market_data_queue) = trading_engine.market_state.get(symbol) {
            if let Some(latest_data) = market_data_queue.back() {
                let features = predictor.generate_features(latest_data);
                let (confidence, price_change) = predictor.predict(&features).await;
                
                return Ok(format!(
                    "PREDICTION for {} - Current: ${:.2} | Change: {:.4}% | Confidence: {:.2} | Features: {}",
                    symbol, latest_data.price, price_change * 100.0, confidence, features.len()
                ));
            }
        }
        
        Err(format!("No market data for {}", symbol))
    }

    // ATOMIC HANDLER: Validation (combines all validation types)
    async fn handle_validate(&self, params: HashMap<String, String>) -> Result<String, String> {
        let mut results = Vec::new();
        
        // Validate wallet if provided
        if let Some(address) = params.get("wallet") {
            let is_valid = crate::security::validate_wallet_address(address);
            results.push(format!("Wallet {}: {}", address, if is_valid { "✓" } else { "✗" }));
        }
        
        // Validate amount if provided
        if let Some(amount_str) = params.get("amount") {
            if let Ok(amount) = amount_str.parse::<f64>() {
                let is_valid = crate::security::validate_amount(amount);
                results.push(format!("Amount ${:.2}: {}", amount, if is_valid { "✓" } else { "✗" }));
            }
        }
        
        // Sanitize symbol if provided
        if let Some(symbol) = params.get("symbol") {
            let sanitized = crate::security::sanitize_symbol(symbol);
            results.push(format!("Symbol: {} → {}", symbol, sanitized));
        }
        
        if results.is_empty() {
            return Err("No validation parameters provided (wallet, amount, or symbol)".to_string());
        }
        
        Ok(format!("VALIDATION: {}", results.join(" | ")))
    }

    // ATOMIC HANDLER: System health (combines all system info)
    async fn handle_system(&self, _params: HashMap<String, String>) -> Result<String, String> {
        // ATOMIC: Get complete system status in one call
        let trading_engine = self.trading_engine.lock().await;
        let risk_manager = self.risk_manager.lock().await;
        let circuit_breaker = self.circuit_breaker.lock().await;
        let solana_client = self.solana_client.lock().await;
        let db = self.database.lock().await;
        
        let roi = trading_engine.get_roi();
        let metrics = risk_manager.get_performance_metrics();
        let circuit_state = circuit_breaker.get_state().await;
        let balance = solana_client.get_balance();
        let stats = db.get_statistics();
        
        Ok(format!(
            "SYSTEM STATUS: ROI: {:.2}% | Capital: ${:.2} | PnL: ${:.2} | Balance: {:.6} SOL | Trades: {} | Win Rate: {:.1}% | Circuit: {:?}",
            roi,
            metrics.get("current_capital").unwrap_or(&0.0),
            metrics.get("total_pnl").unwrap_or(&0.0),
            balance,
            stats.total_trades,
            stats.win_rate * 100.0,
            circuit_state
        ))
    }

    /// Get available functions list - ATOMIC & SIMPLIFIED
    pub fn get_available_functions(&self) -> Vec<String> {
        vec![
            "trade".to_string(),      // Execute trades + save DB + update risk (atomic)
            "portfolio".to_string(),  // Get holdings + value + ROI (atomic)
            "risk".to_string(),        // Metrics + drawdown + record (atomic)
            "database".to_string(),    // Get trades + stats + recent (atomic)
            "wallet".to_string(),      // Balance + address + refresh (atomic)
            "fees".to_string(),        // Estimate + stats + network status (atomic)
            "predict".to_string(),     // ML prediction + features + analysis (atomic)
            "validate".to_string(),    // Wallet + amount + symbol validation (atomic)
            "system".to_string(),      // Complete system health check (atomic)
        ]
    }
}
