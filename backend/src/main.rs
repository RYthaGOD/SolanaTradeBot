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
mod prediction_markets;

#[cfg(test)]
mod algorithm_tests;

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("🚀 Starting AgentBurn Solana Trading System...");
    log::info!("🤖 Enhanced with Switchboard Oracle, DEX Screener, and PumpFun integrations");

    // Get RPC URL from environment
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    
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
    
    // Use new_default for trading engine (includes built-in risk manager)
    let trading_engine = Arc::new(Mutex::new(trading_engine::TradingEngine::new(risk_manager.clone())));

    // Initialize WebSocket broadcaster for real-time updates
    log::info!("📡 Initializing WebSocket broadcaster...");
    let ws_broadcaster = websocket::create_ws_broadcaster();
    
    // Initialize Reinforcement Learning Coordinator
    log::info!("🤖 Initializing RL Coordinator...");
    let rl_coordinator = Arc::new(Mutex::new(reinforcement_learning::LearningCoordinator::new()));
    
    // Initialize Meme Analyzer for memecoin analysis
    log::info!("🎪 Initializing Meme Analyzer...");
    let meme_analyzer = Arc::new(Mutex::new(pumpfun::MemeAnalyzer::new()));
    
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
    let marketplace = Arc::new(signal_platform::SignalMarketplace::new(rpc_url.clone()));
    
    // Initialize 6 Specialized Provider Agents with RL integration
    log::info!("🤖 Initializing 6 Specialized Signal Providers with RL...");
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
    
    // Start each specialized provider in its own task
    for provider in rl_connected_providers {
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

    // Start both APIs in parallel
    let api_engine = trading_engine.clone();
    let api_risk = risk_manager.clone();
    let api_solana = solana_client.clone();
    let api_orchestrator = ai_orchestrator.clone();
    
    log::info!("🌐 Starting Legacy API on port 8080 and AI-Orchestrated API v2 on port 8081...");
    
    // Start legacy API in background
    let legacy_api = tokio::spawn(async move {
        api::start_server(api_engine, api_risk, api_solana).await;
    });
    
    // Start new AI-orchestrated API v2 in background
    let ai_api = tokio::spawn(async move {
        api_v2::start_server(api_orchestrator).await;
    });
    
    // Wait for both servers (they run forever)
    let _ = tokio::try_join!(legacy_api, ai_api);
}
