mod trading_engine;
mod solana_integration;
mod risk_management;
mod ml_models;
mod api;

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("🚀 Starting AgentBurn Solana Trading System...");

    let trading_engine = Arc::new(Mutex::new(trading_engine::TradingEngine::new()));
    let risk_manager = Arc::new(Mutex::new(risk_management::RiskManager::new(10000.0, 0.1)));
    let _solana_client = Arc::new(Mutex::new(solana_integration::SolanaClient::new()));

    let market_engine = trading_engine.clone();
    tokio::spawn(async move {
        solana_integration::simulate_market_data(market_engine).await;
    });

    let signal_engine = trading_engine.clone();
    let signal_risk = risk_manager.clone();
    tokio::spawn(async move {
        trading_engine::generate_trading_signals(signal_engine, signal_risk).await;
    });

    let api_engine = trading_engine.clone();
    let api_risk = risk_manager.clone();
    log::info!("🌐 Starting Web API on port 8080...");
    api::start_server(api_engine, api_risk).await;
}
