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

#[cfg(test)]
mod algorithm_tests;

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("🚀 Starting AgentBurn Solana Trading System...");
    log::info!("🤖 Enhanced with Switchboard Oracle, DEX Screener, and PumpFun integrations");

    let risk_manager = Arc::new(Mutex::new(risk_management::RiskManager::new(10000.0, 0.1)));
    let trading_engine = Arc::new(Mutex::new(trading_engine::TradingEngine::new(risk_manager.clone())));
    let _solana_client = Arc::new(Mutex::new(solana_integration::SolanaClient::new()));

    // Start market data simulation
    let market_engine = trading_engine.clone();
    tokio::spawn(async move {
        solana_integration::simulate_market_data(market_engine).await;
    });

    // Start traditional signal generation
    let signal_engine = trading_engine.clone();
    let signal_risk = risk_manager.clone();
    tokio::spawn(async move {
        trading_engine::generate_trading_signals(signal_engine, signal_risk).await;
    });

    // Initialize Signal Marketplace
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let marketplace = Arc::new(signal_platform::SignalMarketplace::new(rpc_url.clone()));
    
    // Initialize 6 Specialized Provider Agents
    log::info!("🤖 Initializing 6 Specialized Signal Providers...");
    let providers = specialized_providers::initialize_all_providers(
        marketplace.clone(),
        rpc_url.clone(),
    ).await;
    
    log::info!("✅ Initialized {} specialized providers", providers.len());
    
    // Start each specialized provider in its own task
    for provider in providers {
        tokio::spawn(async move {
            provider.run().await;
        });
    }
    
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

    let api_engine = trading_engine.clone();
    let api_risk = risk_manager.clone();
    log::info!("🌐 Starting Web API on port 8080...");
    api::start_server(api_engine, api_risk).await;
}
