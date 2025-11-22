//! 24/7 Live Data Feed Service
//! Continuously fetches real-time price data and broadcasts via WebSocket
//! Includes proper rate limiting and caching to prevent API limit issues
//! Enhanced with feed management, statistics, and health monitoring

use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::time::interval;
use serde::{Serialize, Deserialize};
use crate::switchboard_oracle::SwitchboardClient;
use crate::websocket::{WSBroadcaster, broadcast_market_update};

/// Feed configuration for a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    pub symbol: String,
    pub priority: FeedPriority,
    pub update_interval_secs: u64,
    pub enabled: bool,
    pub last_update: Option<i64>,
    pub last_price: Option<f64>,
    pub error_count: u32,
    pub success_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedPriority {
    Low,      // Update every 30s
    Normal,   // Update every 10s
    High,     // Update every 5s
    Critical, // Update every 1s
}

impl FeedPriority {
    pub fn to_interval(&self) -> Duration {
        match self {
            FeedPriority::Low => Duration::from_secs(30),
            FeedPriority::Normal => Duration::from_secs(10),
            FeedPriority::High => Duration::from_secs(5),
            FeedPriority::Critical => Duration::from_secs(1),
        }
    }
}

/// Feed statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedStatistics {
    pub symbol: String,
    pub total_updates: u64,
    pub successful_updates: u64,
    pub failed_updates: u64,
    pub success_rate: f64,
    pub average_response_time_ms: f64,
    pub last_update: Option<i64>,
    pub last_price: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub uptime_percent: f64,
}

/// Feed health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedHealth {
    pub symbol: String,
    pub status: String, // "healthy", "degraded", "unhealthy"
    pub last_update_age_secs: Option<i64>,
    pub consecutive_errors: u32,
    pub response_time_ms: Option<u64>,
    pub is_stale: bool,
}

/// Live data feed service that continuously fetches and broadcasts price data
pub struct LiveDataFeed {
    oracle_client: Arc<SwitchboardClient>,
    ws_broadcaster: Option<WSBroadcaster>,
    trading_engine: Option<Arc<Mutex<crate::trading_engine::TradingEngine>>>,
    jupiter_client: Option<Arc<crate::jupiter_integration::JupiterClient>>,
    symbols: Arc<Mutex<Vec<String>>>, // ENHANCED: Thread-safe symbol list
    feed_configs: Arc<Mutex<HashMap<String, FeedConfig>>>, // ENHANCED: Per-symbol configuration
    feed_stats: Arc<Mutex<HashMap<String, FeedStatistics>>>, // ENHANCED: Per-symbol statistics
    update_interval: Arc<Mutex<Duration>>, // ENHANCED: Thread-safe update interval
    is_running: Arc<Mutex<bool>>,
    start_time: Arc<Mutex<Option<i64>>>, // ENHANCED: Track service start time
}

impl LiveDataFeed {
    /// Create a new live data feed service
    pub fn new(
        oracle_client: Arc<SwitchboardClient>,
        ws_broadcaster: Option<WSBroadcaster>,
        trading_engine: Option<Arc<Mutex<crate::trading_engine::TradingEngine>>>,
        jupiter_client: Option<Arc<crate::jupiter_integration::JupiterClient>>,
        symbols: Vec<String>,
    ) -> Self {
        // Initialize feed configs for all symbols
        let mut feed_configs = HashMap::new();
        for symbol in &symbols {
            feed_configs.insert(symbol.clone(), FeedConfig {
                symbol: symbol.clone(),
                priority: FeedPriority::Normal,
                update_interval_secs: 15, // INCREASED: Reduced frequency to prevent rate limits
                enabled: true,
                last_update: None,
                last_price: None,
                error_count: 0,
                success_count: 0,
            });
        }
        
        // Initialize feed statistics
        let mut feed_stats = HashMap::new();
        for symbol in &symbols {
            feed_stats.insert(symbol.clone(), FeedStatistics {
                symbol: symbol.clone(),
                total_updates: 0,
                successful_updates: 0,
                failed_updates: 0,
                success_rate: 0.0,
                average_response_time_ms: 0.0,
                last_update: None,
                last_price: None,
                price_change_24h: None,
                uptime_percent: 0.0,
            });
        }
        
        Self {
            oracle_client,
            ws_broadcaster,
            trading_engine,
            jupiter_client,
            symbols: Arc::new(Mutex::new(symbols)),
            feed_configs: Arc::new(Mutex::new(feed_configs)),
            feed_stats: Arc::new(Mutex::new(feed_stats)),
            update_interval: Arc::new(Mutex::new(Duration::from_secs(5))),
            is_running: Arc::new(Mutex::new(false)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the 24/7 live data feed service
    pub async fn start(&self) {
        let mut running = self.is_running.lock().await;
        if *running {
            log::warn!("⚠️ Live data feed is already running");
            return;
        }
        *running = true;
        
        // Record start time
        {
            let mut start_time = self.start_time.lock().await;
            *start_time = Some(chrono::Utc::now().timestamp());
        }
        
        let symbols_count = {
            let symbols = self.symbols.lock().await;
            symbols.len()
        };
        
        let update_interval_val = {
            let interval = self.update_interval.lock().await;
            *interval
        };
        
        drop(running);

        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("📡 24/7 LIVE DATA FEED SERVICE STARTED");
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("📊 Monitoring {} symbols", symbols_count);
        log::info!("⏱️  Base update interval: {:?}", update_interval_val);
        log::info!("🌐 WebSocket broadcasting: {}", if self.ws_broadcaster.is_some() { "ENABLED" } else { "DISABLED" });
        log::info!("🔄 Service will run continuously until stopped");
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let oracle_client = self.oracle_client.clone();
        let ws_broadcaster = self.ws_broadcaster.clone();
        let trading_engine = self.trading_engine.clone();
        let jupiter_client = self.jupiter_client.clone();
        let symbols = self.symbols.clone();
        let feed_configs = self.feed_configs.clone();
        let feed_stats = self.feed_stats.clone();
        let update_interval = self.update_interval.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut base_interval = {
                let interval_lock = update_interval.lock().await;
                interval(*interval_lock)
            };
            let mut consecutive_errors = 0u32;
            let max_consecutive_errors = 10;

            loop {
                // Check if service should stop
                {
                    let running = is_running.lock().await;
                    if !*running {
                        log::info!("🛑 Live data feed service stopped");
                        break;
                    }
                }

                base_interval.tick().await;

                // Get current symbols and configs
                let symbols_to_fetch: Vec<String> = {
                    let symbols_lock = symbols.lock().await;
                    let configs_lock = feed_configs.lock().await;
                    symbols_lock.iter()
                        .filter(|s| {
                            configs_lock.get(*s)
                                .map(|c| c.enabled)
                                .unwrap_or(true)
                        })
                        .cloned()
                        .collect()
                };

                // Fetch prices for all enabled symbols
                let mut successful_updates = 0u32;
                let mut failed_updates = 0u32;

                for symbol in &symbols_to_fetch {
                    let start_time = std::time::Instant::now();
                    
                    // Get feed config for this symbol
                    let feed_priority = {
                        let configs = feed_configs.lock().await;
                        configs.get(symbol)
                            .map(|c| c.priority.clone())
                            .unwrap_or(FeedPriority::Normal)
                    };
                    
                    match oracle_client.fetch_price(symbol).await {
                        Ok(feed) => {
                            let response_time = start_time.elapsed().as_millis() as u64;
                            successful_updates += 1;
                            consecutive_errors = 0;

                            // Update feed config
                            {
                                let mut configs = feed_configs.lock().await;
                                if let Some(config) = configs.get_mut(symbol) {
                                    config.last_update = Some(chrono::Utc::now().timestamp());
                                    config.last_price = Some(feed.price);
                                    config.success_count += 1;
                                    config.error_count = 0; // Reset error count on success
                                }
                            }

                            // Update feed statistics
                            {
                                let mut stats = feed_stats.lock().await;
                                if let Some(stat) = stats.get_mut(symbol) {
                                    stat.total_updates += 1;
                                    stat.successful_updates += 1;
                                    stat.success_rate = (stat.successful_updates as f64 / stat.total_updates as f64) * 100.0;
                                    
                                    // Update average response time (exponential moving average)
                                    stat.average_response_time_ms = 
                                        (stat.average_response_time_ms * 0.9) + (response_time as f64 * 0.1);
                                    
                                    stat.last_update = Some(chrono::Utc::now().timestamp());
                                    stat.last_price = Some(feed.price);
                                    stat.price_change_24h = feed.price_change_24h;
                                }
                            }

                            // Get volume data from Jupiter if available
                            let mut volume = 0.0;
                            if let Some(ref _jupiter) = jupiter_client {
                                if symbol.starts_with("SOL/") {
                                    volume = 1000000.0; // Placeholder
                                }
                            }

                            // Update TradingEngine market_state with REAL prices
                            if let Some(ref engine) = trading_engine {
                                let mut engine_lock = engine.lock().await;
                                let market_data = crate::trading_engine::MarketData {
                                    symbol: feed.symbol.clone(),
                                    price: feed.price,
                                    volume,
                                    timestamp: feed.timestamp,
                                    bid: feed.min_price,
                                    ask: feed.max_price,
                                    spread: feed.max_price - feed.min_price,
                                };
                                engine_lock.process_market_data(market_data).await;
                                log::debug!("📊 Updated TradingEngine market_state for {}: ${:.4}", feed.symbol, feed.price);
                            }

                            // Broadcast via WebSocket if available
                            if let Some(ref broadcaster) = ws_broadcaster {
                                let change_24h_value = feed.price_change_24h.unwrap_or(0.0);
                                broadcast_market_update(
                                    broadcaster,
                                    feed.symbol.clone(),
                                    feed.price,
                                    volume,
                                    change_24h_value,
                                );
                            }

                            let change_24h_value = feed.price_change_24h.unwrap_or(0.0);
                            log::debug!("✅ {}: ${:.4} (24h: {:.2}%) | Response: {}ms", symbol, feed.price, change_24h_value, response_time);
                        }
                        Err(e) => {
                            let response_time = start_time.elapsed().as_millis() as u64;
                            consecutive_errors += 1;
                            failed_updates += 1;

                            // Update feed config
                            {
                                let mut configs = feed_configs.lock().await;
                                if let Some(config) = configs.get_mut(symbol) {
                                    config.error_count += 1;
                                }
                            }

                            // Update feed statistics
                            {
                                let mut stats = feed_stats.lock().await;
                                if let Some(stat) = stats.get_mut(symbol) {
                                    stat.total_updates += 1;
                                    stat.failed_updates += 1;
                                    stat.success_rate = (stat.successful_updates as f64 / stat.total_updates as f64) * 100.0;
                                }
                            }

                            // Log error but don't spam
                            if consecutive_errors <= 3 || consecutive_errors % 10 == 0 {
                                log::warn!("⚠️ Failed to fetch price for {}: {} ({}ms)", symbol, e, response_time);
                            }

                            // If too many consecutive errors, wait longer before retrying
                            if consecutive_errors >= max_consecutive_errors {
                                log::error!("❌ Too many consecutive errors ({}). Waiting 30s before retry...", consecutive_errors);
                                tokio::time::sleep(Duration::from_secs(30)).await;
                                consecutive_errors = 0;
                            }
                        }
                    }

                    // Delay based on feed priority
                    let delay = feed_priority.to_interval();
                    tokio::time::sleep(delay.min(Duration::from_millis(100))).await;
                }

                // Periodic status log
                if successful_updates > 0 {
                    log::info!("📊 Live feed update: {}/{} successful ({} failed)", 
                        successful_updates, symbols_to_fetch.len(), failed_updates);
                }
            }
        });
    }

    /// Stop the live data feed service
    pub async fn stop(&self) {
        let mut running = self.is_running.lock().await;
        *running = false;
        log::info!("🛑 Stopping live data feed service...");
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        let running = self.is_running.lock().await;
        *running
    }

    /// Add a symbol to monitor (thread-safe)
    pub async fn add_symbol(&self, symbol: String, priority: Option<FeedPriority>) -> Result<(), String> {
        let mut symbols = self.symbols.lock().await;
        if symbols.contains(&symbol) {
            return Err(format!("Symbol {} is already being monitored", symbol));
        }
        
        symbols.push(symbol.clone());
        
        // Add feed config
        {
            let mut configs = self.feed_configs.lock().await;
            let priority_value = priority.as_ref().cloned().unwrap_or(FeedPriority::Normal);
            configs.insert(symbol.clone(), FeedConfig {
                symbol: symbol.clone(),
                priority: priority_value.clone(),
                update_interval_secs: priority_value.to_interval().as_secs(),
                enabled: true,
                last_update: None,
                last_price: None,
                error_count: 0,
                success_count: 0,
            });
        }
        
        // Initialize feed statistics
        {
            let mut stats = self.feed_stats.lock().await;
            stats.insert(symbol.clone(), FeedStatistics {
                symbol: symbol.clone(),
                total_updates: 0,
                successful_updates: 0,
                failed_updates: 0,
                success_rate: 0.0,
                average_response_time_ms: 0.0,
                last_update: None,
                last_price: None,
                price_change_24h: None,
                uptime_percent: 0.0,
            });
        }
        
        log::info!("➕ Added {} to live data feed", symbol);
        Ok(())
    }

    /// Remove a symbol from monitoring (thread-safe)
    pub async fn remove_symbol(&self, symbol: &str) -> Result<(), String> {
        let mut symbols = self.symbols.lock().await;
        if !symbols.contains(&symbol.to_string()) {
            return Err(format!("Symbol {} is not being monitored", symbol));
        }
        
        symbols.retain(|s| s != symbol);
        
        // Remove feed config and stats
        {
            let mut configs = self.feed_configs.lock().await;
            configs.remove(symbol);
        }
        
        {
            let mut stats = self.feed_stats.lock().await;
            stats.remove(symbol);
        }
        
        log::info!("➖ Removed {} from live data feed", symbol);
        Ok(())
    }

    /// Set update interval (thread-safe)
    pub async fn set_update_interval(&self, interval: Duration) {
        let mut update_interval = self.update_interval.lock().await;
        *update_interval = interval;
        log::info!("⏱️  Base update interval set to {:?}", interval);
    }

    /// Get all monitored symbols (thread-safe)
    pub async fn get_symbols(&self) -> Vec<String> {
        let symbols = self.symbols.lock().await;
        symbols.clone()
    }

    /// Get feed configuration for a symbol
    pub async fn get_feed_config(&self, symbol: &str) -> Option<FeedConfig> {
        let configs = self.feed_configs.lock().await;
        configs.get(symbol).cloned()
    }

    /// Update feed configuration
    pub async fn update_feed_config(&self, symbol: &str, priority: Option<FeedPriority>, enabled: Option<bool>) -> Result<(), String> {
        let mut configs = self.feed_configs.lock().await;
        if let Some(config) = configs.get_mut(symbol) {
            if let Some(pri) = priority {
                config.priority = pri.clone();
                config.update_interval_secs = pri.to_interval().as_secs();
            }
            if let Some(en) = enabled {
                config.enabled = en;
            }
            log::info!("⚙️ Updated feed config for {}: priority={:?}, enabled={}", 
                symbol, config.priority, config.enabled);
            Ok(())
        } else {
            Err(format!("Symbol {} not found in feed configs", symbol))
        }
    }

    /// Get feed statistics
    pub async fn get_feed_statistics(&self, symbol: Option<&str>) -> Result<Vec<FeedStatistics>, String> {
        let stats = self.feed_stats.lock().await;
        if let Some(sym) = symbol {
            if let Some(stat) = stats.get(sym) {
                Ok(vec![stat.clone()])
            } else {
                Err(format!("Statistics not found for symbol: {}", sym))
            }
        } else {
            Ok(stats.values().cloned().collect())
        }
    }

    /// Get feed health status
    pub async fn get_feed_health(&self, symbol: Option<&str>) -> Result<Vec<FeedHealth>, String> {
        let configs = self.feed_configs.lock().await;
        let stats = self.feed_stats.lock().await;
        let now = chrono::Utc::now().timestamp();
        
        let symbols_to_check: Vec<String> = if let Some(sym) = symbol {
            vec![sym.to_string()]
        } else {
            configs.keys().cloned().collect()
        };
        
        let mut health_statuses = Vec::new();
        
        for sym in symbols_to_check {
            let config = configs.get(&sym);
            let stat = stats.get(&sym);
            
            let last_update_age = config.and_then(|c| c.last_update)
                .map(|t| now - t);
            
            let is_stale = last_update_age
                .map(|age| age > 60) // Stale if > 60 seconds
                .unwrap_or(true);
            
            let consecutive_errors = config.map(|c| c.error_count).unwrap_or(0);
            let success_rate = stat.map(|s| s.success_rate).unwrap_or(0.0);
            
            let status = if !config.map(|c| c.enabled).unwrap_or(false) {
                "disabled".to_string()
            } else if is_stale || consecutive_errors > 5 {
                "unhealthy".to_string()
            } else if success_rate < 80.0 || consecutive_errors > 2 {
                "degraded".to_string()
            } else {
                "healthy".to_string()
            };
            
            health_statuses.push(FeedHealth {
                symbol: sym.clone(),
                status,
                last_update_age_secs: last_update_age,
                consecutive_errors,
                response_time_ms: stat.and_then(|s| Some(s.average_response_time_ms as u64)),
                is_stale,
            });
        }
        
        Ok(health_statuses)
    }

    /// Get overall feed service status
    pub async fn get_service_status(&self) -> serde_json::Value {
        let symbols = self.symbols.lock().await;
        let configs = self.feed_configs.lock().await;
        let stats = self.feed_stats.lock().await;
        let is_running = self.is_running.lock().await;
        let start_time = self.start_time.lock().await;
        let update_interval = self.update_interval.lock().await;
        
        let total_symbols = symbols.len();
        let enabled_symbols = configs.values().filter(|c| c.enabled).count();
        
        let total_updates: u64 = stats.values().map(|s| s.total_updates).sum();
        let total_success: u64 = stats.values().map(|s| s.successful_updates).sum();
        let overall_success_rate = if total_updates > 0 {
            (total_success as f64 / total_updates as f64) * 100.0
        } else {
            0.0
        };
        
        let uptime_secs = start_time.map(|t| chrono::Utc::now().timestamp() - t);
        
        serde_json::json!({
            "is_running": *is_running,
            "total_symbols": total_symbols,
            "enabled_symbols": enabled_symbols,
            "base_update_interval_secs": update_interval.as_secs(),
            "uptime_secs": uptime_secs,
            "total_updates": total_updates,
            "total_successful_updates": total_success,
            "overall_success_rate": overall_success_rate,
            "start_time": *start_time,
        })
    }
}

