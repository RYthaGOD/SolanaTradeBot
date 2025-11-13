use warp::Filter;
use std::collections::HashMap;
use serde::Serialize;
use std::sync::Arc;
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

pub async fn start_server(
    engine: Arc<Mutex<super::trading_engine::TradingEngine>>,
    risk_manager: Arc<Mutex<super::risk_management::RiskManager>>,
) {
    log::info!("🌐 Starting Warp server on :8080");
    
    // Create WebSocket broadcaster for real-time updates
    let ws_broadcaster = create_ws_broadcaster();
    
    // Create Jupiter client for DEX integration
    let jupiter_client = Arc::new(JupiterClient::new());
    
    // Create Switchboard Oracle client
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let switchboard_client = Arc::new(SwitchboardClient::new(rpc_url.clone()));
    
    // Create DEX Screener client
    let dex_screener_client = Arc::new(DexScreenerClient::new());
    
    // Create PumpFun client
    let pumpfun_client = Arc::new(PumpFunClient::new());
    
    // Create Signal Marketplace
    let signal_marketplace = Arc::new(SignalMarketplace::new(rpc_url));
    
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
                    
                    let mut response = HashMap::new();
                    response.insert("positions".to_string(), serde_json::to_value(portfolio_data).unwrap());
                    response.insert("total_value".to_string(), serde_json::to_value(metrics.get("current_capital").unwrap_or(&0.0)).unwrap());
                    response.insert("cash".to_string(), serde_json::to_value(engine_lock.current_balance).unwrap());
                    response.insert("daily_pnl".to_string(), serde_json::to_value(metrics.get("daily_pnl").unwrap_or(&0.0)).unwrap());
                    response.insert("total_pnl".to_string(), serde_json::to_value(metrics.get("total_pnl").unwrap_or(&0.0)).unwrap());
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(response, "Portfolio data retrieved")))
                }
            })
    };
    
    let performance_route = {
        let risk_manager = risk_manager.clone();
        
        warp::path("performance")
            .and(warp::get())
            .and_then(move || {
                let risk_manager = risk_manager.clone();
                
                async move {
                    let risk_lock = risk_manager.lock().await;
                    let metrics = risk_lock.get_performance_metrics();
                    
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
    
    let routes = health
        .or(portfolio_route)
        .or(performance_route)
        .or(market_data_route)
        .or(signals_route)
        .or(ws_route)
        .or(jupiter_route)
        .or(ai_route)
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
        .with(cors)
        .with(warp::log("api"));
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
