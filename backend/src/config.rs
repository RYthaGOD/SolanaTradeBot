use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub solana: SolanaConfig,
    pub wallet: WalletConfig,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub api: ApiConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub rpc_fallbacks: Vec<String>,
    pub ws_url: String,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub encrypted_key_path: String,
    pub encryption_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub initial_capital: f64,
    pub max_position_size_percent: f64,
    pub max_drawdown_percent: f64,
    pub confidence_threshold: f64,
    pub enable_trading: bool,
    pub enable_paper_trading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub enable_risk_management: bool,
    pub max_daily_trades: u32,
    pub max_daily_loss: f64,
    pub stop_loss_percent: f64,
    pub take_profit_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub rate_limit_per_minute: u32,
    pub enable_auth: bool,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub log_level: String,
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub alert_webhook_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();
        
        Ok(AppConfig {
            solana: SolanaConfig {
                rpc_url: env::var("SOLANA_RPC_URL")
                    .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
                rpc_fallbacks: vec![
                    env::var("SOLANA_RPC_FALLBACK_1")
                        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
                    env::var("SOLANA_RPC_FALLBACK_2")
                        .unwrap_or_else(|_| "https://rpc.ankr.com/solana".to_string()),
                ],
                ws_url: env::var("SOLANA_WS_URL")
                    .unwrap_or_else(|_| "wss://api.devnet.solana.com".to_string()),
                network: env::var("SOLANA_NETWORK")
                    .unwrap_or_else(|_| "devnet".to_string()),
            },
            wallet: WalletConfig {
                encrypted_key_path: env::var("WALLET_ENCRYPTED_KEY_PATH")
                    .unwrap_or_else(|_| "./wallet/encrypted_key.json".to_string()),
                encryption_password: env::var("WALLET_ENCRYPTION_PASSWORD")
                    .unwrap_or_else(|_| "".to_string()),
            },
            trading: TradingConfig {
                initial_capital: env::var("INITIAL_CAPITAL")
                    .unwrap_or_else(|_| "10000.0".to_string())
                    .parse()?,
                max_position_size_percent: env::var("MAX_POSITION_SIZE_PERCENT")
                    .unwrap_or_else(|_| "10.0".to_string())
                    .parse()?,
                max_drawdown_percent: env::var("MAX_DRAWDOWN_PERCENT")
                    .unwrap_or_else(|_| "10.0".to_string())
                    .parse()?,
                confidence_threshold: env::var("CONFIDENCE_THRESHOLD")
                    .unwrap_or_else(|_| "0.5".to_string())
                    .parse()?,
                enable_trading: env::var("ENABLE_TRADING")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()?,
                enable_paper_trading: env::var("ENABLE_PAPER_TRADING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()?,
            },
            risk: RiskConfig {
                enable_risk_management: env::var("ENABLE_RISK_MANAGEMENT")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()?,
                max_daily_trades: env::var("MAX_DAILY_TRADES")
                    .unwrap_or_else(|_| "50".to_string())
                    .parse()?,
                max_daily_loss: env::var("MAX_DAILY_LOSS")
                    .unwrap_or_else(|_| "500.0".to_string())
                    .parse()?,
                stop_loss_percent: env::var("STOP_LOSS_PERCENT")
                    .unwrap_or_else(|_| "5.0".to_string())
                    .parse()?,
                take_profit_percent: env::var("TAKE_PROFIT_PERCENT")
                    .unwrap_or_else(|_| "10.0".to_string())
                    .parse()?,
            },
            api: ApiConfig {
                host: env::var("API_HOST")
                    .unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("API_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()?,
                rate_limit_per_minute: env::var("API_RATE_LIMIT_PER_MINUTE")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()?,
                enable_auth: env::var("ENABLE_API_AUTH")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()?,
                secret_key: env::var("API_SECRET_KEY")
                    .unwrap_or_else(|_| "insecure_default_key".to_string()),
            },
            monitoring: MonitoringConfig {
                log_level: env::var("LOG_LEVEL")
                    .unwrap_or_else(|_| "info".to_string()),
                enable_metrics: env::var("ENABLE_METRICS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()?,
                metrics_port: env::var("METRICS_PORT")
                    .unwrap_or_else(|_| "9090".to_string())
                    .parse()?,
                alert_webhook_url: env::var("ALERT_WEBHOOK_URL").ok(),
            },
        })
    }
    
    pub fn validate(&self) -> Result<(), String> {
        // Validate critical settings
        if self.trading.enable_trading && self.wallet.encryption_password.is_empty() {
            return Err("Wallet encryption password is required when trading is enabled".to_string());
        }
        
        if self.trading.enable_trading && !self.risk.enable_risk_management {
            return Err("Risk management must be enabled for live trading".to_string());
        }
        
        if self.trading.max_position_size_percent > 50.0 {
            log::warn!("⚠️ Max position size is very high ({}%). Consider reducing for safety.", 
                self.trading.max_position_size_percent);
        }
        
        if self.api.enable_auth && self.api.secret_key == "insecure_default_key" {
            return Err("API authentication requires a secure secret key".to_string());
        }
        
        Ok(())
    }
}
