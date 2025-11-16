use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::pda::TreasuryPDA;
use crate::rpc_client::SolanaRpcClient;
use crate::wallet::Wallet;

#[derive(Debug, Clone)]
pub struct SolanaClient {
    pub connected: bool,
    pub wallet_balance: f64,
    pub transaction_count: u64,
    pub wallet_address: Option<String>,
    pub treasury_address: Option<String>,
    pub rpc_url: Option<String>,
    pub trading_budget: f64,
}

impl SolanaClient {
    pub fn new() -> Self {
        Self {
            connected: true,
            wallet_balance: 10000.0,
            transaction_count: 0,
            wallet_address: None,
            treasury_address: None,
            rpc_url: None,
            trading_budget: 10000.0,
        }
    }

    /// Create a new SolanaClient with wallet and RPC integration
    pub async fn new_with_integration(rpc_url: String) -> Self {
        log::info!("🔐 Initializing Solana integration with wallet and PDA...");

        // Load or create wallet
        let wallet = Wallet::from_env_or_new("WALLET_PRIVATE_KEY");
        let wallet_pubkey = wallet.pubkey();

        // Derive treasury PDA for agent trading
        let treasury_pda = match TreasuryPDA::derive_default(&wallet_pubkey) {
            Ok(pda) => {
                log::info!("🏦 Treasury PDA derived: {}", pda.address);
                Some(pda)
            }
            Err(e) => {
                log::warn!("Failed to derive treasury PDA: {}", e);
                None
            }
        };

        // Create RPC client
        let rpc_client = SolanaRpcClient::new(rpc_url.clone());

        // Get wallet balance
        let wallet_balance = match rpc_client.get_balance(&wallet_pubkey).await {
            Ok(balance) => {
                log::info!("💰 Wallet balance: {} SOL", balance);
                balance
            }
            Err(e) => {
                log::warn!("Failed to get wallet balance: {}. Using simulated mode.", e);
                10000.0 // Default simulation balance
            }
        };

        // Get trading budget from environment or use default
        let trading_budget = std::env::var("TRADING_BUDGET")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(10000.0);

        log::info!("💵 Trading budget set to: ${:.2}", trading_budget);

        Self {
            connected: true,
            wallet_balance,
            transaction_count: 0,
            wallet_address: Some(wallet_pubkey.to_string()),
            treasury_address: treasury_pda.map(|pda| pda.address.to_string()),
            rpc_url: Some(rpc_url),
            trading_budget,
        }
    }

    pub async fn execute_trade(
        &mut self,
        symbol: &str,
        size: f64,
        is_buy: bool,
        price: f64,
    ) -> Result<String, String> {
        self.transaction_count += 1;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if rand::thread_rng().gen_bool(0.05) {
            return Err("Simulated trade execution failure".to_string());
        }

        let action = if is_buy { "BUY" } else { "SELL" };
        let trade_id = format!("{}_{}_{}", action, symbol, self.transaction_count);

        log::info!(
            "🔧 Executed trade: {} {} {} at ${}",
            action,
            size,
            symbol,
            price
        );

        Ok(trade_id)
    }

    pub fn get_balance(&self) -> f64 {
        self.wallet_balance
    }

    /// Get wallet address
    pub fn get_wallet_address(&self) -> Option<String> {
        self.wallet_address.clone()
    }

    /// Get treasury PDA address
    pub fn get_treasury_address(&self) -> Option<String> {
        self.treasury_address.clone()
    }

    /// Update wallet balance from RPC
    pub async fn refresh_balance(&mut self) -> Result<f64, String> {
        if let (Some(rpc_url), Some(wallet_addr)) = (&self.rpc_url, &self.wallet_address) {
            let rpc_client = SolanaRpcClient::new(rpc_url.clone());
            let pubkey = solana_sdk::pubkey::Pubkey::try_from(wallet_addr.as_str())
                .map_err(|e| format!("Invalid wallet address: {}", e))?;

            let balance = rpc_client.get_balance(&pubkey).await?;
            self.wallet_balance = balance;
            Ok(balance)
        } else {
            Err("No RPC connection configured".to_string())
        }
    }

    /// Set trading budget
    pub fn set_trading_budget(&mut self, budget: f64) -> Result<(), String> {
        if budget <= 0.0 {
            return Err("Budget must be positive".to_string());
        }

        self.trading_budget = budget;
        log::info!("💵 Trading budget updated to: ${:.2}", budget);
        Ok(())
    }

    /// Get current trading budget
    pub fn get_trading_budget(&self) -> f64 {
        self.trading_budget
    }

    /// Deposit funds to trading budget (simulated)
    pub fn deposit_funds(&mut self, amount: f64) -> Result<f64, String> {
        if amount <= 0.0 {
            return Err("Deposit amount must be positive".to_string());
        }

        self.trading_budget += amount;
        log::info!(
            "💰 Deposited ${:.2} to trading budget. New budget: ${:.2}",
            amount,
            self.trading_budget
        );
        Ok(self.trading_budget)
    }

    /// Withdraw funds from trading budget (simulated)
    pub fn withdraw_funds(&mut self, amount: f64) -> Result<f64, String> {
        if amount <= 0.0 {
            return Err("Withdrawal amount must be positive".to_string());
        }

        if amount > self.trading_budget {
            return Err(format!(
                "Insufficient funds. Budget: ${:.2}, Requested: ${:.2}",
                self.trading_budget, amount
            ));
        }

        self.trading_budget -= amount;
        log::info!(
            "💸 Withdrew ${:.2} from trading budget. New budget: ${:.2}",
            amount,
            self.trading_budget
        );
        Ok(self.trading_budget)
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
                let new_price = (base_price + price_change)
                    .max(base_price * 0.5)
                    .min(base_price * 1.5);

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
