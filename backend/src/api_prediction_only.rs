use warp::Filter;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::prediction_markets::{PredictionMarketClient, SignalAction};

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

pub async fn start_prediction_server() {
    log::info!("🔮 Starting Prediction Markets Trading Server on :8080");
    
    // Create Prediction Market client
    let use_real_data = std::env::var("USE_REAL_PREDICTION_DATA").is_ok();
    let prediction_client = Arc::new(Mutex::new(PredictionMarketClient::new(use_real_data)));
    
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"]);
    
    // Health check
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&ApiResponse::new(
                "OK",
                "Prediction Markets Trading Server is healthy"
            ))
        });
    
    // Get all active prediction markets
    let markets_route = {
        let client = prediction_client.clone();
        
        warp::path!("markets")
            .and(warp::get())
            .and_then(move || {
                let client = client.clone();
                async move {
                    let client_lock = client.lock().await;
                    let markets = client_lock.get_active_markets().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        markets,
                        "Active prediction markets retrieved"
                    )))
                }
            })
    };
    
    // Get specific market details
    let market_detail_route = {
        let client = prediction_client.clone();
        
        warp::path!("markets" / String)
            .and(warp::get())
            .and_then(move |market_id: String| {
                let client = client.clone();
                async move {
                    let client_lock = client.lock().await;
                    match client_lock.get_market(&market_id).await {
                        Some(market) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                market,
                                "Market details retrieved"
                            )))
                        }
                        None => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                String::new(),
                                "Market not found"
                            )))
                        }
                    }
                }
            })
    };
    
    // Get market statistics
    let stats_route = {
        let client = prediction_client.clone();
        
        warp::path!("stats")
            .and(warp::get())
            .and_then(move || {
                let client = client.clone();
                async move {
                    let client_lock = client.lock().await;
                    let stats = client_lock.get_market_stats().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        stats,
                        "Market statistics retrieved"
                    )))
                }
            })
    };
    
    // Get trading signals for a market
    let signals_route = {
        let client = prediction_client.clone();
        
        warp::path!("signals" / String)
            .and(warp::get())
            .and_then(move |market_id: String| {
                let client = client.clone();
                async move {
                    let client_lock = client.lock().await;
                    match client_lock.analyze_market(&market_id).await {
                        Ok(signals) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                signals,
                                "Trading signals generated"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                Vec::<String>::new(),
                                &format!("Failed to analyze market: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    // Get all signals for all markets
    let all_signals_route = {
        let client = prediction_client.clone();
        
        warp::path!("signals")
            .and(warp::get())
            .and_then(move || {
                let client = client.clone();
                async move {
                    let client_lock = client.lock().await;
                    let markets = client_lock.get_active_markets().await;
                    
                    let mut all_signals = Vec::new();
                    for market in markets {
                        if let Ok(signals) = client_lock.analyze_market(&market.market_id).await {
                            all_signals.extend(signals);
                        }
                    }
                    
                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                        all_signals,
                        "All trading signals generated"
                    )))
                }
            })
    };
    
    // Execute a trade
    let trade_route = {
        let client = prediction_client.clone();
        
        warp::path!("trade")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body: HashMap<String, String>| {
                let client = client.clone();
                async move {
                    let market_id = body.get("market_id").cloned().unwrap_or_default();
                    let outcome_id = body.get("outcome_id").cloned().unwrap_or_default();
                    let action_str = body.get("action").cloned().unwrap_or_default();
                    let amount: f64 = body.get("amount")
                        .and_then(|a| a.parse().ok())
                        .unwrap_or(100.0);
                    
                    let action = match action_str.as_str() {
                        "buy_yes" => SignalAction::BuyYes,
                        "sell_yes" => SignalAction::SellYes,
                        "buy_no" => SignalAction::BuyNo,
                        "sell_no" => SignalAction::SellNo,
                        _ => SignalAction::Hold,
                    };
                    
                    let mut client_lock = client.lock().await;
                    match client_lock.execute_trade(&market_id, &outcome_id, &action, amount).await {
                        Ok(trade_id) => {
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                trade_id,
                                "Trade executed successfully"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                String::new(),
                                &format!("Trade failed: {}", e)
                            )))
                        }
                    }
                }
            })
    };
    
    let routes = health
        .or(markets_route)
        .or(market_detail_route)
        .or(stats_route)
        .or(signals_route)
        .or(all_signals_route)
        .or(trade_route)
        .with(cors)
        .with(warp::log("api"));
    
    log::info!("✅ Prediction Markets API ready:");
    log::info!("   GET  /health          - Health check");
    log::info!("   GET  /markets         - List all markets");
    log::info!("   GET  /markets/:id     - Get market details");
    log::info!("   GET  /stats           - Market statistics");
    log::info!("   GET  /signals         - All trading signals");
    log::info!("   GET  /signals/:id     - Signals for specific market");
    log::info!("   POST /trade           - Execute trade");
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
