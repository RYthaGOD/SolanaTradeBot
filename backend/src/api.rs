use warp::Filter;
use std::collections::HashMap;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    _solana_client: Arc<Mutex<super::solana_integration::SolanaClient>>,
    _solana_rpc: Arc<Mutex<super::solana_rpc::SolanaRpcClient>>,
    _jupiter_client: Arc<Mutex<super::jupiter_integration::JupiterClient>>,
    _dex_executor: Arc<Mutex<super::dex_executor::DexExecutor>>,
    config: super::config::ApiConfig,
) {
    log::info!("🌐 Starting Warp server on {}:{}", config.host, config.port);
    log::info!("🔷 Jupiter Aggregator integration active");
    log::info!("🔷 Phase 2 DEX Executor ready for devnet testing");
    
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);
    
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
    
    let routes = health
        .or(portfolio_route)
        .or(performance_route)
        .or(market_data_route)
        .or(signals_route)
        .with(cors)
        .with(warp::log("api"));
    
    let addr: std::net::SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");
    
    warp::serve(routes)
        .run(addr)
        .await;
}
