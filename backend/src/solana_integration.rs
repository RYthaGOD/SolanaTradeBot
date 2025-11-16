use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_client::client_error::ClientError;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

/// Solana client with RPC fallback support
#[derive(Clone)]
pub struct SolanaClient {
    pub rpc_urls: Vec<String>,
    pub current_rpc_index: usize,
    pub connected: bool,
    pub wallet_pubkey: Option<Pubkey>,
    pub transaction_count: u64,
    pub paper_trading_mode: bool,
}

impl std::fmt::Debug for SolanaClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolanaClient")
            .field("current_rpc", &self.rpc_urls.get(self.current_rpc_index))
            .field("connected", &self.connected)
            .field("wallet_pubkey", &self.wallet_pubkey)
            .field("transaction_count", &self.transaction_count)
            .field("paper_trading_mode", &self.paper_trading_mode)
            .finish()
    }
}

impl SolanaClient {
    pub fn new(rpc_urls: Vec<String>, paper_trading_mode: bool) -> Self {
        log::info!("🌐 Initializing Solana client with {} RPC endpoints", rpc_urls.len());
        log::info!("📝 Paper trading mode: {}", paper_trading_mode);
        
        Self {
            rpc_urls,
            current_rpc_index: 0,
            connected: false,
            wallet_pubkey: None,
            transaction_count: 0,
            paper_trading_mode,
        }
    }
    
    /// Get RPC client with automatic fallback
    fn get_rpc_client(&mut self) -> Result<RpcClient, ClientError> {
        let url = &self.rpc_urls[self.current_rpc_index];
        log::debug!("Connecting to RPC: {}", url);
        
        let client = RpcClient::new_with_timeout(
            url.clone(),
            Duration::from_secs(30),
        );
        
        // Test connection
        match client.get_health() {
            Ok(_) => {
                self.connected = true;
                super::monitoring::RPC_REQUESTS_TOTAL.inc();
                Ok(client)
            }
            Err(e) => {
                super::monitoring::RPC_ERRORS_TOTAL.inc();
                log::warn!("❌ RPC connection failed: {}. Trying fallback...", e);
                
                // Try next RPC endpoint
                self.current_rpc_index = (self.current_rpc_index + 1) % self.rpc_urls.len();
                
                if self.current_rpc_index == 0 {
                    // We've tried all endpoints
                    self.connected = false;
                    Err(ClientError::from(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "All RPC endpoints failed"
                    )))
                } else {
                    // Retry with next endpoint
                    self.get_rpc_client()
                }
            }
        }
    }
    
    /// Set wallet pubkey
    pub fn set_wallet(&mut self, keypair: &Keypair) {
        self.wallet_pubkey = Some(keypair.pubkey());
        log::info!("👛 Wallet configured: {}", keypair.pubkey());
    }
    
    /// Get SOL balance for configured wallet
    pub async fn get_balance(&mut self) -> Result<f64, String> {
        if self.paper_trading_mode {
            return Ok(10000.0); // Simulated balance
        }
        
        let wallet_pubkey = self.wallet_pubkey
            .ok_or_else(|| "Wallet not configured".to_string())?;
        
        let client = self.get_rpc_client()
            .map_err(|e| format!("Failed to connect to RPC: {}", e))?;
        
        let balance = client.get_balance(&wallet_pubkey)
            .map_err(|e| format!("Failed to get balance: {}", e))?;
        
        // Convert lamports to SOL
        Ok(balance as f64 / 1_000_000_000.0)
    }
    
    /// Execute trade (simulated or real depending on mode)
    pub async fn execute_trade(&mut self, symbol: &str, size: f64, is_buy: bool, price: f64) -> Result<String, String> {
        self.transaction_count += 1;
        super::monitoring::TRADES_TOTAL.inc();
        
        let action = if is_buy { "BUY" } else { "SELL" };
        
        if self.paper_trading_mode {
            // Simulate trade execution
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            if rand::thread_rng().gen_bool(0.05) {
                super::monitoring::TRADES_FAILED.inc();
                return Err("Simulated trade execution failure".to_string());
            }
            
            let trade_id = format!("PAPER_{}_{}_{}", action, symbol, self.transaction_count);
            log::info!("📝 Paper trade executed: {} {} {} at ${}", action, size, symbol, price);
            super::monitoring::TRADES_SUCCESSFUL.inc();
            
            Ok(trade_id)
        } else {
            // Real trade execution would go here
            // This would integrate with Jupiter Aggregator or specific DEX
            log::warn!("⚠️ Real trading not yet implemented. Enable paper_trading_mode.");
            Err("Real trading not implemented".to_string())
        }
    }
    
    /// Health check for RPC connection
    pub async fn health_check(&mut self) -> bool {
        match self.get_rpc_client() {
            Ok(_) => {
                self.connected = true;
                true
            }
            Err(_) => {
                self.connected = false;
                false
            }
        }
    }
}

pub async fn simulate_market_data(engine: Arc<Mutex<super::trading_engine::TradingEngine>>) {
    log::info!("📊 Starting market data simulation");
    
    let symbols = vec!["SOL/USDC", "BTC/USDC", "ETH/USDC"];
    let mut prices = HashMap::new();
    prices.insert("SOL/USDC".to_string(), 100.0);
    prices.insert("BTC/USDC".to_string(), 50000.0);
    prices.insert("ETH/USDC".to_string(), 3000.0);
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
    
    loop {
        interval.tick().await;
        
        let mut market_updates = Vec::new();
        
        {
            let mut rng = rand::thread_rng();
            for symbol in &symbols {
                let base_price = prices.get(*symbol).unwrap();
                let price_change = (rng.gen::<f64>() - 0.5) * base_price * 0.02;
                let new_price = (base_price + price_change).max(base_price * 0.5).min(base_price * 1.5);
                
                prices.insert(symbol.to_string(), new_price);
                
                let market_data = super::trading_engine::MarketData {
                    symbol: symbol.to_string(),
                    price: new_price,
                    volume: rng.gen::<f64>() * 1000000.0,
                    timestamp: chrono::Utc::now().timestamp(),
                    bid: new_price * 0.999,
                    ask: new_price * 1.001,
                    spread: new_price * 0.002,
                };
                
                market_updates.push(market_data);
            }
        }
        
        for market_data in market_updates {
            let mut engine_lock = engine.lock().await;
            if let Some(signal) = engine_lock.process_market_data(market_data).await {
                log::info!("🎯 Generated trading signal: {} {:?} @ ${:.2} (confidence: {:.2})", 
                    signal.symbol, signal.action, signal.price, signal.confidence);
            }
        }
    }
}
