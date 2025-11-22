mod trading_engine;
mod solana_integration;
mod risk_management;
mod ml_models;
mod api;
mod jupiter_integration;
mod security;
mod websocket;
mod deepseek_ai;
mod error_handling;
mod fee_optimization;
mod key_manager;
mod database;
mod switchboard_oracle;
mod dex_screener;
mod pumpfun;
mod autonomous_agent;
mod signal_platform;
mod specialized_providers;
mod reinforcement_learning;
mod secure_config;
mod enhanced_marketplace;
mod historical_data;
mod wallet;
mod pda;
mod rpc_client;
mod quant_analysis;
mod jito_bam;
mod ai_orchestrator;
mod api_v2;
mod live_data_feed;
mod http_client;
mod twitter_sentiment;
mod backtesting;
mod production_safeguards;

#[cfg(test)]
mod algorithm_tests;

use std::sync::Arc;
use tokio::sync::Mutex;
use futures::FutureExt; // For catch_unwind

/// Auto-execute high-confidence signals from marketplace
async fn auto_execute_marketplace_signals(
    trading_engine: Arc<Mutex<trading_engine::TradingEngine>>,
    marketplace: Arc<signal_platform::SignalMarketplace>,
    enhanced_marketplace: Arc<enhanced_marketplace::EnhancedMarketplace>,
    _oracle_client: Arc<switchboard_oracle::SwitchboardClient>,
    trading_enabled: Arc<Mutex<bool>>,
    dry_run: Arc<Mutex<bool>>,
    rl_coordinator: Arc<Mutex<reinforcement_learning::LearningCoordinator>>,
) {
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🚀 AUTONOMOUS TRADING SERVICE STARTED");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("📊 Monitoring marketplace for high-confidence signals (≥75%)");
    log::info!("⏱️  Check interval: 30 seconds");
    log::info!("💡 Signals are published to marketplace AND executed autonomously");
    log::info!("🔗 Using REAL Solana transactions via Jupiter API");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30)); // Check every 30 seconds
    let min_confidence = 0.75; // Auto-execute signals with 75%+ confidence
    let mut execution_count = 0u64;
    let mut last_check_time = std::time::Instant::now();
    
    // CRASH PROTECTION: Wrap main loop in panic handler
    let mut consecutive_errors = 0u32;
    let max_consecutive_errors = 10u32;
    
    loop {
        // CRASH PROTECTION: Catch panics in auto-execution loop
        let result = std::panic::AssertUnwindSafe(async {
            interval.tick().await;
            
            // Check if trading is enabled
            let is_enabled = {
                let enabled = trading_enabled.lock().await;
                *enabled
            };
            
            if !is_enabled {
                if last_check_time.elapsed().as_secs() > 60 {
                    log::debug!("⏸️  Trading is disabled - waiting for enable signal");
                    last_check_time = std::time::Instant::now();
                }
                return; // Skip execution if trading is disabled
            }
            
            // Get executable signals from marketplace
            let signals = marketplace.get_executable_signals(min_confidence).await;
            
            if signals.is_empty() {
                return;
            }
            
            log::info!("🔍 Found {} high-confidence signals ready for auto-execution", signals.len());
            
            for signal in signals {
                log::info!("📈 Processing signal: {} | Symbol: {} | Confidence: {:.1}% | Provider: {}", 
                          signal.id, signal.symbol, signal.confidence * 100.0, signal.provider);
                
                // FIX #1: Atomic status update - mark as Executing BEFORE execution to prevent duplicate execution
                match marketplace.try_mark_executing(&signal.id).await {
                    Ok(true) => {
                        log::debug!("🔒 Acquired execution lock for signal {}", signal.id);
                    }
                    Ok(false) => {
                        log::warn!("⚠️ Signal {} already being processed by another task, skipping", signal.id);
                        continue; // Already being processed
                    }
                    Err(e) => {
                        log::warn!("⚠️ Failed to mark signal {} as Executing: {}", signal.id, e);
                        continue;
                    }
                }
                
                // Initialize performance tracking
                if let Err(e) = enhanced_marketplace.initialize_signal_performance(&signal).await {
                    log::warn!("⚠️ Failed to initialize performance tracking for {}: {}", signal.id, e);
                    // FIX #1: Revert status on failure
                    let _ = marketplace.update_signal_status(&signal.id, signal_platform::SignalStatus::Active).await;
                    continue;
                }
                
                // Execute signal via trading engine (REAL Solana transactions)
                let mut engine = trading_engine.lock().await;
                match engine.execute_marketplace_signal(&signal, Some(&trading_enabled), Some(&dry_run)).await {
                    Ok(result) => {
                        execution_count += 1;
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("✅ AUTONOMOUS TRADE EXECUTED #{}", execution_count);
                        log::info!("   Signal ID: {}", signal.id);
                        log::info!("   Symbol: {}", signal.symbol);
                        log::info!("   Action: {:?}", signal.action);
                        log::info!("   Entry Price: ${:.8}", signal.entry_price);
                        log::info!("   Target Price: ${:.8}", signal.target_price);
                        log::info!("   Stop Loss: ${:.8}", signal.stop_loss);
                        log::info!("   Result: {}", result);
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        
                        // Mark signal as filled
                        if let Err(e) = enhanced_marketplace.mark_signal_filled(&signal.id).await {
                            log::warn!("⚠️ Failed to mark signal as filled: {}", e);
                        }
                        
                        // FIX #1: Update marketplace signal status to Filled after successful execution
                        if let Err(e) = marketplace.update_signal_status(&signal.id, signal_platform::SignalStatus::Filled).await {
                            log::warn!("⚠️ Failed to update signal status: {}", e);
                        }
                        
                        // RL LEARNING: Update RL agents with trade outcome
                        // Record experience via LearningCoordinator
                        let coordinator = rl_coordinator.lock().await;
                        // Calculate position size from signal (estimate based on confidence)
                        let estimated_size = signal.entry_price * 0.05; // 5% of entry price as default size
                        
                        // Create experience for RL learning
                        let experience = reinforcement_learning::Experience {
                            state: reinforcement_learning::MarketState {
                                symbol: signal.symbol.clone(),
                                price: signal.entry_price,
                                volume: 0.0,
                                price_change_1h: 0.0,
                                price_change_24h: 0.0,
                                sentiment_score: signal.confidence * 100.0,
                                liquidity: 0.0,
                                volatility: 0.0,
                                market_cap: None,
                            },
                            action: reinforcement_learning::Action {
                                action_type: format!("{:?}", signal.action),
                                confidence: signal.confidence,
                                size: estimated_size,
                                price: signal.entry_price,
                            },
                            reward: 0.0, // Will be updated when position closes
                            next_state: None,
                            timestamp: chrono::Utc::now().timestamp(),
                            provider_id: signal.provider.clone(),
                        };
                        
                        // Record experience via coordinator (it will route to the appropriate agent)
                        coordinator.record_experience_for_provider(&signal.provider, experience).await;
                        log::debug!("🧠 Recorded trade experience for RL agent: {}", signal.provider);
                        drop(coordinator);
                        
                        // Note: Provider reputation is updated when signal performance is tracked
                        // in the track_signal_performance function based on actual outcomes
                    }
                    Err(e) => {
                        log::warn!("⚠️ Failed to auto-execute signal {}: {}", signal.id, e);
                        log::warn!("   Signal remains in marketplace for manual execution");
                        // Revert status to Active on failure (validated transition)
                        match marketplace.update_signal_status(&signal.id, signal_platform::SignalStatus::Active).await {
                            Ok(_) => {
                                log::info!("🔄 Signal {} reverted to Active for retry", signal.id);
                            }
                            Err(revert_err) => {
                                log::warn!("⚠️ Failed to revert signal {} status to Active: {}", signal.id, revert_err);
                                log::warn!("   Signal may be stuck in Executing state - manual intervention may be needed");
                            }
                        }
                    }
                }
            }
        }).catch_unwind().await;
        
        match result {
            Ok(_) => {
                consecutive_errors = 0; // Reset error counter on success
            }
            Err(_panic) => {
                consecutive_errors += 1;
                log::error!("💥 PANIC in auto-execute loop (consecutive: {}) - Recovering...", consecutive_errors);
                
                // Exponential backoff after panic
                let backoff = std::cmp::min(30u64 * consecutive_errors as u64, 300); // Max 5 minutes
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff)).await;
                
                if consecutive_errors >= max_consecutive_errors {
                    log::error!("🛑 Too many panics in auto-execute. Waiting 5 minutes...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                    consecutive_errors = 0;
                }
            }
        }
    }
}

/// Track signal performance with real-time price updates
async fn track_signal_performance(
    marketplace: Arc<signal_platform::SignalMarketplace>,
    enhanced_marketplace: Arc<enhanced_marketplace::EnhancedMarketplace>,
    _oracle_client: Arc<switchboard_oracle::SwitchboardClient>,
    rl_coordinator: Arc<Mutex<reinforcement_learning::LearningCoordinator>>,
) {
    log::info!("📊 Signal Performance Tracker started - updating prices every 10 seconds");
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
    let mut consecutive_errors = 0u32;
    let max_consecutive_errors = 10u32;
    
    // CRASH PROTECTION: Wrap main loop in panic handler
    loop {
        // CRASH PROTECTION: Catch panics in performance tracking loop
        let result = std::panic::AssertUnwindSafe(async {
            interval.tick().await;
            
            // Get all active signals
            let active_signals = marketplace.get_active_signals().await;
            
            for signal in active_signals {
                // Get current price from oracle or other sources
                let current_price = match _oracle_client.fetch_price(&signal.symbol).await {
                    Ok(feed) => feed.price,
                    Err(_) => {
                        // Try to get price from signal's entry price as fallback
                        signal.entry_price
                    }
                };
                
                // Update performance tracking
                if let Err(e) = enhanced_marketplace.update_signal_performance(&signal.id, current_price).await {
                    log::debug!("Could not update performance for {}: {}", signal.id, e);
                }
                
                // Check if target or stop loss hit
                let perf = match enhanced_marketplace.get_signal_performance(&signal.id).await {
                    Some(p) => p,
                    None => continue,
                };
                
                // Auto-close if target or stop loss reached
                let target_pct = (signal.target_price - signal.entry_price) / signal.entry_price * 100.0;
                let stop_loss_pct = (signal.stop_loss - signal.entry_price) / signal.entry_price * 100.0;
                
                if perf.profit_loss_pct >= target_pct {
                    // Target reached - close with profit
                    if let Ok(_closed_perf) = enhanced_marketplace.close_signal_position(&signal.id, current_price).await {
                        log::info!("🎯 Signal {} target reached! Closed with {:.2}% profit", signal.id, perf.profit_loss_pct);
                        marketplace.update_signal_status(&signal.id, signal_platform::SignalStatus::Filled).await.ok();
                        
                        // REPUTATION UPDATE: Already handled in close_signal_position
                        // Enhanced marketplace automatically updates provider reputation with:
                        // - Profit percentage bonus
                        // - Confidence accuracy
                        // - Target achievement bonus
                        // - Timing bonus
                        // - Consistency bonus
                        
                        // RL LEARNING: Update agent with successful outcome
                        let coordinator = rl_coordinator.lock().await;
                        let reward = perf.profit_loss_pct; // Positive reward = profit percentage
                        let experience = reinforcement_learning::Experience {
                            state: reinforcement_learning::MarketState {
                                symbol: signal.symbol.clone(),
                                price: signal.entry_price,
                                volume: 0.0,
                                price_change_1h: 0.0,
                                price_change_24h: 0.0,
                                sentiment_score: signal.confidence * 100.0,
                                liquidity: 0.0,
                                volatility: 0.0,
                                market_cap: None,
                            },
                            action: reinforcement_learning::Action {
                                action_type: format!("{:?}", signal.action),
                                confidence: signal.confidence,
                                size: (signal.target_price - signal.entry_price) * 0.05, // Estimate size
                                price: signal.entry_price,
                            },
                            reward: reward * 100.0, // Scale reward (profit percentage)
                            next_state: Some(reinforcement_learning::MarketState {
                                symbol: signal.symbol.clone(),
                                price: current_price,
                                volume: 0.0,
                                price_change_1h: 0.0,
                                price_change_24h: 0.0,
                                sentiment_score: signal.confidence * 100.0,
                                liquidity: 0.0,
                                volatility: 0.0,
                                market_cap: None,
                            }),
                            timestamp: chrono::Utc::now().timestamp(),
                            provider_id: signal.provider.clone(),
                        };
                        coordinator.record_experience_for_provider(&signal.provider, experience).await;
                        log::debug!("🧠 Recorded successful trade outcome for RL agent: {} (profit: {:.2}%)", signal.provider, perf.profit_loss_pct);
                        drop(coordinator);
                    }
                } else if perf.profit_loss_pct <= stop_loss_pct {
                    // Stop loss hit - close with loss
                    if let Ok(_closed_perf) = enhanced_marketplace.close_signal_position(&signal.id, current_price).await {
                        log::warn!("🛑 Signal {} stop loss hit! Closed with {:.2}% loss", signal.id, perf.profit_loss_pct);
                        marketplace.update_signal_status(&signal.id, signal_platform::SignalStatus::Filled).await.ok();
                        
                        // REPUTATION UPDATE: Already handled in close_signal_position
                        // Enhanced marketplace automatically updates provider reputation with:
                        // - Loss percentage penalty
                        // - Overconfidence penalty (if high confidence but failed)
                        // - Failure penalty
                        
                        // RL LEARNING: Update agent with failed outcome
                        let coordinator = rl_coordinator.lock().await;
                        let reward = perf.profit_loss_pct; // Negative reward = loss percentage
                        let experience = reinforcement_learning::Experience {
                            state: reinforcement_learning::MarketState {
                                symbol: signal.symbol.clone(),
                                price: signal.entry_price,
                                volume: 0.0,
                                price_change_1h: 0.0,
                                price_change_24h: 0.0,
                                sentiment_score: signal.confidence * 100.0,
                                liquidity: 0.0,
                                volatility: 0.0,
                                market_cap: None,
                            },
                            action: reinforcement_learning::Action {
                                action_type: format!("{:?}", signal.action),
                                confidence: signal.confidence,
                                size: (signal.entry_price - signal.stop_loss) * 0.05, // Estimate size
                                price: signal.entry_price,
                            },
                            reward: reward * 100.0, // Negative reward (loss percentage)
                            next_state: Some(reinforcement_learning::MarketState {
                                symbol: signal.symbol.clone(),
                                price: current_price,
                                volume: 0.0,
                                price_change_1h: 0.0,
                                price_change_24h: 0.0,
                                sentiment_score: signal.confidence * 100.0,
                                liquidity: 0.0,
                                volatility: 0.0,
                                market_cap: None,
                            }),
                            timestamp: chrono::Utc::now().timestamp(),
                            provider_id: signal.provider.clone(),
                        };
                        coordinator.record_experience_for_provider(&signal.provider, experience).await;
                        log::debug!("🧠 Recorded failed trade outcome for RL agent: {} (loss: {:.2}%)", signal.provider, perf.profit_loss_pct);
                        drop(coordinator);
                    }
                }
            }
        }).catch_unwind().await;
        
        match result {
            Ok(_) => {
                consecutive_errors = 0; // Reset error counter on success
            }
            Err(_panic) => {
                consecutive_errors += 1;
                log::error!("💥 PANIC in performance tracking loop (consecutive: {}) - Recovering...", consecutive_errors);
                
                // Exponential backoff after panic
                let backoff = std::cmp::min(30u64 * consecutive_errors as u64, 300); // Max 5 minutes
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff)).await;
                
                if consecutive_errors >= max_consecutive_errors {
                    log::error!("🛑 Too many panics in performance tracking. Waiting 5 minutes...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                    consecutive_errors = 0;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    pretty_env_logger::init();
    
    // SAFETY: Check for dry-run mode
    let dry_run_mode = std::env::var("DRY_RUN_MODE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true); // Default to true for safety
    
    // SAFETY: Check if trading should be enabled
    let env_trading_enabled = std::env::var("ENABLE_TRADING")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false); // Default to false for safety
    
    log::info!("🚀 Starting AgentBurn Solana Trading System...");
    log::info!("🤖 Enhanced with Switchboard Oracle, Mobula API, and PumpFun integrations");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Get RPC URL and detect network BEFORE validation
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let is_mainnet = rpc_url.contains("mainnet");
    
    // PRODUCTION READINESS: Startup validation
    log::info!("🔍 Running startup validation checks...");
    let mut validation_errors: Vec<String> = Vec::new();
    let mut validation_warnings: Vec<String> = Vec::new();
    
    // Validate critical configuration
    if !dry_run_mode && env_trading_enabled {
        validation_warnings.push("⚠️  REAL TRADING MODE: Both DRY_RUN_MODE=false and ENABLE_TRADING=true - Real funds will be used!".to_string());
    }
    
    if !dry_run_mode && is_mainnet {
        validation_warnings.push("⚠️  MAINNET + REAL TRADING: This will execute real trades on mainnet!".to_string());
    }
    
    // Check RPC connectivity (basic check)
    let rpc_check = std::env::var("SOLANA_RPC_URL").is_ok();
    if !rpc_check {
        validation_warnings.push("⚠️  SOLANA_RPC_URL not set - using default devnet endpoint".to_string());
    }
    
    // Log validation results
    if !validation_errors.is_empty() {
        log::error!("❌ Startup validation FAILED:");
        for error in &validation_errors {
            log::error!("   {}", error);
        }
        log::error!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::error!("System will continue but may not function correctly.");
    }
    
    if !validation_warnings.is_empty() {
        log::warn!("⚠️  Startup validation warnings:");
        for warning in &validation_warnings {
            log::warn!("   {}", warning);
        }
    }
    
    if validation_errors.is_empty() && validation_warnings.is_empty() {
        log::info!("✅ All startup validation checks passed");
    }
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Log safety configuration
    if dry_run_mode {
        log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::warn!("🧪 DRY-RUN MODE ENABLED - NO REAL TRADES WILL BE EXECUTED");
        log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::warn!("All trades will be simulated. Set DRY_RUN_MODE=false to enable real trading.");
        log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        log::warn!("⚠️  DRY-RUN MODE DISABLED - REAL TRADES WILL BE EXECUTED");
        log::warn!("   This will use REAL funds from your PDA treasury!");
    }
    
    if env_trading_enabled {
        log::warn!("⚠️  Trading is ENABLED via ENABLE_TRADING environment variable");
        log::warn!("   This will execute trades automatically!");
    } else {
        log::info!("🔒 Trading is DISABLED by default for safety");
        log::info!("   To enable: POST /trading-toggle with {{\"enabled\": true}}");
        log::info!("   Or set ENABLE_TRADING=true in .env");
    }

    // Network name for logging (is_mainnet already defined above)
    let network_name = if is_mainnet { "MAINNET" } else { "DEVNET" };
    
    log::info!("🌐 Network Configuration:");
    log::info!("   Network: {} ({})", network_name, rpc_url);
    
    // Check API keys and log status
    log::info!("🔑 API Key Status:");
    let has_rpc = std::env::var("SOLANA_RPC_URL").is_ok();
    let has_deepseek = std::env::var("DEEPSEEK_API_KEY").is_ok();
    let has_mobula = std::env::var("MOBULA_API_KEY").is_ok();
    let has_moralis = std::env::var("MORALIS_API_KEY").is_ok();
    let has_jupiter = std::env::var("JUPITER_API_KEY").is_ok();
    
    log::info!("   SOLANA_RPC_URL: {}", if has_rpc { "✅ Configured" } else { "❌ Not set (using devnet default)" });
    log::info!("   DEEPSEEK_API_KEY: {}", if has_deepseek { "✅ Configured" } else { "⚠️  Not set (AI analysis disabled)" });
    log::info!("   MOBULA_API_KEY: {}", if has_mobula { "✅ Configured" } else { "⚠️  Not set (using free tier)" });
    log::info!("   MORALIS_API_KEY: {}", if has_moralis { "✅ Configured" } else { "⚠️  Not set (PumpFun prices simulated)" });
    log::info!("   JUPITER_API_KEY: {}", if has_jupiter { "✅ Configured" } else { "⚠️  Not set (using public API)" });
    
    if is_mainnet {
        log::warn!("⚠️  MAINNET MODE - Real funds will be used!");
        log::warn!("⚠️  Ensure all API keys and wallet are properly configured!");
    } else {
        log::info!("ℹ️  DEVNET MODE - Using test network (safe for testing)");
    }
    
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Initialize Database for persistence
    log::info!("💾 Initializing Database...");
    let database = Arc::new(Mutex::new(database::Database::new("trades.db")));
    
    // Initialize Key Manager for secure wallet operations
    log::info!("🔐 Initializing Key Manager...");
    let key_manager = Arc::new(Mutex::new(key_manager::KeyManager::new(false))); // encryption disabled for dev
    
    // Initialize Security Rate Limiter
    log::info!("🛡️ Initializing Security Rate Limiter...");
    let rate_limiter = Arc::new(Mutex::new(security::RateLimiter::new(100, std::time::Duration::from_secs(60))));
    
    // Initialize DeepSeek AI Client (if API key is set)
    let deepseek_client = if let Ok(api_key) = std::env::var("DEEPSEEK_API_KEY") {
        log::info!("🧠 Initializing DeepSeek AI Client...");
        Some(Arc::new(Mutex::new(deepseek_ai::DeepSeekClient::new(api_key))))
    } else {
        log::warn!("⚠️ DEEPSEEK_API_KEY not set - AI analysis disabled");
        None
    };
    
    // Initialize Error Handling Circuit Breaker
    log::info!("⚡ Initializing Circuit Breaker...");
    let circuit_breaker = Arc::new(Mutex::new(
        error_handling::CircuitBreaker::new(5, 3, std::time::Duration::from_secs(60))
    ));
    
    // Initialize Solana client with wallet and PDA integration
    let solana_client = Arc::new(Mutex::new(
        solana_integration::SolanaClient::new_with_integration(rpc_url.clone()).await
    ));

    let risk_manager = Arc::new(Mutex::new(risk_management::RiskManager::new(10000.0, 0.1)));
    
    // Initialize Fee Optimizer for transaction fee tracking
    log::info!("💰 Initializing Fee Optimizer...");
    let fee_optimizer = Arc::new(Mutex::new(fee_optimization::FeeOptimizer::new(5000))); // Base fee: 5000 lamports
    
    // Initialize Trading Engine with REAL Solana integration
    let trading_engine = Arc::new(Mutex::new(
        trading_engine::TradingEngine::new_with_solana(
            risk_manager.clone(),
            solana_client.clone(),
            Some(Arc::new(jupiter_integration::JupiterClient::new())),
            Some(fee_optimizer.clone()), // PASS: Fee optimizer for transaction tracking
        )
    ));
    
    // Sync initial balance from PDA
    {
        let mut engine = trading_engine.lock().await;
        engine.sync_balance_from_pda().await;
        log::info!("💰 Trading engine initialized with balance: {:.6} SOL", engine.current_balance);
    }

    // Initialize WebSocket broadcaster for real-time updates
    log::info!("📡 Initializing WebSocket broadcaster...");
    let ws_broadcaster = websocket::create_ws_broadcaster();
    
    // Initialize Reinforcement Learning Coordinator
    log::info!("🤖 Initializing RL Coordinator...");
    let rl_coordinator = Arc::new(Mutex::new(reinforcement_learning::LearningCoordinator::new()));
    
    // Initialize Twitter Sentiment Client (optional - service may not be running)
    log::info!("🐦 Initializing Twitter Sentiment Client...");
    let twitter_sentiment_url = std::env::var("TWITTER_SENTIMENT_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8000".to_string());
    let twitter_sentiment_client = Arc::new(twitter_sentiment::TwitterSentimentClient::new(twitter_sentiment_url.clone()));
    
    // Check if Twitter sentiment service is available
    match twitter_sentiment_client.health_check().await {
        Ok(true) => {
            log::info!("✅ Twitter Sentiment service is available at {}", twitter_sentiment_url);
        }
        Ok(false) | Err(_) => {
            log::warn!("⚠️  Twitter Sentiment service not available at {} - sentiment analysis will use base analysis only", twitter_sentiment_url);
            log::info!("💡 To enable Twitter sentiment: Start the Python service (see sentiment_service/README.md)");
        }
    }
    
    // Initialize Meme Analyzer for memecoin analysis
    log::info!("🎪 Initializing Meme Analyzer...");
    let meme_analyzer = Arc::new(Mutex::new(pumpfun::MemeAnalyzer::new()));
    
    // Initialize PumpFun client with WebSocket and scraping support
    log::info!("🚀 Initializing PumpFun client with WebSocket and page scraping...");
    let pumpfun_client_shared = Arc::new(pumpfun::PumpFunClient::new());
    
    // Start PumpFun WebSocket listener for real-time token updates
    let pumpfun_ws_client = pumpfun_client_shared.clone();
    let _pumpfun_ws_handle = pumpfun_ws_client.start_websocket_listener();
    log::info!("✅ PumpFun WebSocket listener started (will connect when available)");
    
    // Start periodic pump.fun page scraping for trading opportunities
    let pumpfun_scraper = pumpfun_client_shared.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Scrape every minute
        loop {
            interval.tick().await;
            match pumpfun_scraper.scrape_trading_opportunities().await {
                Ok(tokens) => {
                    if !tokens.is_empty() {
                        log::info!("📊 Scraped {} tokens from pump.fun page", tokens.len());
                    }
                }
                Err(e) => {
                    log::debug!("Could not scrape pump.fun page: {}", e);
                }
            }
        }
    });
    log::info!("✅ PumpFun page scraper started (scrapes every 60 seconds)");
    
    // Initialize X402 Signal Platform
    log::info!("📡 Initializing X402 Signal Platform...");
    let signal_platform = Arc::new(Mutex::new(signal_platform::SignalMarketplace::new(rpc_url.clone())));
    
    // Initialize AI Orchestrator that coordinates all systems with DeepSeek intelligence
    log::info!("🤖 Initializing AI Orchestrator...");
    let ai_orchestrator = Arc::new(ai_orchestrator::AIOrchestrator::new(
        deepseek_client.clone(),
        database.clone(),
        rate_limiter.clone(),
        key_manager.clone(),
        circuit_breaker.clone(),
        trading_engine.clone(),
        risk_manager.clone(),
        solana_client.clone(),
        Some(ws_broadcaster.clone()),
        rl_coordinator.clone(),
        meme_analyzer.clone(),
        signal_platform.clone(),
    ));
    log::info!("✅ AI Orchestrator ready with {} available functions", ai_orchestrator.get_available_functions().len());

    // Start periodic rate limiter cleanup (uses previously unused cleanup() method)
    let cleanup_limiter = rate_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // Every 5 minutes
            cleanup_limiter.lock().await.cleanup().await;
            log::debug!("🧹 Rate limiter cleanup completed");
        }
    });

    // Start periodic PDA balance sync (ensures agents always use real balance)
    let solana_client_sync = solana_client.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await; // Every 30 seconds
            let mut client_lock = solana_client_sync.lock().await;
            client_lock.sync_trading_budget_from_pda().await;
            log::debug!("🔄 Synced trading budget from REAL PDA balance: {:.6} SOL", client_lock.trading_budget);
        }
    });

    // Start market data simulation ONLY if no real RPC URL is configured
    let use_real_data = std::env::var("SOLANA_RPC_URL").is_ok();
    if !use_real_data {
        log::warn!("⚠️  No SOLANA_RPC_URL configured - using simulated market data");
        log::info!("📊 Starting market data simulation (add SOLANA_RPC_URL to .env for real data)");
    let market_engine = trading_engine.clone();
    tokio::spawn(async move {
        solana_integration::simulate_market_data(market_engine).await;
    });
    } else {
        log::info!("✅ Real market data enabled via SOLANA_RPC_URL");
        log::info!("📊 Market data will be fetched from real sources (Switchboard, CoinGecko, etc.)");
    }

    // Start traditional signal generation
    let signal_engine = trading_engine.clone();
    let signal_risk = risk_manager.clone();
    tokio::spawn(async move {
        trading_engine::generate_trading_signals(signal_engine, signal_risk).await;
    });

    // Initialize Signal Marketplace
    let marketplace = Arc::new(signal_platform::SignalMarketplace::new(rpc_url.clone()));
    
    // Start automatic signal cleanup task (runs every hour)
    log::info!("🧹 Starting automatic signal cleanup service...");
    let marketplace_cleanup = marketplace.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Every hour
        loop {
            interval.tick().await;
            marketplace_cleanup.cleanup_expired_signals().await;
            log::debug!("🔄 Signal cleanup cycle completed");
        }
    });
    
    // Initialize Enhanced Marketplace with performance tracking
    let enhanced_marketplace = Arc::new(enhanced_marketplace::EnhancedMarketplace::new(marketplace.clone()));
    
    // Trading state management (shared across all services)
    // SAFETY: Default to DISABLED - user must explicitly enable
    let trading_enabled = Arc::new(Mutex::new(env_trading_enabled));
    
    // Store dry-run mode in a shared Arc for access throughout the system
    // Note: Currently not passed to all functions, but kept for future use
    let _dry_run = Arc::new(Mutex::new(dry_run_mode));
    
    // Start auto-execution service for high-confidence marketplace signals
    log::info!("🤖 Starting Auto-Execution Service for Marketplace Signals...");
    let auto_exec_engine = trading_engine.clone();
    let auto_exec_marketplace = marketplace.clone();
    let auto_exec_enhanced = enhanced_marketplace.clone();
    let auto_exec_oracle = Arc::new(switchboard_oracle::SwitchboardClient::new(rpc_url.clone(), true));
    let auto_exec_trading_enabled = trading_enabled.clone();
    let auto_exec_dry_run = Arc::new(Mutex::new(dry_run_mode)); // PASS: dry-run mode for paper trading
    let auto_exec_rl_coordinator = rl_coordinator.clone(); // PASS: RL coordinator for learning
    tokio::spawn(async move {
        auto_execute_marketplace_signals(
            auto_exec_engine,
            auto_exec_marketplace,
            auto_exec_enhanced,
            auto_exec_oracle,
            auto_exec_trading_enabled,
            auto_exec_dry_run, // PASS: dry-run mode
            auto_exec_rl_coordinator, // PASS: RL coordinator
        ).await;
    });
    
    // Start real-time signal performance tracking
    log::info!("📊 Starting Real-Time Signal Performance Tracker...");
    let perf_tracker_marketplace = marketplace.clone();
    let perf_tracker_enhanced = enhanced_marketplace.clone();
    let perf_tracker_oracle = Arc::new(switchboard_oracle::SwitchboardClient::new(rpc_url.clone(), true));
    let perf_tracker_rl_coordinator = rl_coordinator.clone(); // PASS: RL coordinator for learning from outcomes
    tokio::spawn(async move {
        track_signal_performance(
            perf_tracker_marketplace,
            perf_tracker_enhanced,
            perf_tracker_oracle,
            perf_tracker_rl_coordinator, // PASS: RL coordinator
        ).await;
    });
    
    // Initialize Oracle Client for live data feed
    log::info!("📊 Initializing Oracle Client for Live Data Feed...");
    let oracle_client = Arc::new(switchboard_oracle::SwitchboardClient::new(rpc_url.clone(), true));
    
    // Initialize and start 24/7 Live Data Feed Service
    log::info!("📡 Initializing 24/7 Live Data Feed Service...");
    let live_feed_symbols = vec![
        "SOL/USD".to_string(),
        "BTC/USD".to_string(),
        "ETH/USD".to_string(),
        "USDC/USD".to_string(),
    ];
    
    // Get Jupiter client for volume data (if available)
    let jupiter_client_for_feed = {
        let engine_lock = trading_engine.lock().await;
        engine_lock.jupiter_client.clone()
    }; // Lock released here
    
    let live_data_feed = Arc::new(live_data_feed::LiveDataFeed::new(
        oracle_client.clone(),
        Some(ws_broadcaster.clone()),
        Some(trading_engine.clone()), // PASS: Trading engine to update market_state with REAL prices
        jupiter_client_for_feed, // PASS: Jupiter client for volume data
        live_feed_symbols.clone(),
    ));
    live_data_feed.start().await;
    log::info!("✅ 24/7 Live Data Feed Service started - monitoring {} symbols", live_feed_symbols.len());
    log::info!("   Symbols: {:?}", live_feed_symbols);
    log::info!("   Updates every 5 seconds via WebSocket");
    log::info!("   🔄 TradingEngine.market_state will be updated with REAL prices from Switchboard Oracle");
    
    // Initialize 7 Specialized Provider Agents with RL integration
    // 1. Memecoin Monitor, 2. Oracle Monitor, 3. Jupiter Memecoin Trader, 4. Jupiter Blue Chip Trader
    // 5. Opportunity Analyzer, 6. Signal Trader, 7. Master Analyzer
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🤖 INITIALIZING SPECIALIZED SIGNAL PROVIDERS");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("📊 Providers will:");
    log::info!("   1. Generate trading signals from market data");
    log::info!("   2. Publish signals to marketplace (available for purchase)");
    log::info!("   3. Signals with ≥75% confidence auto-execute autonomously");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let providers = specialized_providers::initialize_all_providers(
        marketplace.clone(),
        rpc_url.clone(),
    ).await;
    
    // Connect each provider to RL coordinator for centralized learning
    let mut rl_connected_providers = Vec::new();
    for provider in providers {
        let enhanced_provider = provider.with_rl_coordinator(rl_coordinator.clone());
        rl_connected_providers.push(enhanced_provider);
    }
    
    log::info!("✅ Initialized {} specialized providers with RL integration", rl_connected_providers.len());
    log::info!("   Providers: Memecoin Monitor, Oracle Monitor, Jupiter Memecoin Trader,");
    log::info!("              Jupiter Blue Chip Trader, Opportunity Analyzer, Signal Trader, Master Analyzer");
    
    // Start each specialized provider in its own task
    // Each provider runs independently and handles its own errors
    for provider in rl_connected_providers {
        tokio::spawn(async move {
            // Run provider in its own task
            // All errors are converted to String internally, so this is Send-safe
            provider.run().await;
        });
    }
    
    log::info!("✅ All providers started - generating and publishing signals to marketplace");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Start autonomous trading agent (legacy)
    let agent_engine = trading_engine.clone();
    let agent_risk = risk_manager.clone();
    let agent_rpc = rpc_url.clone();
    
    log::info!("🤖 Starting Legacy Autonomous Trading Agent...");
    tokio::spawn(async move {
        let agent = autonomous_agent::AutonomousAgent::new(
            agent_rpc,
            agent_engine,
            agent_risk,
        );
        agent.run().await;
    });

    // Start both APIs in parallel
    let api_engine = trading_engine.clone();
    let api_risk = risk_manager.clone();
    let api_solana = solana_client.clone();
    let api_orchestrator = ai_orchestrator.clone();
    
    log::info!("🌐 Starting Legacy API on port 8080 and AI-Orchestrated API v2 on port 8081...");
    
    // Start legacy API in background
    let api_trading_enabled = trading_enabled.clone();
    let api_rl_coordinator = rl_coordinator.clone(); // PASS: RL coordinator for agent learning metrics
    let api_circuit_breaker = circuit_breaker.clone(); // PASS: Circuit breaker for API protection
    let api_live_data_feed = live_data_feed.clone(); // PASS: Live data feed for management
    let api_enhanced_marketplace = enhanced_marketplace.clone(); // PASS: Enhanced marketplace for advanced features
    let legacy_api = tokio::spawn(async move {
        api::start_server(api_engine, api_risk, api_solana, api_trading_enabled, Some(api_rl_coordinator), Some(api_circuit_breaker), Some(api_live_data_feed), Some(api_enhanced_marketplace)).await;
    });
    
    // Start new AI-orchestrated API v2 in background
    let ai_api = tokio::spawn(async move {
        api_v2::start_server(api_orchestrator).await;
    });
    
    // Wait for both servers (they run forever)
    let _ = tokio::try_join!(legacy_api, ai_api);
}
