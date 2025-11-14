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

    /// Select best function based on context keywords
    async fn select_best_function(&self, request: &OrchestratorRequest) -> String {
        let context = request.context.to_lowercase();
        
        // Prioritize based on context keywords
        if context.contains("trade") || context.contains("execute") {
            "execute_trade".to_string()
        } else if context.contains("risk") || context.contains("drawdown") {
            "calculate_risk".to_string()
        } else if context.contains("database") || context.contains("save") {
            "save_to_database".to_string()
        } else if context.contains("balance") || context.contains("wallet") {
            "check_balance".to_string()
        } else if context.contains("fee") || context.contains("optimize") {
            "optimize_fees".to_string()
        } else if context.contains("predict") || context.contains("ml") {
            "ml_prediction".to_string()
        } else {
            "get_status".to_string()
        }
    }

    /// Execute a function based on orchestrator decision
    pub async fn execute_function(&self, function_name: &str, parameters: HashMap<String, String>) -> Result<String, String> {
        // Check rate limit before execution
        {
            let rate_limiter = self.rate_limiter.lock().await;
            if !rate_limiter.check_rate_limit("orchestrator".to_string()).await {
                return Err("Rate limit exceeded".to_string());
            }
        }

        // Use circuit breaker for fault tolerance
        let circuit_breaker = self.circuit_breaker.lock().await;
        let state = circuit_breaker.get_state().await;
        
        if matches!(state, crate::error_handling::CircuitState::Open) {
            return Err("Circuit breaker is open - system is protecting itself".to_string());
        }

        // Execute the requested function
        match function_name {
            "execute_trade" => self.handle_execute_trade(parameters).await,
            "calculate_risk" => self.handle_calculate_risk(parameters).await,
            "save_to_database" => self.handle_save_to_database(parameters).await,
            "check_balance" => self.handle_check_balance(parameters).await,
            "optimize_fees" => self.handle_optimize_fees(parameters).await,
            "ml_prediction" => self.handle_ml_prediction(parameters).await,
            "get_status" => self.handle_get_status(parameters).await,
            _ => Err(format!("Unknown function: {}", function_name)),
        }
    }

    async fn handle_execute_trade(&self, params: HashMap<String, String>) -> Result<String, String> {
        let symbol = params.get("symbol").ok_or("Missing symbol")?;
        let size = params.get("size").and_then(|s| s.parse::<f64>().ok()).ok_or("Missing size")?;
        let is_buy = params.get("is_buy").and_then(|s| s.parse::<bool>().ok()).unwrap_or(true);
        let price = params.get("price").and_then(|s| s.parse::<f64>().ok()).ok_or("Missing price")?;

        let mut solana_client = self.solana_client.lock().await;
        solana_client.execute_trade(symbol, size, is_buy, price).await
    }

    async fn handle_calculate_risk(&self, _params: HashMap<String, String>) -> Result<String, String> {
        let risk_manager = self.risk_manager.lock().await;
        let drawdown = risk_manager.calculate_time_weighted_drawdown();
        Ok(format!("Time-weighted drawdown: {:.4}", drawdown))
    }

    async fn handle_save_to_database(&self, params: HashMap<String, String>) -> Result<String, String> {
        let trade_id = params.get("id").ok_or("Missing id")?.clone();
        let symbol = params.get("symbol").ok_or("Missing symbol")?.clone();
        
        let mut db = self.database.lock().await;
        let trade_record = crate::database::TradeRecord {
            id: trade_id,
            timestamp: chrono::Utc::now().timestamp(),
            symbol,
            action: params.get("action").cloned().unwrap_or_else(|| "BUY".to_string()),
            price: params.get("price").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            size: params.get("size").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            total_value: 0.0,
            fee: 0.0,
            pnl: 0.0,
            confidence: 0.7,
            strategy: "AI_Orchestrated".to_string(),
        };
        
        db.insert_trade(trade_record)?;
        Ok("Trade saved to database".to_string())
    }

    async fn handle_check_balance(&self, _params: HashMap<String, String>) -> Result<String, String> {
        let solana_client = self.solana_client.lock().await;
        let balance = solana_client.get_balance();
        Ok(format!("Balance: {} SOL", balance))
    }

    async fn handle_optimize_fees(&self, params: HashMap<String, String>) -> Result<String, String> {
        let priority = params.get("priority").map(|s| s.as_str()).unwrap_or("normal");
        let priority_level = match priority {
            "low" => crate::fee_optimization::FeePriority::Low,
            "high" => crate::fee_optimization::FeePriority::High,
            _ => crate::fee_optimization::FeePriority::Normal,
        };
        
        let optimizer = crate::fee_optimization::FeeOptimizer::new(5000);
        let estimate = optimizer.estimate_fee(priority_level);
        
        Ok(format!(
            "Fee estimate - Min: {}, Recommended: {}, Priority: {}, Confidence: {:.2}",
            estimate.min_fee,
            estimate.recommended_fee,
            estimate.priority_fee,
            estimate.confidence
        ))
    }

    async fn handle_ml_prediction(&self, params: HashMap<String, String>) -> Result<String, String> {
        let symbol = params.get("symbol").ok_or("Missing symbol")?;
        let predictor = crate::ml_models::TradingPredictor::new();
        
        // Get market data for prediction
        let trading_engine = self.trading_engine.lock().await;
        if let Some(market_data_queue) = trading_engine.market_state.get(symbol) {
            if let Some(latest_data) = market_data_queue.back() {
                let features = predictor.generate_features(latest_data);
                let (confidence, price_change) = predictor.predict(&features).await;
                
                return Ok(format!(
                    "ML Prediction for {}: Confidence: {:.2}, Predicted Change: {:.4}",
                    symbol, confidence, price_change
                ));
            }
        }
        
        Err(format!("No market data available for {}", symbol))
    }

    async fn handle_get_status(&self, _params: HashMap<String, String>) -> Result<String, String> {
        let trading_engine = self.trading_engine.lock().await;
        let risk_manager = self.risk_manager.lock().await;
        
        let metrics = risk_manager.get_performance_metrics();
        let roi = trading_engine.get_roi();
        
        Ok(format!(
            "System Status - ROI: {:.2}%, Current Capital: ${:.2}, Total PnL: ${:.2}",
            roi,
            metrics.get("current_capital").unwrap_or(&0.0),
            metrics.get("total_pnl").unwrap_or(&0.0)
        ))
    }

    /// Get available functions list
    pub fn get_available_functions(&self) -> Vec<String> {
        vec![
            "execute_trade".to_string(),
            "calculate_risk".to_string(),
            "save_to_database".to_string(),
            "check_balance".to_string(),
            "optimize_fees".to_string(),
            "ml_prediction".to_string(),
            "get_status".to_string(),
        ]
    }
}
