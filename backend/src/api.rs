use warp::Filter;
use std::collections::HashMap;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::jupiter_integration::JupiterClient;
use crate::websocket::{create_ws_broadcaster, handle_websocket};

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
    
    let routes = health
        .or(portfolio_route)
        .or(performance_route)
        .or(market_data_route)
        .or(signals_route)
        .or(ws_route)
        .or(jupiter_route)
        .or(ai_route)
        .with(cors)
        .with(warp::log("api"));
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
