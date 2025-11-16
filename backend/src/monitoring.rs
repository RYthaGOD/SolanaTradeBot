use prometheus::{
    Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Registry,
};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static! {
    // Trading Metrics
    pub static ref TRADES_TOTAL: IntCounter = IntCounter::new(
        "trades_total",
        "Total number of trades executed"
    ).expect("metric can be created");
    
    pub static ref TRADES_SUCCESSFUL: IntCounter = IntCounter::new(
        "trades_successful",
        "Number of successful trades"
    ).expect("metric can be created");
    
    pub static ref TRADES_FAILED: IntCounter = IntCounter::new(
        "trades_failed",
        "Number of failed trades"
    ).expect("metric can be created");
    
    pub static ref PORTFOLIO_VALUE: Gauge = Gauge::new(
        "portfolio_value_usd",
        "Current portfolio value in USD"
    ).expect("metric can be created");
    
    pub static ref ACCOUNT_BALANCE: Gauge = Gauge::new(
        "account_balance_usd",
        "Current account balance in USD"
    ).expect("metric can be created");
    
    pub static ref PROFIT_LOSS: Gauge = Gauge::new(
        "profit_loss_usd",
        "Current profit/loss in USD"
    ).expect("metric can be created");
    
    pub static ref DRAWDOWN_PERCENT: Gauge = Gauge::new(
        "drawdown_percent",
        "Current drawdown percentage"
    ).expect("metric can be created");
    
    // RPC Metrics
    pub static ref RPC_REQUESTS_TOTAL: IntCounter = IntCounter::new(
        "rpc_requests_total",
        "Total number of RPC requests"
    ).expect("metric can be created");
    
    pub static ref RPC_ERRORS_TOTAL: IntCounter = IntCounter::new(
        "rpc_errors_total",
        "Total number of RPC errors"
    ).expect("metric can be created");
    
    pub static ref RPC_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new("rpc_latency_seconds", "RPC request latency in seconds")
    ).expect("metric can be created");
    
    // Market Data Metrics
    pub static ref MARKET_DATA_UPDATES: IntCounter = IntCounter::new(
        "market_data_updates_total",
        "Total number of market data updates received"
    ).expect("metric can be created");
    
    pub static ref PRICE_ORACLE_ERRORS: IntCounter = IntCounter::new(
        "price_oracle_errors_total",
        "Total number of price oracle errors"
    ).expect("metric can be created");
    
    // Signal Generation Metrics
    pub static ref SIGNALS_GENERATED: IntCounter = IntCounter::new(
        "signals_generated_total",
        "Total number of trading signals generated"
    ).expect("metric can be created");
    
    pub static ref SIGNALS_EXECUTED: IntCounter = IntCounter::new(
        "signals_executed_total",
        "Number of signals that resulted in trades"
    ).expect("metric can be created");
    
    pub static ref SIGNALS_REJECTED: IntCounter = IntCounter::new(
        "signals_rejected_total",
        "Number of signals rejected by risk management"
    ).expect("metric can be created");
    
    // System Health Metrics
    pub static ref SYSTEM_UPTIME: IntGauge = IntGauge::new(
        "system_uptime_seconds",
        "System uptime in seconds"
    ).expect("metric can be created");
    
    pub static ref ACTIVE_POSITIONS: IntGauge = IntGauge::new(
        "active_positions",
        "Number of currently active positions"
    ).expect("metric can be created");
}

pub struct MetricsRegistry {
    registry: Arc<RwLock<Registry>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Registry::new();
        
        // Register all metrics
        registry.register(Box::new(TRADES_TOTAL.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(TRADES_SUCCESSFUL.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(TRADES_FAILED.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(PORTFOLIO_VALUE.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(ACCOUNT_BALANCE.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(PROFIT_LOSS.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(DRAWDOWN_PERCENT.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(RPC_REQUESTS_TOTAL.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(RPC_ERRORS_TOTAL.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(RPC_LATENCY.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(MARKET_DATA_UPDATES.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(PRICE_ORACLE_ERRORS.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(SIGNALS_GENERATED.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(SIGNALS_EXECUTED.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(SIGNALS_REJECTED.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(SYSTEM_UPTIME.clone()))
            .expect("collector can be registered");
        registry.register(Box::new(ACTIVE_POSITIONS.clone()))
            .expect("collector can be registered");
        
        Self {
            registry: Arc::new(RwLock::new(registry)),
        }
    }
    
    pub async fn get_registry(&self) -> Arc<RwLock<Registry>> {
        self.registry.clone()
    }
    
    pub async fn gather_metrics(&self) -> String {
        use prometheus::Encoder;
        let registry = self.registry.read().await;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert levels for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert message structure
#[derive(Debug, Clone)]
pub struct Alert {
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub timestamp: i64,
}

impl Alert {
    pub fn new(level: AlertLevel, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            title: title.into(),
            message: message.into(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Alerting system
pub struct AlertManager {
    webhook_url: Option<String>,
    client: reqwest::Client,
}

impl AlertManager {
    pub fn new(webhook_url: Option<String>) -> Self {
        Self {
            webhook_url,
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn send_alert(&self, alert: Alert) {
        // Log the alert
        match alert.level {
            AlertLevel::Info => log::info!("📢 {}: {}", alert.title, alert.message),
            AlertLevel::Warning => log::warn!("⚠️ {}: {}", alert.title, alert.message),
            AlertLevel::Error => log::error!("🚨 {}: {}", alert.title, alert.message),
            AlertLevel::Critical => log::error!("💥 CRITICAL - {}: {}", alert.title, alert.message),
        }
        
        // Send to webhook if configured
        if let Some(webhook_url) = &self.webhook_url {
            if let Err(e) = self.send_webhook(webhook_url, &alert).await {
                log::error!("Failed to send alert to webhook: {}", e);
            }
        }
    }
    
    async fn send_webhook(&self, url: &str, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        let emoji = match alert.level {
            AlertLevel::Info => "ℹ️",
            AlertLevel::Warning => "⚠️",
            AlertLevel::Error => "🚨",
            AlertLevel::Critical => "💥",
        };
        
        let payload = serde_json::json!({
            "text": format!("{} *{}* - {}\n{}", emoji, alert.level_str(), alert.title, alert.message),
            "username": "AgentBurn Trading Bot",
            "icon_emoji": ":robot_face:"
        });
        
        self.client
            .post(url)
            .json(&payload)
            .send()
            .await?;
        
        Ok(())
    }
}

impl Alert {
    fn level_str(&self) -> &'static str {
        match self.level {
            AlertLevel::Info => "INFO",
            AlertLevel::Warning => "WARNING",
            AlertLevel::Error => "ERROR",
            AlertLevel::Critical => "CRITICAL",
        }
    }
}

/// Health check status
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub uptime_seconds: u64,
    pub rpc_connected: bool,
    pub trading_enabled: bool,
    pub last_market_update: Option<i64>,
    pub active_positions: i32,
    pub error_count: u64,
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            healthy: true,
            uptime_seconds: 0,
            rpc_connected: false,
            trading_enabled: false,
            last_market_update: None,
            active_positions: 0,
            error_count: 0,
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}
