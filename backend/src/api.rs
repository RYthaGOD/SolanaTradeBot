use warp::Filter;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::jupiter_integration::JupiterClient;
use crate::websocket::{create_ws_broadcaster, handle_websocket};
use crate::switchboard_oracle::SwitchboardClient;
use crate::dex_screener::DexScreenerClient;
use crate::pumpfun::PumpFunClient;
use crate::signal_platform::SignalMarketplace;

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: String,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T, message: &str) -> Self {
        Self {
            success: true,
            data,
            message: message.to_string(),
        }
    }
}

// Helper function for safe JSON serialization with fallback
fn safe_serialize<T: serde::Serialize>(value: &T, default: serde_json::Value, field_name: &str) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or_else(|e| {
        log::warn!("Failed to serialize {}: {}", field_name, e);
        default
    })
}

pub async fn start_server(
    engine: Arc<Mutex<super::trading_engine::TradingEngine>>,
    risk_manager: Arc<Mutex<super::risk_management::RiskManager>>,
    solana_client: Arc<Mutex<super::solana_integration::SolanaClient>>,
    trading_enabled: Arc<Mutex<bool>>,
    rl_coordinator: Option<Arc<Mutex<super::reinforcement_learning::LearningCoordinator>>>, // ADD: RL coordinator for agent learning metrics
    circuit_breaker: Option<Arc<Mutex<super::error_handling::CircuitBreaker>>>, // ADD: Circuit breaker for API protection
    live_data_feed: Option<Arc<super::live_data_feed::LiveDataFeed>>, // ADD: Live data feed for management
    enhanced_marketplace: Option<Arc<super::enhanced_marketplace::EnhancedMarketplace>>, // ADD: Enhanced marketplace for advanced features
) {
    log::info!("🌐 Starting Warp server on :8080");
    
    // Create WebSocket broadcaster for real-time updates
    let ws_broadcaster = create_ws_broadcaster();
    
    // Create Jupiter client for DEX integration
    let jupiter_client = Arc::new(JupiterClient::new());
    
    // Create Switchboard Oracle client
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let use_real_oracle = std::env::var("SOLANA_RPC_URL").is_ok();
    // CIRCUIT BREAKER: Use circuit breaker if provided for API protection
    let switchboard_client = if let Some(ref cb) = circuit_breaker {
        Arc::new(SwitchboardClient::new_with_circuit_breaker(rpc_url.clone(), use_real_oracle, Some(cb.clone())))
    } else {
        Arc::new(SwitchboardClient::new(rpc_url.clone(), use_real_oracle))
    };
    
    // Create Mobula API client (GMGN-compatible) for token discovery
    // CIRCUIT BREAKER: Use circuit breaker if provided for API protection
    let dex_screener_client = if let Some(ref cb) = circuit_breaker {
        Arc::new(DexScreenerClient::new_with_circuit_breaker(Some(cb.clone())))
    } else {
        Arc::new(DexScreenerClient::new())
    };
    
    // Create PumpFun client
    // CIRCUIT BREAKER: Use circuit breaker if provided for API protection
    let pumpfun_client = if let Some(ref cb) = circuit_breaker {
        Arc::new(PumpFunClient::new_with_circuit_breaker(Some(cb.clone())))
    } else {
        Arc::new(PumpFunClient::new())
    };
    
    // Create Signal Marketplace
    let signal_marketplace = Arc::new(SignalMarketplace::new(rpc_url.clone()));
    
    // Create Quant Analyzer
    let quant_analyzer = Arc::new(crate::quant_analysis::QuantAnalyzer::new());
    
    // Create Jito BAM client for atomic bundle execution
    let use_mainnet = rpc_url.contains("mainnet");
    let jito_client = Arc::new(crate::jito_bam::JitoBamClient::new(use_mainnet));
    
    let cors = crate::security::cors_config();
    
    // Enhanced health check with system status
    let health = {
        let engine = engine.clone();
        let trading_enabled = trading_enabled.clone();
        
        warp::path("health")
            .and(warp::get())
            .and_then(move || {
                let engine = engine.clone();
                let trading_enabled = trading_enabled.clone();
                
                async move {
                    let mut response = HashMap::new();
                    response.insert("status".to_string(), "healthy".to_string());
                    response.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
                    
                    // Check trading engine status
                    let engine_lock = engine.lock().await;
                    response.insert("trades_count".to_string(), engine_lock.trade_history.len().to_string());
                    response.insert("positions_count".to_string(), engine_lock.portfolio.len().to_string());
                    response.insert("balance".to_string(), engine_lock.current_balance.to_string());
                    response.insert("initial_balance".to_string(), engine_lock.initial_balance.to_string());
                    drop(engine_lock);
                    
                    // Check trading status
                    let enabled = trading_enabled.lock().await;
                    response.insert("trading_enabled".to_string(), enabled.to_string());
                    drop(enabled);
                    
                    // Check dry-run mode
                    let dry_run_mode = std::env::var("DRY_RUN_MODE")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse::<bool>()
                        .unwrap_or(true);
                    response.insert("dry_run_mode".to_string(), dry_run_mode.to_string());
                    
                    // Check network
                    let rpc_url = std::env::var("SOLANA_RPC_URL")
                        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
                    let is_mainnet = rpc_url.contains("mainnet");
                    response.insert("network".to_string(), if is_mainnet { "mainnet".to_string() } else { "devnet".to_string() });
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "System is healthy")))
                }
            })
    };
    
    let portfolio_route = {
        let engine = engine.clone();
        let risk_manager = risk_manager.clone();
        
        warp::path("portfolio")
            .and(warp::get())
            .and_then(move || {
                let engine = engine.clone();
                let risk_manager = risk_manager.clone();
                
                async move {
                    let engine_lock = engine.lock().await;
                    let risk_lock = risk_manager.lock().await;
                    
                    let portfolio_data = engine_lock.get_portfolio_data();
                    let metrics = risk_lock.get_performance_metrics();
                    let roi = engine_lock.get_roi();
                    
                    // Build current prices map from market state
                    let mut current_prices = HashMap::new();
                    for (symbol, data) in &engine_lock.market_state {
                        if let Some(latest) = data.back() {
                            current_prices.insert(symbol.clone(), latest.price);
                        }
                    }
                    
                    // Calculate total portfolio value including positions
                    let total_value = engine_lock.get_total_value(&current_prices);
                    
                    // Check if we're in dry-run mode (paper trading)
                    let dry_run_mode = std::env::var("DRY_RUN_MODE")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse::<bool>()
                        .unwrap_or(false);
                    
                    // IMPROVEMENT: Safe JSON serialization with error handling
                    let mut response = HashMap::new();
                    response.insert("positions".to_string(), safe_serialize(&portfolio_data, serde_json::json!([]), "portfolio_data"));
                    response.insert("total_value".to_string(), safe_serialize(&total_value, serde_json::json!(0.0), "total_value"));
                    response.insert("cash".to_string(), safe_serialize(&engine_lock.current_balance, serde_json::json!(0.0), "current_balance"));
                    response.insert("daily_pnl".to_string(), safe_serialize(metrics.get("daily_pnl").unwrap_or(&0.0), serde_json::json!(0.0), "daily_pnl"));
                    response.insert("total_pnl".to_string(), safe_serialize(metrics.get("total_pnl").unwrap_or(&0.0), serde_json::json!(0.0), "total_pnl"));
                    response.insert("roi".to_string(), safe_serialize(&roi, serde_json::json!(0.0), "roi"));
                    response.insert("is_paper_trading".to_string(), safe_serialize(&dry_run_mode, serde_json::Value::Bool(false), "is_paper_trading"));
                    response.insert("initial_balance".to_string(), safe_serialize(&engine_lock.initial_balance, serde_json::json!(0.0), "initial_balance"));
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Portfolio data retrieved")))
                }
            })
    };
    
    let performance_route = {
        let risk_manager = risk_manager.clone();
        let engine = engine.clone();
        
        warp::path("performance")
            .and(warp::get())
            .and_then(move || {
                let risk_manager = risk_manager.clone();
                let engine = engine.clone();
                
                async move {
                    let risk_lock = risk_manager.lock().await;
                    let engine_lock = engine.lock().await;
                    let metrics = risk_lock.get_performance_metrics();
                    
                    // IMPROVEMENT: Safe JSON serialization with error handling
                    let mut response = HashMap::new();
                    response.insert("total_return".to_string(), safe_serialize(metrics.get("total_return").unwrap_or(&0.0), serde_json::json!(0.0), "total_return"));
                    response.insert("current_capital".to_string(), safe_serialize(metrics.get("current_capital").unwrap_or(&0.0), serde_json::json!(0.0), "current_capital"));
                    response.insert("max_drawdown".to_string(), safe_serialize(metrics.get("max_drawdown").unwrap_or(&0.0), serde_json::json!(0.0), "max_drawdown"));
                    response.insert("sharpe_ratio".to_string(), safe_serialize(metrics.get("sharpe_ratio").unwrap_or(&0.0), serde_json::json!(0.0), "sharpe_ratio"));
                    response.insert("win_rate".to_string(), safe_serialize(metrics.get("win_rate").unwrap_or(&0.0), serde_json::json!(0.0), "win_rate"));
                    response.insert("daily_pnl".to_string(), safe_serialize(metrics.get("daily_pnl").unwrap_or(&0.0), serde_json::json!(0.0), "daily_pnl"));
                    response.insert("total_pnl".to_string(), safe_serialize(metrics.get("total_pnl").unwrap_or(&0.0), serde_json::json!(0.0), "total_pnl"));
                    response.insert("trade_count".to_string(), safe_serialize(metrics.get("trade_count").unwrap_or(&0.0), serde_json::json!(0.0), "trade_count"));
                    
                    // Add ROI from engine
                    let roi = engine_lock.get_roi();
                    response.insert("roi_percent".to_string(), safe_serialize(&roi, serde_json::json!(0.0), "roi_percent"));
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Performance metrics retrieved")))
                }
            })
    };
    
    let market_data_route = {
        let engine = engine.clone();
        
        warp::path("market-data")
            .and(warp::get())
            .and_then(move || {
                let engine = engine.clone();
                
                async move {
                    let engine_lock = engine.lock().await;
                    
                    let mut market_data = Vec::new();
                    
                    for (symbol, data) in &engine_lock.market_state {
                        if let Some(latest) = data.back() {
                            // Calculate 24h change if we have historical data
                            let change_24h = if data.len() > 1 {
                                let first_price = data.front().map(|d| d.price).unwrap_or(latest.price);
                                if first_price > 0.0 {
                                    ((latest.price - first_price) / first_price) * 100.0
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            
                            let mut item = HashMap::new();
                            item.insert("symbol".to_string(), symbol.clone());
                            item.insert("price".to_string(), format!("{:.2}", latest.price));
                            item.insert("change".to_string(), format!("{:.2}", change_24h));
                            item.insert("volume".to_string(), format!("{:.0}", latest.volume));
                            market_data.push(item);
                        }
                    }
                    
                    // REMOVED: Hardcoded fallback prices - only return real data from TradingEngine
                    if market_data.is_empty() {
                        log::warn!("⚠️ No market data available in TradingEngine - ensure Live Data Feed is running");
                        log::info!("   Live Data Feed should be updating TradingEngine.market_state with real prices");
                        log::info!("   Check logs for 'Live feed update' messages to verify service is running");
                    }
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(market_data, "Market data retrieved")))
                }
            })
    };
    
    let signals_route = {
        let engine = engine.clone();
        
        warp::path("signals")
            .and(warp::get())
            .and_then(move || {
                let engine = engine.clone();
                
                async move {
                    let engine_lock = engine.lock().await;
                    let recent_signals: Vec<&super::trading_engine::TradingSignal> = 
                        engine_lock.trade_history.iter().rev().take(5).collect();
                    
                    let signals: Vec<HashMap<String, String>> = recent_signals.iter().map(|signal| {
                        let mut item = HashMap::new();
                        item.insert("symbol".to_string(), signal.symbol.clone());
                        item.insert("action".to_string(), format!("{:?}", signal.action));
                        item.insert("confidence".to_string(), format!("{:.2}", signal.confidence));
                        item.insert("price".to_string(), format!("{:.2}", signal.price));
                        item.insert("size".to_string(), format!("{:.2}", signal.size));
                        item
                    }).collect();
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(signals, "Trading signals retrieved")))
                }
            })
    };
    
    // WebSocket endpoint for real-time updates
    let ws_broadcaster_clone = ws_broadcaster.clone();
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let broadcaster = ws_broadcaster_clone.clone();
            ws.on_upgrade(move |socket| handle_websocket(socket, broadcaster))
        });
    
    // Jupiter quote endpoint
    // Jupiter quote endpoint
    let jupiter_quote_route = {
        let jupiter = jupiter_client.clone();
        
        warp::path!("jupiter" / "quote" / String / String / String)
            .and(warp::get())
            .and_then(move |input_mint: String, output_mint: String, amount: String| {
                let jupiter = jupiter.clone();
                
                async move {
                    match amount.parse::<u64>() {
                        Ok(amount_u64) => {
                            match jupiter.get_quote(&input_mint, &output_mint, amount_u64, 50).await {
                                Ok(quote) => {
                                    let mut response = HashMap::new();
                                    response.insert("input_mint".to_string(), quote.input_mint);
                                    response.insert("output_mint".to_string(), quote.output_mint);
                                    response.insert("in_amount".to_string(), quote.in_amount);
                                    response.insert("out_amount".to_string(), quote.out_amount);
                                    response.insert("price_impact".to_string(), quote.price_impact_pct.to_string());
                                    
                                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Quote retrieved")))
                                }
                                Err(e) => {
                                    log::error!("Jupiter quote error: {}", e);
                                    Ok(warp::reply::json(&ApiResponse::new(
                                        HashMap::<String, String>::new(),
                                        &format!("Failed to get quote: {}", e)
                                    )))
                                }
                            }
                        }
                        Err(_) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                "Invalid amount"
                            )))
                        }
                    }
                }
            })
    };
    
    // Jupiter best route endpoint (uses get_best_route method)
    let jupiter_best_route = {
        let jupiter = jupiter_client.clone();
        
        warp::path!("jupiter" / "best-route" / String / String / String)
            .and(warp::get())
            .and_then(move |input: String, output: String, amount_str: String| {
                let jupiter = jupiter.clone();
                
                async move {
                    match amount_str.parse::<u64>() {
                        Ok(amount) => {
                            match jupiter.get_best_route(&input, &output, amount).await {
                                Ok(quote) => {
                                    let mut response = HashMap::new();
                                    response.insert("input_mint".to_string(), quote.input_mint);
                                    response.insert("output_mint".to_string(), quote.output_mint);
                                    response.insert("in_amount".to_string(), quote.in_amount);
                                    response.insert("out_amount".to_string(), quote.out_amount);
                                    response.insert("price_impact".to_string(), quote.price_impact_pct.to_string());
                                    
                                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Best route retrieved")))
                                }
                                Err(e) => {
                                    log::error!("Jupiter best route error: {}", e);
                                    Ok(warp::reply::json(&ApiResponse::new(
                                        HashMap::<String, String>::new(),
                                        &format!("Failed to get best route: {}", e)
                                    )))
                                }
                            }
                        }
                        Err(_) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                "Invalid amount"
                            )))
                        }
                    }
                }
            })
    };
    
    // Jupiter pair supported endpoint (uses is_pair_supported method)
    let jupiter_pair_supported_route = {
        let jupiter = jupiter_client.clone();
        
        warp::path!("jupiter" / "pair" / "supported" / String / String)
            .and(warp::get())
            .and_then(move |input: String, output: String| {
                let jupiter = jupiter.clone();
                
                async move {
                    match jupiter.is_pair_supported(&input, &output).await {
                        Ok(is_supported) => {
                            let mut response = HashMap::new();
                            response.insert("input_mint".to_string(), input);
                            response.insert("output_mint".to_string(), output);
                            response.insert("supported".to_string(), is_supported.to_string());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Pair support checked")))
                        }
                        Err(e) => {
                            log::error!("Jupiter pair check error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to check pair: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // Combine all Jupiter routes
    let jupiter_route = jupiter_quote_route
        .or(jupiter_best_route)
        .or(jupiter_pair_supported_route)
        .boxed();
    
    // AI analysis endpoint (if DeepSeek is configured)
    let ai_route = {
        warp::path("ai")
            .and(warp::path("status"))
            .and(warp::get())
            .map(|| {
                let deepseek_enabled = std::env::var("DEEPSEEK_API_KEY").is_ok();
                let mut status = HashMap::new();
                status.insert("deepseek_enabled".to_string(), deepseek_enabled.to_string());
                status.insert("model".to_string(), "deepseek-chat".to_string());
                status.insert("features".to_string(), "AI-powered trading decisions, risk assessment".to_string());
                
                warp::reply::json(&ApiResponse::new(status, "AI status"))
            })
    };
    
    // Jito BAM endpoints for atomic bundle execution
    let jito_status_route = {
        let jito = jito_client.clone();
        warp::path("jito")
            .and(warp::path("status"))
            .and(warp::get())
            .map(move || {
                let mut status = HashMap::new();
                status.insert("enabled".to_string(), "true".to_string());
                status.insert("network".to_string(), if use_mainnet { "mainnet".to_string() } else { "devnet".to_string() });
                status.insert("block_engine".to_string(), jito.block_engine_url.clone());
                status.insert("features".to_string(), "Atomic bundle execution, MEV protection, priority tips".to_string());
                status.insert("tip_accounts".to_string(), jito.tip_accounts.len().to_string());
                
                warp::reply::json(&ApiResponse::new(status, "Jito BAM status"))
            })
    };
    
    #[derive(Debug, Deserialize)]
    struct BundleStatusRequest {
        bundle_id: String,
    }
    
    let jito_bundle_status_route = {
        let jito = jito_client.clone();
        warp::path("jito")
            .and(warp::path("bundle"))
            .and(warp::path("status"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |request: BundleStatusRequest| {
                let jito = jito.clone();
                async move {
                    match jito.get_bundle_status(&request.bundle_id).await {
                        Ok(status) => {
                            let mut response = HashMap::new();
                            response.insert("bundle_id".to_string(), request.bundle_id);
                            response.insert("status".to_string(), format!("{:?}", status));
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Bundle status retrieved"
                            )))
                        }
                        Err(e) => {
                            log::error!("Jito bundle status error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to get bundle status: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let jito_tip_account_route = {
        let jito = jito_client.clone();
        warp::path("jito")
            .and(warp::path("tip-account"))
            .and(warp::get())
            .map(move || {
                let tip_account = jito.get_random_tip_account()
                    .map(|pubkey| pubkey.to_string())
                    .unwrap_or_else(|| "No tip accounts available".to_string());
                
                let mut response = HashMap::new();
                response.insert("tip_account".to_string(), tip_account);
                response.insert("total_accounts".to_string(), jito.tip_accounts.len().to_string());
                
                warp::reply::json(&ApiResponse::new(response, "Random tip account"))
            })
    };
    
    // Switchboard Oracle endpoints
    let oracle_price_route = {
        let switchboard = switchboard_client.clone();
        
        warp::path!("oracle" / "price" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let switchboard = switchboard.clone();
                
                async move {
                    match switchboard.fetch_price(&symbol).await {
                        Ok(feed) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(feed, "Oracle price retrieved")))
                        }
                        Err(e) => {
                            log::error!("Oracle price error: {}", e);
                            let empty_feed = crate::switchboard_oracle::OracleFeed {
                                feed_address: "".to_string(),
                                symbol: symbol.clone(),
                                price: 0.0,
                                confidence: 0.0,
                                timestamp: 0,
                                slot: 0,
                                min_price: 0.0,
                                max_price: 0.0,
                                price_change_24h: None,
                            };
                            Ok(warp::reply::json(&ApiResponse::new(
                                empty_feed,
                                &format!("Failed to get price: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let oracle_feeds_route = {
        let switchboard = switchboard_client.clone();
        
        warp::path!("oracle" / "feeds")
            .and(warp::get())
            .and_then(move || {
                let switchboard = switchboard.clone();
                
                async move {
                    let symbols = vec!["SOL/USD".to_string(), "BTC/USD".to_string(), "ETH/USD".to_string()];
                    match switchboard.fetch_multiple_feeds(&symbols).await {
                        Ok(feeds) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(feeds, "Oracle feeds retrieved")))
                        }
                        Err(e) => {
                            log::error!("Oracle feeds error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::switchboard_oracle::OracleFeed>::new(),
                                &format!("Failed to get feeds: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // Mobula API endpoints (GMGN-compatible, replacing DEX Screener)
    let dex_search_route = {
        let dex_screener = dex_screener_client.clone();
        
        warp::path!("dex" / "search" / String)
            .and(warp::get())
            .and_then(move |query: String| {
                let dex_screener = dex_screener.clone();
                
                async move {
                    match dex_screener.search_tokens(&query).await {
                        Ok(pairs) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(pairs, "Mobula API pairs retrieved")))
                        }
                        Err(e) => {
                            log::error!("Mobula API search error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::dex_screener::TokenPair>::new(),
                                &format!("Failed to search: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let dex_opportunities_route = {
        let dex_screener = dex_screener_client.clone();
        
        warp::path!("dex" / "opportunities")
            .and(warp::get())
            .and_then(move || {
                let dex_screener = dex_screener.clone();
                
                async move {
                    match dex_screener.get_top_opportunities(10).await {
                        Ok(opportunities) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(opportunities, "Trading opportunities retrieved")))
                        }
                        Err(e) => {
                            log::error!("DEX opportunities error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::dex_screener::TradingOpportunity>::new(),
                                &format!("Failed to get opportunities: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // PumpFun endpoints
    let pumpfun_launches_route = {
        let pumpfun = pumpfun_client.clone();
        
        warp::path!("pumpfun" / "launches")
            .and(warp::get())
            .and_then(move || {
                let pumpfun = pumpfun.clone();
                
                async move {
                    match pumpfun.get_recent_launches(20).await {
                        Ok(launches) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(launches, "Recent launches retrieved")))
                        }
                        Err(e) => {
                            log::error!("PumpFun launches error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::pumpfun::TokenLaunch>::new(),
                                &format!("Failed to get launches: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let pumpfun_signals_route = {
        let pumpfun = pumpfun_client.clone();
        
        warp::path!("pumpfun" / "signals")
            .and(warp::get())
            .and_then(move || {
                let pumpfun = pumpfun.clone();
                
                async move {
                    match pumpfun.get_top_opportunities(10).await {
                        Ok(signals) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(signals, "Meme coin signals retrieved")))
                        }
                        Err(e) => {
                            log::error!("PumpFun signals error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::pumpfun::MemeTradeSignal>::new(),
                                &format!("Failed to get signals: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // Signal Platform endpoints (X402 protocol)
    let signal_marketplace_stats_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "stats")
            .and(warp::get())
            .and_then(move || {
                let marketplace = marketplace.clone();
                
                async move {
                    let stats = marketplace.get_marketplace_stats().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(stats, "Marketplace stats retrieved")))
                }
            })
    };
    
    let signal_active_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "active")
            .and(warp::get())
            .and_then(move || {
                let marketplace = marketplace.clone();
                
                async move {
                    let signals = marketplace.get_active_signals().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(signals, "Active signals retrieved")))
                }
            })
    };
    
    let signal_by_symbol_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "symbol" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let marketplace = marketplace.clone();
                
                async move {
                    let signals = marketplace.get_signals_by_symbol(&symbol).await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(signals, "Symbol signals retrieved")))
                }
            })
    };
    
    let signal_generate_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "generate" / String)
            .and(warp::post())
            .and_then(move |provider_id: String| {
                let marketplace = marketplace.clone();
                
                async move {
                    match marketplace.generate_signals(&provider_id).await {
                        Ok(signals) => {
                            let signal_count = signals.len();
                            let mut published_count = 0;
                            
                            // Publish signals one by one
                            for signal in signals.into_iter() {
                                match marketplace.publish_signal(signal).await {
                                    Ok(_) => published_count += 1,
                                    Err(e) => log::warn!("Failed to publish signal: {}", e),
                                }
                            }
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                format!("{}/{} signals published", published_count, signal_count),
                                "Signals generated and published"
                            )))
                        }
                        Err(_) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                String::new(),
                                "Failed to generate signals"
                            )))
                        }
                    }
                }
            })
    };
    
    let signal_provider_register_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "provider" / "register")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body: HashMap<String, String>| {
                let marketplace = marketplace.clone();
                
                async move {
                    let provider_id = body.get("id").cloned().unwrap_or_default();
                    let provider_name = body.get("name").cloned().unwrap_or_default();
                    
                    match marketplace.register_provider(provider_id.clone(), provider_name).await {
                        Ok(_) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                format!("Provider {} registered", provider_id),
                                "Provider registered successfully"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                String::new(),
                                &format!("Failed to register provider: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let signal_provider_stats_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "provider" / String)
            .and(warp::get())
            .and_then(move |provider_id: String| {
                let marketplace = marketplace.clone();
                
                async move {
                    match marketplace.get_provider_stats(&provider_id).await {
                        Some(provider) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                provider,
                                "Provider stats retrieved"
                            )))
                        }
                        None => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                String::new(),
                                "Provider not found"
                            )))
                        }
                    }
                }
            })
    };
    
    // Get all providers endpoint
    let signal_providers_list_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "providers")
            .and(warp::get())
            .and_then(move || {
                let marketplace = marketplace.clone();
                
                async move {
                    let providers = marketplace.get_all_providers().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        providers,
                        "All providers retrieved"
                    )))
                }
            })
    };
    
    let signal_purchase_route = {
        let marketplace = signal_marketplace.clone();
        
        warp::path!("signals" / "marketplace" / "purchase")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body: HashMap<String, String>| {
                let marketplace = marketplace.clone();
                
                async move {
                    let user_id = body.get("user_id").cloned().unwrap_or_default();
                    let signal_id = body.get("signal_id").cloned().unwrap_or_default();
                    let payment: f64 = body.get("payment")
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(0.0);
                    
                    match marketplace.purchase_signal(&user_id, &signal_id, payment).await {
                        Ok(confirmation) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                confirmation,
                                "Signal purchased successfully"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                String::new(),
                                &format!("Purchase failed: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // Wallet status route
    let wallet_status_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("wallet" / "status")
            .and(warp::get())
            .and_then(move || {
                let solana_client = solana_client.clone();
                
                async move {
                    let client_lock = solana_client.lock().await;
                    
                    let mut status = HashMap::new();
                    status.insert("connected", serde_json::to_value(client_lock.connected).unwrap());
                    status.insert("balance", serde_json::to_value(client_lock.wallet_balance).unwrap());
                    status.insert("transaction_count", serde_json::to_value(client_lock.transaction_count).unwrap());
                    status.insert("trading_budget", serde_json::to_value(client_lock.trading_budget).unwrap());
                    
                    if let Some(addr) = client_lock.get_wallet_address() {
                        status.insert("wallet_address", serde_json::to_value(addr).unwrap());
                    }
                    
                    if let Some(treasury) = client_lock.get_treasury_address() {
                        status.insert("treasury_address", serde_json::to_value(treasury).unwrap());
                    }
                    
                    if let Some(rpc) = &client_lock.rpc_url {
                        status.insert("rpc_url", serde_json::to_value(rpc).unwrap());
                    }
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        status,
                        "Wallet status retrieved"
                    )))
                }
            })
    };
    
    // Treasury status route
    let treasury_status_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("treasury" / "status")
            .and(warp::get())
            .and_then(move || {
                let solana_client = solana_client.clone();
                
                async move {
                    let client_lock = solana_client.lock().await;
                    
                    let mut status = HashMap::new();
                    
                    if let Some(treasury) = client_lock.get_treasury_address() {
                        status.insert("address", serde_json::to_value(treasury).unwrap());
                        status.insert("type", serde_json::to_value("PDA").unwrap());
                        status.insert("purpose", serde_json::to_value("Agent Trading Treasury").unwrap());
                    } else {
                        status.insert("status", serde_json::to_value("Not initialized").unwrap());
                    }
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        status,
                        "Treasury status retrieved"
                    )))
                }
            })
    };

    // Budget status route
    let budget_status_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("budget" / "status")
            .and(warp::get())
            .and_then(move || {
                let solana_client = solana_client.clone();
                
                async move {
                    let client_lock = solana_client.lock().await;
                    
                    let mut status = HashMap::new();
                    status.insert("trading_budget", serde_json::to_value(client_lock.get_trading_budget()).unwrap());
                    status.insert("wallet_balance", serde_json::to_value(client_lock.wallet_balance).unwrap());
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        status,
                        "Budget status retrieved"
                    )))
                }
            })
    };

    // Set budget route
    #[derive(Deserialize)]
    struct SetBudgetRequest {
        budget: f64,
    }

    let set_budget_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("budget" / "set")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: SetBudgetRequest| {
                let solana_client = solana_client.clone();
                
                async move {
                    let mut client_lock = solana_client.lock().await;
                    
                    match client_lock.set_trading_budget(req.budget) {
                        Ok(_) => {
                            let mut response = HashMap::new();
                            response.insert("trading_budget", serde_json::to_value(client_lock.get_trading_budget()).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Trading budget updated successfully"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to set budget: {}", e)
                            )))
                        }
                    }
                }
            })
    };

    // Deposit to PDA treasury route (real Solana transaction)
    #[derive(Deserialize)]
    struct PDADepositRequest {
        amount_sol: f64,
    }

    let pda_deposit_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("pda" / "deposit")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: PDADepositRequest| {
                let solana_client = solana_client.clone();
                
                async move {
                    let mut client_lock = solana_client.lock().await;
                    
                    match client_lock.deposit_to_pda(req.amount_sol).await {
                        Ok((signature, new_balance)) => {
                            let mut response: HashMap<String, serde_json::Value> = HashMap::new();
                            response.insert("transaction_signature".to_string(), serde_json::to_value(signature).unwrap());
                            response.insert("pda_balance".to_string(), serde_json::to_value(new_balance).unwrap());
                            response.insert("deposited_sol".to_string(), serde_json::to_value(req.amount_sol).unwrap());
                            response.insert("treasury_address".to_string(), 
                                serde_json::to_value(client_lock.get_treasury_address().unwrap_or_default()).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Funds deposited to PDA treasury successfully"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to deposit to PDA: {}", e)
                            )))
                        }
                    }
                }
            })
    };

    // Get PDA balance route
    let pda_balance_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("pda" / "balance")
            .and(warp::get())
            .and_then(move || {
                let solana_client = solana_client.clone();
                
                async move {
                    let client_lock = solana_client.lock().await;
                    
                    match client_lock.get_pda_balance().await {
                        Ok(balance) => {
                            let mut response = HashMap::new();
                            response.insert("pda_balance_sol".to_string(), serde_json::to_value(balance).unwrap());
                            response.insert("treasury_address".to_string(), 
                                serde_json::to_value(client_lock.get_treasury_address().unwrap_or_default()).unwrap());
                            response.insert("wallet_address".to_string(), 
                                serde_json::to_value(client_lock.get_wallet_address().unwrap_or_default()).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "PDA balance retrieved"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to get PDA balance: {}", e)
                            )))
                        }
                    }
                }
            })
    };

    // Get PDA info route (address and details)
    let pda_info_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("pda" / "info")
            .and(warp::get())
            .and_then(move || {
                let solana_client = solana_client.clone();
                
                async move {
                    let client_lock = solana_client.lock().await;
                    
                    let mut response = HashMap::new();
                    let treasury_addr = client_lock.get_treasury_address().unwrap_or_default();
                    response.insert("treasury_address".to_string(), serde_json::to_value(&treasury_addr).unwrap());
                    response.insert("pda_deposit_address".to_string(), serde_json::to_value(&treasury_addr).unwrap());
                    response.insert("wallet_address".to_string(), 
                        serde_json::to_value(client_lock.get_wallet_address().unwrap_or_default()).unwrap());
                    response.insert("wallet_balance_sol".to_string(), 
                        serde_json::to_value(client_lock.get_balance()).unwrap());
                    response.insert("trading_budget_sol".to_string(), 
                        serde_json::to_value(client_lock.get_trading_budget()).unwrap());
                    
                    // Try to get PDA balance
                    if let Ok(pda_balance) = client_lock.get_pda_balance().await {
                        response.insert("pda_balance_sol".to_string(), serde_json::to_value(pda_balance).unwrap());
                        response.insert("pda_status".to_string(), serde_json::to_value("initialized").unwrap());
                    } else {
                        response.insert("pda_balance_sol".to_string(), serde_json::to_value(0.0).unwrap());
                        response.insert("pda_status".to_string(), serde_json::to_value("not_created_yet").unwrap());
                        response.insert("note".to_string(), 
                            serde_json::to_value("PDA will be auto-created on first deposit. Send SOL to treasury_address above.").unwrap());
                    }
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        response,
                        "PDA info retrieved"
                    )))
                }
            })
    };

    // Deposit funds route (legacy simulated - kept for backward compatibility)
    #[derive(Deserialize)]
    struct DepositRequest {
        amount: f64,
    }

    let deposit_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("budget" / "deposit")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: DepositRequest| {
                let solana_client = solana_client.clone();
                
                async move {
                    let mut client_lock = solana_client.lock().await;
                    
                    match client_lock.deposit_funds(req.amount) {
                        Ok(new_budget) => {
                            let mut response: HashMap<String, serde_json::Value> = HashMap::new();
                            response.insert("trading_budget".to_string(), serde_json::to_value(new_budget).unwrap());
                            response.insert("deposited".to_string(), serde_json::to_value(req.amount).unwrap());
                            response.insert("note".to_string(), serde_json::to_value("Simulated deposit. Use /pda/deposit for real Solana transactions.").unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Funds deposited successfully (simulated)"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to deposit funds: {}", e)
                            )))
                        }
                    }
                }
            })
    };

    // Withdraw from PDA treasury route (real Solana transaction)
    #[derive(Deserialize)]
    struct PDAWithdrawRequest {
        amount_sol: f64,
    }

    let pda_withdraw_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("pda" / "withdraw")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: PDAWithdrawRequest| {
                let solana_client = solana_client.clone();
                
                async move {
                    let mut client_lock = solana_client.lock().await;
                    
                    match client_lock.withdraw_from_pda(req.amount_sol).await {
                        Ok((signature, new_balance)) => {
                            let mut response: HashMap<String, serde_json::Value> = HashMap::new();
                            response.insert("transaction_signature".to_string(), serde_json::to_value(signature).unwrap());
                            response.insert("pda_balance".to_string(), serde_json::to_value(new_balance).unwrap());
                            response.insert("withdrawn_sol".to_string(), serde_json::to_value(req.amount_sol).unwrap());
                            response.insert("treasury_address".to_string(), 
                                serde_json::to_value(client_lock.get_treasury_address().unwrap_or_default()).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Funds withdrawn from PDA treasury successfully"
                            )))
                        }
                        Err(e) => {
                            // Return error with helpful information
                            let mut error_response: HashMap<String, serde_json::Value> = HashMap::new();
                            error_response.insert("error".to_string(), serde_json::to_value(e.clone()).unwrap());
                            error_response.insert("note".to_string(), 
                                serde_json::to_value("PDA withdrawals require a Solana program using invoke_signed. See error details for more information.").unwrap());
                            error_response.insert("pda_address".to_string(),
                                serde_json::to_value(client_lock.get_treasury_address().unwrap_or_default()).unwrap());
                            
                            Ok(warp::reply::json(&ApiResponse::new(
                                error_response,
                                &format!("Withdrawal not yet implemented: {}", e)
                            )))
                        }
                    }
                }
            })
    };

    // Withdraw funds route (legacy simulated - kept for backward compatibility)
    #[derive(Deserialize)]
    struct WithdrawRequest {
        amount: f64,
    }

    let withdraw_route = {
        let solana_client = solana_client.clone();
        
        warp::path!("budget" / "withdraw")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: WithdrawRequest| {
                let solana_client = solana_client.clone();
                
                async move {
                    let mut client_lock = solana_client.lock().await;
                    
                    match client_lock.withdraw_funds(req.amount) {
                        Ok(new_budget) => {
                            let mut response: HashMap<String, serde_json::Value> = HashMap::new();
                            response.insert("trading_budget".to_string(), serde_json::to_value(new_budget).unwrap());
                            response.insert("withdrawn".to_string(), serde_json::to_value(req.amount).unwrap());
                            response.insert("note".to_string(), serde_json::to_value("Simulated withdrawal. Use /pda/withdraw for real Solana transactions.").unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Funds withdrawn successfully (simulated)"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Failed to withdraw funds: {}", e)
                            )))
                        }
                    }
                }
            })
    };

    // Quant analysis endpoint
    let quant_analysis_route = {
        let engine = engine.clone();
        let quant_analyzer = quant_analyzer.clone();
        
        warp::path!("quant" / "analyze" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let engine = engine.clone();
                let quant_analyzer = quant_analyzer.clone();
                
                async move {
                    let engine_lock = engine.lock().await;
                    
                    if let Some(market_data) = engine_lock.market_state.get(&symbol) {
                        let prices: Vec<f64> = market_data.iter().map(|d| d.price).collect();
                        let volumes: Vec<f64> = market_data.iter().map(|d| d.volume).collect();
                        
                        if let Some(indicators) = quant_analyzer.calculate_indicators(&prices, &volumes) {
                            let current_price = prices.last().copied().unwrap_or(0.0);
                            let signal_quality = quant_analyzer.analyze_signal_quality(&indicators, current_price);
                            
                            let mut response = HashMap::new();
                            response.insert("symbol", serde_json::to_value(&symbol).unwrap());
                            response.insert("current_price", serde_json::to_value(current_price).unwrap());
                            response.insert("indicators", serde_json::to_value(&indicators).unwrap());
                            response.insert("signal_quality", serde_json::to_value(&signal_quality).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Quantitative analysis completed"
                            )))
                        } else {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                "Insufficient data for analysis"
                            )))
                        }
                    } else {
                        Ok(warp::reply::json(&ApiResponse::new(
                            HashMap::<String, String>::new(),
                            &format!("Symbol {} not found", symbol)
                        )))
                    }
                }
            })
    };

    // Quant indicators overview endpoint
    let quant_overview_route = {
        let engine = engine.clone();
        let quant_analyzer = quant_analyzer.clone();
        
        warp::path!("quant" / "overview")
            .and(warp::get())
            .and_then(move || {
                let engine = engine.clone();
                let quant_analyzer = quant_analyzer.clone();
                
                async move {
                    let engine_lock = engine.lock().await;
                    let mut results = Vec::new();
                    
                    for (symbol, market_data) in engine_lock.market_state.iter() {
                        let prices: Vec<f64> = market_data.iter().map(|d| d.price).collect();
                        let volumes: Vec<f64> = market_data.iter().map(|d| d.volume).collect();
                        
                        if let Some(indicators) = quant_analyzer.calculate_indicators(&prices, &volumes) {
                            let current_price = prices.last().copied().unwrap_or(0.0);
                            let signal_quality = quant_analyzer.analyze_signal_quality(&indicators, current_price);
                            
                            let mut symbol_data = HashMap::new();
                            symbol_data.insert("symbol", serde_json::to_value(symbol).unwrap());
                            symbol_data.insert("current_price", serde_json::to_value(current_price).unwrap());
                            symbol_data.insert("recommendation", serde_json::to_value(&signal_quality.recommendation).unwrap());
                            symbol_data.insert("score", serde_json::to_value(signal_quality.score).unwrap());
                            symbol_data.insert("trend", serde_json::to_value(&signal_quality.trend).unwrap());
                            symbol_data.insert("confidence", serde_json::to_value(signal_quality.confidence).unwrap());
                            symbol_data.insert("rsi", serde_json::to_value(indicators.rsi_14).unwrap());
                            
                            results.push(symbol_data);
                        }
                    }
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        results,
                        "Quantitative analysis overview"
                    )))
                }
            })
    };
    
    // Break the route chain into smaller groups to avoid type complexity
    // Agent stats endpoint (uses autonomous_agent.get_stats method)
    let agent_stats_route = {
        let engine = engine.clone();
        
        warp::path!("agent" / "stats")
            .and(warp::get())
            .and_then(move || {
                let engine = engine.clone();
                
                async move {
                    // Create a temporary autonomous agent to get stats
                    // Note: In production, you might want to pass the agent instance
                    let engine_lock = engine.lock().await;
                    let mut stats = HashMap::new();
                    stats.insert("status".to_string(), "active".to_string());
                    stats.insert("total_trades".to_string(), engine_lock.trade_history.len().to_string());
                    stats.insert("current_balance".to_string(), format!("${:.2}", engine_lock.current_balance));
                    stats.insert("active_positions".to_string(), engine_lock.portfolio.len().to_string());
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(stats, "Agent stats retrieved")))
                }
            })
    };
    
    let core_routes = health
        .or(portfolio_route)
        .or(performance_route)
        .or(market_data_route)
        .or(signals_route)
        .or(ws_route)
        .or(jupiter_route)
        .or(ai_route)
        .or(agent_stats_route)
        .boxed();
    
    let jito_routes = jito_status_route
        .or(jito_bundle_status_route)
        .or(jito_tip_account_route)
        .boxed();
    
    // Oracle Aggregator endpoints
    let oracle_aggregator = Arc::new(crate::switchboard_oracle::OracleAggregator::new(switchboard_client.clone()));
    
    let oracle_aggregated_route = {
        let aggregator = oracle_aggregator.clone();
        
        warp::path!("oracle" / "aggregated" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let aggregator = aggregator.clone();
                
                async move {
                    match aggregator.get_aggregated_price(&symbol).await {
                        Ok(aggregated) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(aggregated, "Aggregated price retrieved")))
                        }
                        Err(e) => {
                            log::error!("Oracle aggregated price error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({}),
                                &format!("Failed to get aggregated price: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let oracle_confidence_route = {
        let aggregator = oracle_aggregator.clone();
        
        warp::path!("oracle" / "price-confidence" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let aggregator = aggregator.clone();
                
                async move {
                    match aggregator.get_price_with_confidence(&symbol).await {
                        Ok((price, confidence)) => {
                            let result = serde_json::json!({
                                "symbol": symbol,
                                "price": price,
                                "confidence": confidence,
                                "min_price": price - confidence,
                                "max_price": price + confidence,
                            });
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(result, "Price with confidence retrieved")))
                        }
                        Err(e) => {
                            log::error!("Oracle price-confidence error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({}),
                                &format!("Failed to get price with confidence: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let oracle_compare_route = {
        let aggregator = oracle_aggregator.clone();
        
        warp::path!("oracle" / "compare" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let aggregator = aggregator.clone();
                
                async move {
                    match aggregator.compare_prices(&symbol).await {
                        Ok(comparison) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(comparison, "Price comparison retrieved")))
                        }
                        Err(e) => {
                            log::error!("Oracle price comparison error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({}),
                                &format!("Failed to compare prices: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let oracle_health_route = {
        let aggregator = oracle_aggregator.clone();
        
        warp::path!("oracle" / "health")
            .and(warp::get())
            .and_then(move || {
                let aggregator = aggregator.clone();
                
                async move {
                    let health = aggregator.get_health().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(health, "Oracle health status retrieved")))
                }
            })
    };
    
    let oracle_batch_route = {
        let aggregator = oracle_aggregator.clone();
        
        warp::path!("oracle" / "batch")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |symbols: Vec<String>| {
                let aggregator = aggregator.clone();
                
                async move {
                    match aggregator.batch_aggregated_prices(&symbols).await {
                        Ok(prices) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(prices, "Batch prices retrieved")))
                        }
                        Err(e) => {
                            log::error!("Oracle batch prices error: {}", e);
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<serde_json::Value>::new(),
                                &format!("Failed to get batch prices: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // Live Price Feed Management endpoints
    let feed_management_routes = if let Some(ref feed) = live_data_feed {
        let feed_clone = feed.clone();
        
        // GET /feed/status - Get service status
        let feed_status_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "status")
                .and(warp::get())
                .and_then(move || {
                    let feed = feed.clone();
                    async move {
                        let status = feed.get_service_status().await;
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            status,
                            "Feed service status retrieved"
                        )))
                    }
                })
        };
        
        // GET /feed/symbols - Get all tracked symbols
        let feed_symbols_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "symbols")
                .and(warp::get())
                .and_then(move || {
                    let feed = feed.clone();
                    async move {
                        let symbols = feed.get_symbols().await;
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            symbols,
                            "Feed symbols retrieved"
                        )))
                    }
                })
        };
        
        // POST /feed/add - Add symbol to feed
        #[derive(Deserialize)]
        struct AddSymbolRequest {
            symbol: String,
            priority: Option<String>,
        }
        
        let feed_add_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "add")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |req: AddSymbolRequest| {
                    let feed = feed.clone();
                    async move {
                        let priority = req.priority.as_ref()
                            .and_then(|p| match p.as_str() {
                                "low" => Some(crate::live_data_feed::FeedPriority::Low),
                                "normal" => Some(crate::live_data_feed::FeedPriority::Normal),
                                "high" => Some(crate::live_data_feed::FeedPriority::High),
                                "critical" => Some(crate::live_data_feed::FeedPriority::Critical),
                                _ => None,
                            });
                        
                        match feed.add_symbol(req.symbol.clone(), priority).await {
                            Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"symbol": req.symbol, "status": "added"}),
                                "Symbol added to feed"
                            ))),
                            Err(e) => Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"error": e}),
                                "Failed to add symbol"
                            )))
                        }
                    }
                })
        };
        
        // POST /feed/remove - Remove symbol from feed
        #[derive(Deserialize)]
        struct RemoveSymbolRequest {
            symbol: String,
        }
        
        let feed_remove_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "remove")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |req: RemoveSymbolRequest| {
                    let feed = feed.clone();
                    async move {
                        match feed.remove_symbol(&req.symbol).await {
                            Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"symbol": req.symbol, "status": "removed"}),
                                "Symbol removed from feed"
                            ))),
                            Err(e) => Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"error": e}),
                                "Failed to remove symbol"
                            )))
                        }
                    }
                })
        };
        
        // GET /feed/config/{symbol} - Get feed configuration
        let feed_config_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "config" / String)
                .and(warp::get())
                .and_then(move |symbol: String| {
                    let feed = feed.clone();
                    async move {
                        if let Some(config) = feed.get_feed_config(&symbol).await {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                config,
                                "Feed configuration retrieved"
                            )))
                        } else {
                            Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({}),
                                &format!("Symbol {} not found in feed", symbol)
                            )))
                        }
                    }
                })
        };
        
        // POST /feed/config/{symbol} - Update feed configuration
        #[derive(Deserialize)]
        struct UpdateFeedConfigRequest {
            priority: Option<String>,
            enabled: Option<bool>,
        }
        
        let feed_update_config_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "config" / String)
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |symbol: String, req: UpdateFeedConfigRequest| {
                    let feed = feed.clone();
                    async move {
                        let priority = req.priority.as_ref()
                            .and_then(|p| match p.as_str() {
                                "low" => Some(crate::live_data_feed::FeedPriority::Low),
                                "normal" => Some(crate::live_data_feed::FeedPriority::Normal),
                                "high" => Some(crate::live_data_feed::FeedPriority::High),
                                "critical" => Some(crate::live_data_feed::FeedPriority::Critical),
                                _ => None,
                            });
                        
                        match feed.update_feed_config(&symbol, priority, req.enabled).await {
                            Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"symbol": symbol, "status": "updated"}),
                                "Feed configuration updated"
                            ))),
                            Err(e) => Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"error": e}),
                                "Failed to update feed configuration"
                            )))
                        }
                    }
                })
        };
        
        // GET /feed/stats - Get feed statistics
        let feed_stats_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "stats")
                .and(warp::get())
                .and_then(move || {
                    let feed = feed.clone();
                    async move {
                        match feed.get_feed_statistics(None).await {
                            Ok(stats) => Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                stats,
                                "Feed statistics retrieved"
                            ))),
                            Err(e) => Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::live_data_feed::FeedStatistics>::new(),
                                &format!("Failed to get feed statistics: {}", e)
                            )))
                        }
                    }
                })
        };
        
        // GET /feed/health - Get feed health status
        let feed_health_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "health")
                .and(warp::get())
                .and_then(move || {
                    let feed = feed.clone();
                    async move {
                        match feed.get_feed_health(None).await {
                            Ok(health) => Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                health,
                                "Feed health status retrieved"
                            ))),
                            Err(e) => Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<crate::live_data_feed::FeedHealth>::new(),
                                &format!("Failed to get feed health: {}", e)
                            )))
                        }
                    }
                })
        };
        
        // POST /feed/interval - Set update interval
        #[derive(Deserialize)]
        struct SetIntervalRequest {
            interval_secs: u64,
        }
        
        let feed_interval_route = {
            let feed = feed_clone.clone();
            warp::path!("feed" / "interval")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |req: SetIntervalRequest| {
                    let feed = feed.clone();
                    async move {
                        feed.set_update_interval(Duration::from_secs(req.interval_secs)).await;
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            serde_json::json!({"interval_secs": req.interval_secs}),
                            "Update interval set"
                        )))
                    }
                })
        };
        
        feed_status_route
            .or(feed_symbols_route)
            .or(feed_add_route)
            .or(feed_remove_route)
            .or(feed_config_route)
            .or(feed_update_config_route)
            .or(feed_stats_route)
            .or(feed_health_route)
            .or(feed_interval_route)
            .boxed()
    } else {
        // Return empty routes if live data feed not provided - match all routes from if branch
        let empty_feed_status = warp::path!("feed" / "status")
            .and(warp::get())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({"error": "Live data feed not available"}),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_symbols = warp::path!("feed" / "symbols")
            .and(warp::get())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<String>::new(),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_add = warp::path!("feed" / "add")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({"error": "Live data feed not available"}),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_remove = warp::path!("feed" / "remove")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({"error": "Live data feed not available"}),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_config = warp::path!("feed" / "config" / String)
            .and(warp::get())
            .and_then(move |_: String| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({}),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_update_config = warp::path!("feed" / "config" / String)
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: String, _: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({"error": "Live data feed not available"}),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_stats = warp::path!("feed" / "stats")
            .and(warp::get())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<crate::live_data_feed::FeedStatistics>::new(),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_health = warp::path!("feed" / "health")
            .and(warp::get())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<crate::live_data_feed::FeedHealth>::new(),
                    "Live data feed service not initialized"
                )))
            });
        
        let empty_feed_interval = warp::path!("feed" / "interval")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({"error": "Live data feed not available"}),
                    "Live data feed service not initialized"
                )))
            });
        
        empty_feed_status
            .or(empty_feed_symbols)
            .or(empty_feed_add)
            .or(empty_feed_remove)
            .or(empty_feed_config)
            .or(empty_feed_update_config)
            .or(empty_feed_stats)
            .or(empty_feed_health)
            .or(empty_feed_interval)
            .boxed()
    };
    
    let oracle_routes = oracle_price_route
        .or(oracle_feeds_route)
        .or(oracle_aggregated_route)
        .or(oracle_confidence_route)
        .or(oracle_compare_route)
        .or(oracle_health_route)
        .or(oracle_batch_route)
        .boxed();
    
    let dex_routes = dex_search_route
        .or(dex_opportunities_route)
        .boxed();
    
    let pumpfun_routes = pumpfun_launches_route
        .or(pumpfun_signals_route)
        .boxed();
    
    // ENHANCED: Enhanced marketplace routes
    let enhanced_marketplace_routes = if let Some(ref enhanced) = enhanced_marketplace {
        let enhanced_clone = enhanced.clone();
        
        // POST /marketplace/recommend - Get signal recommendations
        let recommend_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "recommend")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |body: serde_json::Value| {
                    let enhanced = enhanced.clone();
                    async move {
                        let user_id = body.get("user_id").and_then(|u| u.as_str());
                        let limit = body.get("limit")
                            .and_then(|l| l.as_u64())
                            .map(|l| l as usize)
                            .unwrap_or(10);
                        
                        let recommendations = enhanced.recommend_signals(user_id, limit).await;
                        let count = recommendations.len();
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            recommendations,
                            &format!("Generated {} recommendations", count)
                        )))
                    }
                })
        };
        
        // POST /marketplace/search/advanced - Advanced signal search
        let advanced_search_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "search" / "advanced")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |body: serde_json::Value| {
                    let enhanced = enhanced.clone();
                    async move {
                        let filters = super::enhanced_marketplace::AdvancedSearchFilters {
                            symbol: body.get("symbol").and_then(|s| s.as_str()).map(|s| s.to_string()),
                            min_confidence: body.get("min_confidence").and_then(|c| c.as_f64()),
                            max_confidence: body.get("max_confidence").and_then(|c| c.as_f64()),
                            min_price: body.get("min_price").and_then(|p| p.as_f64()),
                            max_price: body.get("max_price").and_then(|p| p.as_f64()),
                            min_rating: body.get("min_rating").and_then(|r| r.as_f64()),
                            provider_id: body.get("provider_id").and_then(|p| p.as_str()).map(|s| s.to_string()),
                            action: body.get("action").and_then(|a| {
                                match a.as_str() {
                                    Some("Buy") => Some(crate::signal_platform::SignalAction::Buy),
                                    Some("Sell") => Some(crate::signal_platform::SignalAction::Sell),
                                    Some("Hold") => Some(crate::signal_platform::SignalAction::Hold),
                                    _ => None,
                                }
                            }),
                            min_profit_target: body.get("min_profit_target").and_then(|t| t.as_f64()),
                            max_profit_target: body.get("max_profit_target").and_then(|t| t.as_f64()),
                            timeframe: body.get("timeframe").and_then(|t| t.as_str()).map(|s| s.to_string()),
                            min_reputation: body.get("min_reputation").and_then(|r| r.as_f64()),
                            risk_level: body.get("risk_level").and_then(|r| r.as_str()).map(|s| s.to_string()),
                        };
                        
                        let results = enhanced.advanced_search_signals(filters).await;
                        let count = results.len();
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            results,
                            &format!("Found {} signals matching criteria", count)
                        )))
                    }
                })
        };
        
        // POST /marketplace/providers/compare - Compare providers
        let compare_providers_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "providers" / "compare")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |body: serde_json::Value| {
                    let enhanced = enhanced.clone();
                    async move {
                        let provider_ids: Vec<String> = body.get("provider_ids")
                            .and_then(|ids| ids.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();
                        
                        if provider_ids.is_empty() {
                            return Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<super::enhanced_marketplace::ProviderComparison>::new(),
                                "No provider IDs provided"
                            )));
                        }
                        
                        let comparison = enhanced.compare_providers(provider_ids).await;
                        let count = comparison.len();
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            comparison,
                            &format!("Compared {} providers", count)
                        )))
                    }
                })
        };
        
        // GET /marketplace/portfolio/{user_id} - Get user portfolio
        let portfolio_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "portfolio" / String)
                .and(warp::get())
                .and_then(move |user_id: String| {
                    let enhanced = enhanced.clone();
                    async move {
                        let portfolio = enhanced.get_user_portfolio(&user_id).await;
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            portfolio,
                            "User portfolio retrieved"
                        )))
                    }
                })
        };
        
        // GET /marketplace/trends - Analyze market trends
        let trends_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "trends")
                .and(warp::get())
                .and_then(move || {
                    let enhanced = enhanced.clone();
                    async move {
                        let trends = enhanced.analyze_market_trends().await;
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            trends,
                            "Market trends analyzed"
                        )))
                    }
                })
        };
        
        // GET /marketplace/leaderboard - Get leaderboard
        let leaderboard_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "leaderboard")
                .and(warp::get())
                .and_then(move || {
                    let enhanced = enhanced.clone();
                    async move {
                        let leaderboard = enhanced.get_leaderboard().await;
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            leaderboard,
                            "Leaderboard retrieved"
                        )))
                    }
                })
        };
        
        // POST /marketplace/leaderboard/update - Update leaderboard
        let update_leaderboard_route = {
            let enhanced = enhanced_clone.clone();
            warp::path!("marketplace" / "leaderboard" / "update")
                .and(warp::post())
                .and_then(move || {
                    let enhanced = enhanced.clone();
                    async move {
                        match enhanced.update_leaderboard().await {
                            Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"status": "updated"}),
                                "Leaderboard updated successfully"
                            ))),
                            Err(e) => Ok(warp::reply::json(&ApiResponse::new(
                                serde_json::json!({"error": e}),
                                "Failed to update leaderboard"
                            )))
                        }
                    }
                })
        };
        
        recommend_route
            .or(advanced_search_route)
            .or(compare_providers_route)
            .or(portfolio_route)
            .or(trends_route)
            .or(leaderboard_route)
            .or(update_leaderboard_route)
            .boxed()
    } else {
        // Return empty routes if enhanced marketplace not provided
        // Create routes that match the same type signature as the if branch
        let empty_recommend = warp::path!("marketplace" / "recommend")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<super::enhanced_marketplace::SignalRecommendation>::new(),
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        let empty_search = warp::path!("marketplace" / "search" / "advanced")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<super::signal_platform::TradingSignalData>::new(),
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        let empty_compare = warp::path!("marketplace" / "providers" / "compare")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |_: serde_json::Value| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<super::enhanced_marketplace::ProviderComparison>::new(),
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        let empty_portfolio = warp::path!("marketplace" / "portfolio" / String)
            .and(warp::get())
            .and_then(move |_: String| async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    super::enhanced_marketplace::UserPortfolio {
                        user_id: "".to_string(),
                        active_positions: Vec::new(),
                        closed_positions: Vec::new(),
                        total_profit_loss: 0.0,
                        win_rate: 0.0,
                        total_invested: 0.0,
                    },
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        let empty_trends = warp::path!("marketplace" / "trends")
            .and(warp::get())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    super::enhanced_marketplace::MarketTrends {
                        bullish_symbols: Vec::new(),
                        bearish_symbols: Vec::new(),
                        high_confidence_signals: 0,
                        avg_market_confidence: 0.0,
                        top_providers: Vec::new(),
                        market_sentiment: "NEUTRAL".to_string(),
                    },
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        let empty_leaderboard = warp::path!("marketplace" / "leaderboard")
            .and(warp::get())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    Vec::<super::enhanced_marketplace::LeaderboardEntry>::new(),
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        let empty_update_leaderboard = warp::path!("marketplace" / "leaderboard" / "update")
            .and(warp::post())
            .and_then(move || async move {
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    serde_json::json!({"error": "Enhanced marketplace not available"}),
                    "Enhanced marketplace service not initialized"
                )))
            });
        
        empty_recommend
            .or(empty_search)
            .or(empty_compare)
            .or(empty_portfolio)
            .or(empty_trends)
            .or(empty_leaderboard)
            .or(empty_update_leaderboard)
            .boxed()
    };
    
    let marketplace_routes = signal_marketplace_stats_route
        .or(signal_active_route)
        .or(signal_by_symbol_route)
        .or(signal_generate_route)
        .or(signal_provider_register_route)
        .or(signal_provider_stats_route)
        .or(signal_providers_list_route)
        .or(signal_purchase_route)
        .or(enhanced_marketplace_routes)
        .boxed();
    
    let wallet_routes = wallet_status_route
        .or(treasury_status_route)
        .boxed();
    
    let budget_routes = budget_status_route
        .or(set_budget_route)
        .or(pda_deposit_route)
        .or(pda_balance_route)
        .or(pda_info_route)
        .or(pda_withdraw_route)
        .or(deposit_route)
        .or(withdraw_route)
        .boxed();
    
    // Trading state routes
    let trading_state_route = {
        let trading_enabled = trading_enabled.clone();
        warp::path("trading-state")
            .and(warp::get())
            .and_then(move || {
                let trading_enabled = trading_enabled.clone();
                async move {
                    let enabled = trading_enabled.lock().await;
                    let mut response = HashMap::new();
                    response.insert("enabled".to_string(), safe_serialize(&*enabled, serde_json::Value::Bool(false), "enabled"));
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Trading state retrieved")))
                }
            })
    };
    
    let trading_toggle_route = {
        let trading_enabled = trading_enabled.clone();
        warp::path("trading-toggle")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body: HashMap<String, serde_json::Value>| {
                let trading_enabled = trading_enabled.clone();
                async move {
                    let new_state = body.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    
                    let mut enabled = trading_enabled.lock().await;
                    *enabled = new_state;
                    
                    let status = if new_state { "enabled" } else { "disabled" };
                    log::info!("🔄 Trading {} by user request", status);
                    
                    let mut response = HashMap::new();
                    response.insert("enabled".to_string(), safe_serialize(&*enabled, serde_json::Value::Bool(false), "enabled"));
                    response.insert("message".to_string(), safe_serialize(&format!("Trading {}", status), serde_json::Value::String(String::new()), "message"));
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, &format!("Trading {}", status))))
                }
            })
    };
    
    // Safety status endpoint
    let safety_status_route = {
        let trading_enabled = trading_enabled.clone();
        let solana_client = solana_client.clone();
        let engine = engine.clone();
        let risk_manager = risk_manager.clone();
        
        warp::path!("safety" / "status")
            .and(warp::get())
            .and_then(move || {
                let trading_enabled = trading_enabled.clone();
                let solana_client = solana_client.clone();
                let engine = engine.clone();
                let risk_manager = risk_manager.clone();
                
                async move {
                    let enabled = trading_enabled.lock().await;
                    let client_lock = solana_client.lock().await;
                    let engine_lock = engine.lock().await;
                    
                    // Check dry-run mode from environment
                    let dry_run_mode = std::env::var("DRY_RUN_MODE")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse::<bool>()
                        .unwrap_or(true);
                    
                    let mut response = HashMap::new();
                    response.insert("trading_enabled".to_string(), safe_serialize(&*enabled, serde_json::Value::Bool(false), "trading_enabled"));
                    response.insert("dry_run_mode".to_string(), safe_serialize(&dry_run_mode, serde_json::Value::Bool(false), "dry_run_mode"));
                    
                    // Add risk manager status (drawdown, capital, etc.)
                    let risk_lock = risk_manager.lock().await;
                    let current_drawdown = risk_lock.calculate_drawdown();
                    let max_drawdown = risk_lock.max_drawdown;
                    response.insert("current_drawdown_pct".to_string(), safe_serialize(&(current_drawdown * 100.0), serde_json::json!(0.0), "current_drawdown_pct"));
                    response.insert("max_drawdown_pct".to_string(), safe_serialize(&(max_drawdown * 100.0), serde_json::json!(10.0), "max_drawdown_pct"));
                    response.insert("risk_manager_capital".to_string(), safe_serialize(&risk_lock.current_capital, serde_json::json!(0.0), "risk_manager_capital"));
                    response.insert("risk_manager_peak_capital".to_string(), safe_serialize(&risk_lock.peak_capital, serde_json::json!(0.0), "risk_manager_peak_capital"));
                    response.insert("drawdown_blocking_trades".to_string(), safe_serialize(&(current_drawdown >= max_drawdown), serde_json::Value::Bool(false), "drawdown_blocking_trades"));
                    drop(risk_lock);
                    response.insert("pda_balance".to_string(), safe_serialize(&client_lock.get_trading_budget(), serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap_or(serde_json::Number::from(0))), "pda_balance"));
                    response.insert("wallet_balance".to_string(), safe_serialize(&client_lock.wallet_balance, serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap_or(serde_json::Number::from(0))), "wallet_balance"));
                    response.insert("current_balance".to_string(), safe_serialize(&engine_lock.current_balance, serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap_or(serde_json::Number::from(0))), "current_balance"));
                    response.insert("total_trades".to_string(), safe_serialize(&engine_lock.trade_history.len(), serde_json::Value::Number(serde_json::Number::from(0)), "total_trades"));
                    
                    // Check if on mainnet
                    let is_mainnet = client_lock.rpc_url.as_ref()
                        .map(|url| url.contains("mainnet"))
                        .unwrap_or(false);
                    response.insert("network".to_string(), safe_serialize(&if is_mainnet { "mainnet" } else { "devnet" }, serde_json::Value::String(String::new()), "network"));
                    response.insert("is_mainnet".to_string(), safe_serialize(&is_mainnet, serde_json::Value::Bool(false), "is_mainnet"));
                    
                    // Safety warnings
                    let mut warnings = Vec::new();
                    if *enabled && is_mainnet && !dry_run_mode {
                        warnings.push("⚠️ Trading is ENABLED on MAINNET with DRY_RUN disabled - Real funds at risk!".to_string());
                    }
                    if *enabled && !dry_run_mode {
                        warnings.push("⚠️ DRY_RUN_MODE is disabled - Real trades will be executed!".to_string());
                    }
                    if *enabled && is_mainnet {
                        warnings.push("⚠️ Connected to MAINNET - Real funds will be used!".to_string());
                    }
                    if client_lock.get_trading_budget() == 0.0 {
                        warnings.push("ℹ️ PDA balance is 0 - No funds available for trading".to_string());
                    }
                    if dry_run_mode {
                        warnings.push("✅ DRY_RUN mode is active - All trades will be simulated".to_string());
                    }
                    if !*enabled {
                        warnings.push("ℹ️ Trading is disabled - Enable via POST /trading-toggle".to_string());
                    }
                    // Add drawdown warning if blocking trades
                    if current_drawdown >= max_drawdown {
                        warnings.push(format!("⚠️ Drawdown {:.2}% exceeds max {:.2}% - Trading blocked", 
                                            current_drawdown * 100.0, max_drawdown * 100.0));
                    }
                    response.insert("warnings".to_string(), safe_serialize(&warnings, serde_json::Value::Array(vec![]), "warnings"));
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Safety status retrieved")))
                }
            })
    };
    
    // RL Agents endpoints - Learning metrics from past trades only (no subscription needed)
    let rl_agents_route = {
        let rl_coordinator = rl_coordinator.clone();
        
        warp::path!("rl" / "agents")
            .and(warp::get())
            .and_then(move || {
                let rl_coordinator = rl_coordinator.clone();
                
                async move {
                    if let Some(ref coordinator) = rl_coordinator {
                        let coordinator_lock = coordinator.lock().await;
                        let perf_map = coordinator_lock.get_all_performance().await;
                        
                        let mut agents = Vec::new();
                        for (agent_id, perf) in perf_map.iter() {
                            let mut agent_data = HashMap::new();
                            agent_data.insert("agent_id".to_string(), serde_json::to_value(agent_id).unwrap());
                            agent_data.insert("total_trades".to_string(), serde_json::to_value(perf.total_trades).unwrap());
                            agent_data.insert("successful_trades".to_string(), serde_json::to_value(perf.successful_trades).unwrap());
                            agent_data.insert("failed_trades".to_string(), serde_json::to_value(perf.failed_trades).unwrap());
                            agent_data.insert("win_rate".to_string(), serde_json::to_value(perf.win_rate).unwrap());
                            agent_data.insert("avg_reward".to_string(), serde_json::to_value(perf.avg_reward).unwrap());
                            agent_data.insert("sharpe_ratio".to_string(), serde_json::to_value(perf.sharpe_ratio).unwrap());
                            agent_data.insert("max_drawdown".to_string(), serde_json::to_value(perf.max_drawdown).unwrap());
                            agent_data.insert("learning_rate".to_string(), serde_json::to_value(perf.learning_rate).unwrap());
                            agent_data.insert("total_profit".to_string(), serde_json::to_value(perf.total_profit).unwrap());
                            agent_data.insert("total_loss".to_string(), serde_json::to_value(perf.total_loss).unwrap());
                            agents.push(agent_data);
                        }
                        
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(agents, "RL agents learning metrics retrieved")))
                    } else {
                        Ok(warp::reply::json(&ApiResponse::new(Vec::<HashMap<String, serde_json::Value>>::new(), "RL coordinator not available")))
                    }
                }
            })
    };
    
    let trading_routes = trading_state_route
        .or(trading_toggle_route)
        .boxed();
    
    let rl_routes = rl_agents_route.boxed();
    
    // Circuit Breaker status endpoint
    let circuit_breaker_status_route = {
        let circuit_breaker = circuit_breaker.clone();
        
        warp::path!("circuit" / "breaker" / "status")
            .and(warp::get())
            .and_then(move || {
                let circuit_breaker = circuit_breaker.clone();
                async move {
                    if let Some(ref cb) = circuit_breaker {
                        let cb_lock = cb.lock().await;
                        let state = cb_lock.get_state().await;
                        
                        // Get stats from circuit breaker (fields are private, use available methods)
                        // For now, just return basic state info
                        let failures = 0u32; // CircuitBreaker doesn't expose these directly
                        let successes = 0u32;
                        let total_calls = failures + successes;
                        let success_rate = if total_calls > 0 {
                            (successes as f64 / total_calls as f64) * 100.0
                        } else {
                            0.0
                        };
                        
                        let mut response = HashMap::new();
                        response.insert("state".to_string(), serde_json::to_value(format!("{:?}", state)).unwrap());
                        response.insert("total_calls".to_string(), serde_json::to_value(total_calls).unwrap());
                        response.insert("successful_calls".to_string(), serde_json::to_value(successes).unwrap());
                        response.insert("failed_calls".to_string(), serde_json::to_value(failures).unwrap());
                        response.insert("success_rate".to_string(), serde_json::to_value(success_rate).unwrap());
                        response.insert("consecutive_failures".to_string(), serde_json::to_value(failures).unwrap());
                        
                        Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                            response,
                            "Circuit breaker status retrieved"
                        )))
                    } else {
                        Ok(warp::reply::json(&ApiResponse::new(
                            serde_json::json!({"error": "Circuit breaker not available"}),
                            "Circuit breaker service not initialized"
                        )))
                    }
                }
            })
    };
    
    let circuit_breaker_routes = circuit_breaker_status_route.boxed();
    
    let quant_routes = quant_analysis_route
        .or(quant_overview_route)
        .boxed();
    
    let routes = core_routes
        .or(jito_routes)
        .or(oracle_routes)
        .or(feed_management_routes)
        .or(dex_routes)
        .or(pumpfun_routes)
        .or(marketplace_routes)
        .or(wallet_routes)
        .or(budget_routes)
        .or(quant_routes)
        .or(trading_routes)
        .or(rl_routes)
        .or(circuit_breaker_routes)
        .or(safety_status_route)
        .with(cors)
        .with(warp::log("api"));
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
