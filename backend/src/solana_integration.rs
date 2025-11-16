use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct SolanaClient {
    pub connected: bool,
    pub wallet_balance: f64,
    pub transaction_count: u64,
}

impl SolanaClient {
    pub fn new() -> Self {
        Self {
            connected: true,
            wallet_balance: 10000.0,
            transaction_count: 0,
        }
    }
    
    pub async fn execute_trade(&mut self, symbol: &str, size: f64, is_buy: bool, price: f64) -> Result<String, String> {
        self.transaction_count += 1;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        if rand::thread_rng().gen_bool(0.05) {
            return Err("Simulated trade execution failure".to_string());
        }
        
        let action = if is_buy { "BUY" } else { "SELL" };
        let trade_id = format!("{}_{}_{}", action, symbol, self.transaction_count);
        
        log::info!("🔧 Executed trade: {} {} {} at ${}", action, size, symbol, price);
        
        Ok(trade_id)
    }
    
    pub fn get_balance(&self) -> f64 {
        self.wallet_balance
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
                log::info!("🎯 Generated trading signal: {:?}", signal);
            }
        }
    }
}
