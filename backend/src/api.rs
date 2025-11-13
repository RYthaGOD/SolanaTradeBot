use warp::Filter;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::jupiter_integration::JupiterClient;
use crate::websocket::{create_ws_broadcaster, handle_websocket};
use crate::switchboard_oracle::SwitchboardClient;
use crate::dex_screener::DexScreenerClient;
use crate::pumpfun::PumpFunClient;
use crate::signal_platform::SignalMarketplace;
use crate::ml_models::TradingPredictor;
use crate::security::validate_wallet_address;
use crate::fee_optimization::FeeOptimizer;

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

pub async fn start_server(
    engine: Arc<Mutex<super::trading_engine::TradingEngine>>,
    risk_manager: Arc<Mutex<super::risk_management::RiskManager>>,
    solana_client: Arc<Mutex<super::solana_integration::SolanaClient>>,
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
    let switchboard_client = Arc::new(SwitchboardClient::new(rpc_url.clone(), use_real_oracle));
    
    // Create DEX Screener client
    let dex_screener_client = Arc::new(DexScreenerClient::new());
    
    // Create PumpFun client
    let pumpfun_client = Arc::new(PumpFunClient::new());
    
    // Create Signal Marketplace
    let signal_marketplace = Arc::new(SignalMarketplace::new(rpc_url.clone()));
    
    // Create Quant Analyzer
    let quant_analyzer = Arc::new(crate::quant_analysis::QuantAnalyzer::new());
    
    // Create Jito BAM client for atomic bundle execution
    let use_mainnet = rpc_url.contains("mainnet");
    let jito_client = Arc::new(crate::jito_bam::JitoBamClient::new(use_mainnet));
    
    let cors = crate::security::cors_config();
    
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&ApiResponse::new("OK", "Server is healthy"))
        });
    
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
                    
                    let mut response = HashMap::new();
                    response.insert("positions".to_string(), serde_json::to_value(portfolio_data).unwrap());
                    response.insert("total_value".to_string(), serde_json::to_value(total_value).unwrap());
                    response.insert("cash".to_string(), serde_json::to_value(engine_lock.current_balance).unwrap());
                    response.insert("daily_pnl".to_string(), serde_json::to_value(metrics.get("daily_pnl").unwrap_or(&0.0)).unwrap());
                    response.insert("total_pnl".to_string(), serde_json::to_value(metrics.get("total_pnl").unwrap_or(&0.0)).unwrap());
                    response.insert("roi".to_string(), serde_json::to_value(roi).unwrap());
                    
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
                    let mut metrics = risk_lock.get_performance_metrics();
                    
                    // Add ROI using initial_balance
                    let roi = engine_lock.get_roi();
                    metrics.insert("roi_percent".to_string(), roi);
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(metrics, "Performance metrics retrieved")))
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
                            let mut item = HashMap::new();
                            item.insert("symbol".to_string(), symbol.clone());
                            item.insert("price".to_string(), format!("{:.2}", latest.price));
                            item.insert("change".to_string(), "0.0".to_string());
                            item.insert("volume".to_string(), format!("{:.0}", latest.volume));
                            market_data.push(item);
                        }
                    }
                    
                    if market_data.is_empty() {
                        let mut sol_item = HashMap::new();
                        sol_item.insert("symbol".to_string(), "SOL/USDC".to_string());
                        sol_item.insert("price".to_string(), "105.50".to_string());
                        sol_item.insert("change".to_string(), "2.5".to_string());
                        sol_item.insert("volume".to_string(), "2500000".to_string());
                        
                        let mut btc_item = HashMap::new();
                        btc_item.insert("symbol".to_string(), "BTC/USDC".to_string());
                        btc_item.insert("price".to_string(), "51200.00".to_string());
                        btc_item.insert("change".to_string(), "1.2".to_string());
                        btc_item.insert("volume".to_string(), "150000000".to_string());
                        
                        let mut eth_item = HashMap::new();
                        eth_item.insert("symbol".to_string(), "ETH/USDC".to_string());
                        eth_item.insert("price".to_string(), "3050.00".to_string());
                        eth_item.insert("change".to_string(), "0.8".to_string());
                        eth_item.insert("volume".to_string(), "75000000".to_string());
                        
                        market_data = vec![sol_item, btc_item, eth_item];
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
    let jupiter_route = {
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
    
    // DEX Screener endpoints
    let dex_search_route = {
        let dex_screener = dex_screener_client.clone();
        
        warp::path!("dex" / "search" / String)
            .and(warp::get())
            .and_then(move |query: String| {
                let dex_screener = dex_screener.clone();
                
                async move {
                    match dex_screener.search_tokens(&query).await {
                        Ok(pairs) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(pairs, "DEX pairs retrieved")))
                        }
                        Err(e) => {
                            log::error!("DEX search error: {}", e);
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

    // Deposit funds route
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
                            let mut response = HashMap::new();
                            response.insert("trading_budget", serde_json::to_value(new_budget).unwrap());
                            response.insert("deposited", serde_json::to_value(req.amount).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Funds deposited successfully"
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

    // Withdraw funds route
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
                            let mut response = HashMap::new();
                            response.insert("trading_budget", serde_json::to_value(new_budget).unwrap());
                            response.insert("withdrawn", serde_json::to_value(req.amount).unwrap());
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Funds withdrawn successfully"
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
    
    // ML Prediction endpoint
    let ml_predict_route = {
        let engine = engine.clone();
        
        warp::path!("ml" / "predict" / String)
            .and(warp::get())
            .and_then(move |symbol: String| {
                let engine = engine.clone();
                
                async move {
                    let engine_lock = engine.lock().await;
                    let predictor = TradingPredictor::new();
                    
                    if let Some(market_data_queue) = engine_lock.market_state.get(&symbol) {
                        if let Some(latest_data) = market_data_queue.back() {
                            let features = predictor.generate_features(latest_data);
                            let (confidence, price_change) = predictor.predict(&features).await;
                            
                            let mut response = HashMap::new();
                            response.insert("symbol".to_string(), symbol);
                            response.insert("confidence".to_string(), format!("{:.2}", confidence));
                            response.insert("predicted_change".to_string(), format!("{:.4}", price_change));
                            response.insert("current_price".to_string(), format!("{:.2}", latest_data.price));
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "ML prediction generated"
                            )))
                        } else {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                "No market data available"
                            )))
                        }
                    } else {
                        Ok(warp::reply::json(&ApiResponse::new(
                            HashMap::<String, String>::new(),
                            "Symbol not found"
                        )))
                    }
                }
            })
    };
    
    // Fee optimization endpoint
    let fee_estimate_route = {
        warp::path!("fees" / "estimate" / String)
            .and(warp::get())
            .and_then(move |priority: String| {
                async move {
                    let optimizer = FeeOptimizer::new(5000); // base fee
                    let priority_level = match priority.as_str() {
                        "low" => crate::fee_optimization::FeePriority::Low,
                        "normal" => crate::fee_optimization::FeePriority::Normal,
                        "high" => crate::fee_optimization::FeePriority::High,
                        _ => crate::fee_optimization::FeePriority::Normal,
                    };
                    
                    let estimate = optimizer.estimate_fee(priority_level);
                    
                    let mut response = HashMap::new();
                    response.insert("priority".to_string(), priority);
                    response.insert("min_fee".to_string(), estimate.min_fee.to_string());
                    response.insert("recommended_fee".to_string(), estimate.recommended_fee.to_string());
                    response.insert("priority_fee".to_string(), estimate.priority_fee.to_string());
                    response.insert("max_fee".to_string(), estimate.max_fee.to_string());
                    response.insert("confidence".to_string(), format!("{:.2}", estimate.confidence));
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        response,
                        "Fee estimate calculated"
                    )))
                }
            })
    };
    
    // Wallet validation endpoint
    let validate_wallet_route = {
        warp::path!("validate" / "wallet" / String)
            .and(warp::get())
            .and_then(move |address: String| {
                async move {
                    let is_valid = validate_wallet_address(&address);
                    
                    let mut response = HashMap::new();
                    response.insert("address".to_string(), address);
                    response.insert("valid".to_string(), is_valid.to_string());
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        response,
                        if is_valid { "Valid wallet address" } else { "Invalid wallet address" }
                    )))
                }
            })
    };
    
    let routes = health
        .or(portfolio_route)
        .or(performance_route)
        .or(market_data_route)
        .or(signals_route)
        .or(ws_route)
        .or(jupiter_route)
        .or(ai_route)
        .or(jito_status_route)
        .or(jito_bundle_status_route)
        .or(jito_tip_account_route)
        .or(oracle_price_route)
        .or(oracle_feeds_route)
        .or(dex_search_route)
        .or(dex_opportunities_route)
        .or(pumpfun_launches_route)
        .or(pumpfun_signals_route)
        .or(signal_marketplace_stats_route)
        .or(signal_active_route)
        .or(signal_by_symbol_route)
        .or(signal_generate_route)
        .or(signal_provider_register_route)
        .or(signal_provider_stats_route)
        .or(signal_purchase_route)
        .or(wallet_status_route)
        .or(treasury_status_route)
        .or(budget_status_route)
        .or(set_budget_route)
        .or(deposit_route)
        .or(withdraw_route)
        .or(quant_analysis_route)
        .or(quant_overview_route)
        .or(ml_predict_route)
        .or(fee_estimate_route)
        .or(validate_wallet_route)
        .with(cors)
        .with(warp::log("api"));
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
