mod trading_engine;
mod solana_integration;
mod risk_management;
mod ml_models;
mod api;
mod config;
mod key_management;
mod monitoring;
mod solana_rpc;
mod jupiter_integration;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::Path;
use solana_sdk::signer::Signer;
use solana_sdk::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    pretty_env_logger::init();
    
    log::info!("🚀 Starting AgentBurn Solana Trading System...");
    log::info!("📋 Version: {}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config = config::AppConfig::from_env()?;
    config.validate()?;
    
    log::info!("⚙️ Configuration loaded successfully");
    log::info!("🌐 Network: {}", config.solana.network);
    log::info!("📝 Paper trading: {}", config.trading.enable_paper_trading);
    log::info!("🔒 Live trading: {}", config.trading.enable_trading);
    
    // Initialize monitoring
    let metrics_registry = monitoring::MetricsRegistry::new();
    let alert_manager = Arc::new(monitoring::AlertManager::new(config.monitoring.alert_webhook_url.clone()));
    
    log::info!("📊 Metrics and monitoring initialized");
    
    // Send startup alert
    alert_manager.send_alert(monitoring::Alert::new(
        monitoring::AlertLevel::Info,
        "System Starting",
        format!("AgentBurn Trading System starting on {} network", config.solana.network)
    )).await;
    
    // Initialize key management
    let key_manager = key_management::KeyManager::new();
    
    // Load or create wallet keypair
    let wallet_path = Path::new(&config.wallet.encrypted_key_path);
    let keypair = if config.trading.enable_paper_trading {
        log::info!("👛 Using simulated wallet for paper trading");
        key_manager.generate_keypair()
    } else if config.trading.enable_trading {
        log::info!("👛 Loading encrypted wallet for live trading");
        key_manager.load_or_create_keypair(
            wallet_path,
            &config.wallet.encryption_password
        )?
    } else {
        log::info!("👛 Generating temporary wallet (trading disabled)");
        key_manager.generate_keypair()
    };
    
    log::info!("🔑 Wallet public key: {}", keypair.pubkey());
    
    // Initialize Solana RPC client with fallbacks
    let mut rpc_urls = vec![config.solana.rpc_url.clone()];
    rpc_urls.extend(config.solana.rpc_fallbacks.clone());
    
    let mut solana_rpc = solana_rpc::SolanaRpcClient::new(
        rpc_urls,
        config.trading.enable_paper_trading,
        CommitmentConfig::confirmed(),
    );
    
    // Perform health check
    if solana_rpc.health_check().await {
        log::info!("✅ Solana RPC connection established");
    } else {
        log::warn!("⚠️ Failed to connect to Solana RPC (using simulation mode)");
        alert_manager.send_alert(monitoring::Alert::new(
            monitoring::AlertLevel::Warning,
            "RPC Connection Failed",
            "Using simulation mode - no real RPC connection"
        )).await;
        
        if config.trading.enable_trading {
            return Err("Cannot start with live trading enabled without RPC connection".into());
        }
    }
    
    // Initialize Jupiter aggregator client
    let jupiter_client = jupiter_integration::JupiterClient::new(
        "https://quote-api.jup.ag/v6".to_string(),
        config.trading.enable_paper_trading,
    );
    
    log::info!("🔷 Jupiter Aggregator initialized");
    
    // Get wallet balance
    match solana_rpc.get_balance(&keypair.pubkey()).await {
        Ok(balance) => {
            let sol_balance = jupiter_integration::lamports_to_sol(balance);
            log::info!("💰 Wallet balance: {:.4} SOL ({} lamports)", sol_balance, balance);
        }
        Err(e) => {
            log::warn!("⚠️ Could not fetch wallet balance: {}", e);
        }
    }
    
    let solana_rpc = Arc::new(Mutex::new(solana_rpc));
    let jupiter_client = Arc::new(Mutex::new(jupiter_client));
    
    // Create legacy solana client wrapper for backward compatibility
    let mut rpc_urls_legacy = vec![config.solana.rpc_url.clone()];
    rpc_urls_legacy.extend(config.solana.rpc_fallbacks.clone());
    
    let mut solana_client = solana_integration::SolanaClient::new(
        rpc_urls_legacy,
        config.trading.enable_paper_trading
    );
    solana_client.set_wallet(&keypair);
    let solana_client = Arc::new(Mutex::new(solana_client));
    
    // Initialize trading engine with config
    let trading_engine = Arc::new(Mutex::new(trading_engine::TradingEngine::new()));
    
    // Initialize risk manager with config
    let risk_manager = Arc::new(Mutex::new(risk_management::RiskManager::new(
        config.trading.initial_capital,
        config.trading.max_drawdown_percent / 100.0
    )));
    
    log::info!("💼 Trading engine initialized with ${:.2} capital", config.trading.initial_capital);
    
    // Start market data simulation
    let market_engine = trading_engine.clone();
    tokio::spawn(async move {
        solana_integration::simulate_market_data(market_engine).await;
    });
    
    // Start trading signal generation
    let signal_engine = trading_engine.clone();
    let signal_risk = risk_manager.clone();
    let signal_solana = solana_client.clone();
    let signal_rpc = solana_rpc.clone();
    let signal_jupiter = jupiter_client.clone();
    let signal_alert = alert_manager.clone();
    tokio::spawn(async move {
        trading_engine::generate_trading_signals(
            signal_engine, 
            signal_risk, 
            signal_solana,
            signal_rpc,
            signal_jupiter,
            signal_alert
        ).await;
    });
    
    // Start metrics server if enabled
    if config.monitoring.enable_metrics {
        let metrics_port = config.monitoring.metrics_port;
        let _registry = metrics_registry.get_registry().await;
        
        tokio::spawn(async move {
            log::info!("📊 Starting metrics server on port {}", metrics_port);
            // Metrics endpoint would be served here
        });
    }
    
    // Start system health monitor
    let health_engine = trading_engine.clone();
    let health_risk = risk_manager.clone();
    let health_alert = alert_manager.clone();
    tokio::spawn(async move {
        monitor_system_health(health_engine, health_risk, health_alert).await;
    });
    
    // Start API server
    let api_engine = trading_engine.clone();
    let api_risk = risk_manager.clone();
    let api_solana = solana_client.clone();
    let api_rpc = solana_rpc.clone();
    let api_jupiter = jupiter_client.clone();
    let api_config = config.api.clone();
    
    log::info!("🌐 Starting Web API on {}:{}", api_config.host, api_config.port);
    
    alert_manager.send_alert(monitoring::Alert::new(
        monitoring::AlertLevel::Info,
        "System Started",
        "AgentBurn Trading System is now running with Jupiter integration"
    )).await;
    
    api::start_server(api_engine, api_risk, api_solana, api_rpc, api_jupiter, api_config).await;
    
    Ok(())
}

/// Monitor system health and send alerts
async fn monitor_system_health(
    _engine: Arc<Mutex<trading_engine::TradingEngine>>,
    risk_manager: Arc<Mutex<risk_management::RiskManager>>,
    alert_manager: Arc<monitoring::AlertManager>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    let start_time = chrono::Utc::now().timestamp();
    
    loop {
        interval.tick().await;
        
        let risk_lock = risk_manager.lock().await;
        let metrics = risk_lock.get_performance_metrics();
        
        // Update Prometheus metrics
        if let Some(&portfolio_value) = metrics.get("current_capital") {
            monitoring::PORTFOLIO_VALUE.set(portfolio_value);
            monitoring::ACCOUNT_BALANCE.set(portfolio_value);
        }
        
        if let Some(&pnl) = metrics.get("total_pnl") {
            monitoring::PROFIT_LOSS.set(pnl);
        }
        
        if let Some(&drawdown) = metrics.get("max_drawdown") {
            monitoring::DRAWDOWN_PERCENT.set(drawdown);
            
            // Alert on high drawdown
            if drawdown > 8.0 {
                alert_manager.send_alert(monitoring::Alert::new(
                    monitoring::AlertLevel::Warning,
                    "High Drawdown Alert",
                    format!("Current drawdown: {:.2}%", drawdown)
                )).await;
            }
        }
        
        // Update uptime
        let uptime = chrono::Utc::now().timestamp() - start_time;
        monitoring::SYSTEM_UPTIME.set(uptime);
        
        drop(risk_lock);
    }
}
