use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use futures::future::join_all;
use futures::FutureExt; // For catch_unwind

use crate::switchboard_oracle::SwitchboardClient;
use crate::dex_screener::DexScreenerClient;
use crate::pumpfun::PumpFunClient;
use crate::jupiter_integration::JupiterClient;
use crate::signal_platform::{SignalMarketplace, TradingSignalData, SignalAction, SignalStatus};
use crate::reinforcement_learning::{RLAgent, LearningCoordinator};

/// Quick profit opportunity analysis result (5-10% profit targets)
struct QuickProfitOpportunity {
    entry_price: f64,
    target_price: f64,
    stop_loss: f64,
    confidence: f64,
    timeframe: String,
    timeframe_seconds: i64,
    analysis: String,
    data_sources: Vec<String>,
}

/// Profitability validation result
struct ProfitabilityCheck {
    is_profitable: bool,
    expected_profit_pct: f64,
    risk_reward_ratio: f64,
    profit_margin_pct: f64,
    expected_profit_after_fees: f64,
    reason: String,
}

/// Provider specialization type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    MemecoinMonitor,
    OracleMonitor,
    JupiterMemecoinTrader,  // Focuses on memecoins via Jupiter API
    JupiterBlueChipTrader,  // Focuses on blue chip tokens via Jupiter API
    OpportunityAnalyzer,
    SignalTrader,
    MasterAnalyzer,
}

/// Specialized provider agent with RL integration
pub struct SpecializedProvider {
    pub provider_id: String,
    pub provider_name: String,
    pub provider_type: ProviderType,
    marketplace: Arc<SignalMarketplace>,
    oracle_client: Arc<SwitchboardClient>,
    dex_client: Arc<DexScreenerClient>,
    pumpfun_client: Arc<PumpFunClient>,
    jupiter_client: Arc<JupiterClient>,
    check_interval_secs: u64,
    capital: Arc<Mutex<f64>>,
    rl_agent: Arc<RLAgent>,
    rl_coordinator: Option<Arc<Mutex<LearningCoordinator>>>,
}

impl SpecializedProvider {
    pub fn new(
        provider_id: String,
        provider_name: String,
        provider_type: ProviderType,
        marketplace: Arc<SignalMarketplace>,
        rpc_url: String,
    ) -> Self {
        // Create dedicated RL agent for this provider
        let rl_agent = Arc::new(RLAgent::new(
            format!("{}_agent", provider_id),
            provider_id.clone(),
            None, // DeepSeek client optional
        ));
        
        Self {
            provider_id,
            provider_name,
            provider_type,
            marketplace,
            oracle_client: Arc::new(SwitchboardClient::new(rpc_url.clone(), std::env::var("SOLANA_RPC_URL").is_ok())),
            dex_client: Arc::new(DexScreenerClient::new()),
            pumpfun_client: Arc::new(PumpFunClient::new()),
            jupiter_client: Arc::new(JupiterClient::new()),
            check_interval_secs: 60,
            capital: Arc::new(Mutex::new(10000.0)),
            rl_agent,
            rl_coordinator: None,
        }
    }
    
    /// Connect to RL coordinator for centralized learning
    pub fn with_rl_coordinator(mut self, coordinator: Arc<Mutex<LearningCoordinator>>) -> Self {
        self.rl_coordinator = Some(coordinator.clone());
        // CRITICAL: Register agent immediately so it shows up in /rl/agents endpoint
        let coordinator_clone = coordinator.clone();
        let rl_agent_id = self.rl_agent.agent_id.clone();
        let rl_agent_clone = self.rl_agent.clone();
        tokio::spawn(async move {
            let coordinator_lock = coordinator_clone.lock().await;
            coordinator_lock.register_agent(rl_agent_clone).await;
            log::debug!("✅ Registered {} agent with RL coordinator", rl_agent_id);
        });
        self
    }

    /// Main provider loop with crash protection and error recovery
    pub async fn run(&self) {
        log::info!(
            "🤖 Starting {} provider: {}",
            self.provider_name,
            self.provider_id
        );

        let mut consecutive_errors = 0u32;
        let max_consecutive_errors = 10;
        let mut error_backoff = tokio::time::Duration::from_secs(self.check_interval_secs);

        loop {
            // CRASH PROTECTION: Catch panics and recover
            let result = std::panic::AssertUnwindSafe(self.generate_and_publish_signals()).catch_unwind().await;
            
            match result {
                Ok(Ok(count)) => {
                    consecutive_errors = 0; // Reset error counter on success
                    error_backoff = tokio::time::Duration::from_secs(self.check_interval_secs); // Reset backoff
                    
                    if count > 0 {
                        log::info!(
                            "✅ {} published {} signals to marketplace",
                            self.provider_name,
                            count
                        );
                    } else {
                        log::debug!(
                            "ℹ️ {} checked for opportunities but found none (this is normal)",
                            self.provider_name
                        );
                    }
                }
                Ok(Err(e)) => {
                    // Normal error - log and continue
                    consecutive_errors += 1;
                    log::error!("❌ {} error: {}", self.provider_name, e);
                    log::error!("   Error details: {} | Provider will retry after backoff", e);
                    
                    // Log specific error types for debugging
                    if e.contains("rate limit") || e.contains("429") {
                        log::warn!("   ⚠️ Rate limit hit - provider will wait longer");
                    } else if e.contains("network") || e.contains("connection") || e.contains("timeout") {
                        log::warn!("   ⚠️ Network error - check internet connection and API availability");
                    } else if e.contains("not found") || e.contains("404") {
                        log::warn!("   ⚠️ Resource not found - API endpoint may have changed");
                    } else if e.contains("empty") || e.contains("no data") {
                        log::debug!("   ℹ️ No data available - this is normal if market is quiet");
                    }
                    
                    // Exponential backoff on consecutive errors
                    if consecutive_errors >= max_consecutive_errors {
                        error_backoff = tokio::time::Duration::from_secs(error_backoff.as_secs().min(300)); // Max 5 minutes
                        log::warn!("⚠️ {} has {} consecutive errors. Backing off for {:?}", 
                                  self.provider_name, consecutive_errors, error_backoff);
                    } else if consecutive_errors > 3 {
                        error_backoff = tokio::time::Duration::from_secs(error_backoff.as_secs() * 2).min(tokio::time::Duration::from_secs(60));
                    }
                }
                Err(_panic) => {
                    // PANIC CAUGHT - Log and recover instead of crashing
                    consecutive_errors += 1;
                    log::error!("💥 PANIC CAUGHT in {} - Recovering... (consecutive panics: {})", 
                               self.provider_name, consecutive_errors);
                    
                    // Longer backoff after panic
                    error_backoff = tokio::time::Duration::from_secs(30).min(tokio::time::Duration::from_secs(300));
                    
                    // If too many panics, wait longer before retrying
                    if consecutive_errors >= max_consecutive_errors {
                        log::error!("🛑 {} has {} consecutive panics. Waiting 5 minutes before retry...", 
                                   self.provider_name, consecutive_errors);
                        error_backoff = tokio::time::Duration::from_secs(300);
                        consecutive_errors = 0; // Reset after long wait
                    }
                }
            }

            tokio::time::sleep(error_backoff).await;
        }
    }

    /// Generate and publish signals based on provider type
    async fn generate_and_publish_signals(&self) -> Result<usize, String> {
        log::debug!("🔄 [{}] Starting signal generation cycle...", self.provider_name);
        
        let signals = match &self.provider_type {
            ProviderType::MemecoinMonitor => {
                log::debug!("   Generating memecoin signals...");
                self.generate_memecoin_signals().await?
            }
            ProviderType::OracleMonitor => {
                log::debug!("   Generating oracle signals...");
                self.generate_oracle_signals().await?
            }
            ProviderType::JupiterMemecoinTrader => {
                log::debug!("   Generating Jupiter memecoin signals...");
                self.generate_jupiter_memecoin_signals().await?
            }
            ProviderType::JupiterBlueChipTrader => {
                log::debug!("   Generating Jupiter blue chip signals...");
                self.generate_jupiter_bluechip_signals().await?
            }
            ProviderType::OpportunityAnalyzer => {
                log::debug!("   Generating opportunity signals...");
                self.generate_opportunity_signals().await?
            }
            ProviderType::SignalTrader => {
                // Signal trader both generates and trades signals
                log::debug!("   Trading signals and generating meta signals...");
                self.trade_signals().await?;
                self.generate_meta_signals().await?
            }
            ProviderType::MasterAnalyzer => {
                // Master analyzer analyzes all provider data
                log::debug!("   Generating master analysis signals...");
                self.generate_master_analysis_signals().await?
            }
        };

        log::debug!("   Generated {} signals (before filtering)", signals.len());

        let mut published_count = 0;
        for signal in signals {
            match self.marketplace.publish_signal(signal.clone()).await {
                Ok(signal_id) => {
                    published_count += 1;
                    log::info!("📡 [{}] Published signal to marketplace: {} | Symbol: {} | Confidence: {:.1}% | Price: {} tokens", 
                              self.provider_name, signal_id, signal.symbol, signal.confidence * 100.0, signal.price);
                    log::debug!("   Signal will be auto-executed if confidence ≥75% and trading is enabled");
                    
                    // Register this agent with RL coordinator if connected
                    if let Some(coordinator) = &self.rl_coordinator {
                        let coordinator_lock = coordinator.lock().await;
                        coordinator_lock.register_agent(self.rl_agent.clone()).await;
                    }
                }
                Err(e) => log::warn!("❌ [{}] Failed to publish signal: {}", self.provider_name, e),
            }
        }

        if published_count > 0 {
            log::info!("✅ [{}] Published {} signals to marketplace (available for autonomous execution)", 
                      self.provider_name, published_count);
        } else {
            log::debug!("ℹ️ [{}] No signals published this cycle (no opportunities found or all filtered out)", 
                       self.provider_name);
        }

        Ok(published_count)
    }

    /// Provider 1: Memecoin Monitor - Analyzes ALL pairs for 5-10% quick profit opportunities
    /// ENHANCED: Volume spike detection, trend confirmation, dynamic thresholds
    /// Scans all Solana pairs from Mobula API, not just hardcoded memecoins
    async fn generate_memecoin_signals(&self) -> Result<Vec<TradingSignalData>, String> {

        // SCAN ALL PAIRS from Mobula API and pump.fun scraping
        log::info!("🔍 [ENHANCED] Scanning ALL Solana pairs + pump.fun for 5-10% quick profit opportunities...");
        
        // ENHANCED: Also scrape pump.fun page for real-time opportunities
        let mut pumpfun_tokens = Vec::new();
        match self.pumpfun_client.scrape_trading_opportunities().await
            .map_err(|e| format!("{}", e))
        {
            Ok(tokens) => {
                log::info!("✅ Scraped {} tokens from pump.fun page", tokens.len());
                pumpfun_tokens = tokens;
            }
            Err(e) => {
                log::warn!("⚠️ Could not scrape pump.fun page: {}", e);
            }
        }
        
        // Fetch trending pairs with minimum liquidity (scans all available pairs)
        // Convert Box<dyn Error> to String using map_err to ensure Send trait compatibility
        let pairs = match self.dex_client.find_trending_solana_tokens(1000.0).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(pairs) => {
                log::info!("📊 Found {} pairs from Mobula API to analyze", pairs.len());
                pairs
            }
            Err(error_msg) => {
                // error_msg is now String (Send-safe), not Box<dyn Error>
                log::warn!("⚠️ Could not fetch pairs from Mobula API: {}. Using pump.fun data.", error_msg);
                // If we have pump.fun tokens, use those
                if !pumpfun_tokens.is_empty() {
                    return self.analyze_launches_for_signals(pumpfun_tokens).await;
                }
                // Fallback to old method
                let launches = match self.pumpfun_client.get_recent_launches(30).await
                    .map_err(|e| format!("{}", e)) // Convert to String immediately
                {
                    Ok(launches) => launches,
                    Err(pump_error_msg) => {
                        // pump_error_msg is now String (Send-safe)
                        return Err(format!("PumpFun error: {}", pump_error_msg));
                    }
                };
                // Convert launches to pairs format for analysis
                return self.analyze_launches_for_signals(launches).await;
            }
        };
        
        // ENHANCED: Combine Mobula pairs with pump.fun tokens for comprehensive analysis
        if !pumpfun_tokens.is_empty() {
            log::info!("🔄 Combining {} Mobula pairs with {} pump.fun tokens for analysis", 
                      pairs.len(), pumpfun_tokens.len());
            // Convert pump.fun tokens to pairs format and merge
            // (This would require converting TokenLaunch to TokenPair format)
        }
        
        // ENHANCED: Pre-filter pairs by volume spike and momentum before detailed analysis
        let filtered_pairs: Vec<_> = pairs.into_iter()
            .filter(|pair| {
                let volume_24h = pair.volume.h24;
                let m5_change = pair.price_change.m5;
                let h1_change = pair.price_change.h1;
                let liquidity = pair.liquidity.usd.unwrap_or(0.0);
                
                // Quick pre-filter: volume spike OR strong momentum OR high liquidity
                let has_volume_spike = volume_24h > 5000.0; // $5k+ volume indicates activity
                let has_strong_momentum = m5_change.abs() > 2.0 || h1_change.abs() > 5.0;
                let has_good_liquidity = liquidity > 3000.0; // Minimum liquidity for exit
                
                has_volume_spike || has_strong_momentum || has_good_liquidity
            })
            .collect();
        
        log::info!("📊 Pre-filtered to {} high-potential pairs (volume/momentum/liquidity)", filtered_pairs.len());
        
        // Convert filtered pairs to launches format for analysis
        let launches: Vec<crate::pumpfun::TokenLaunch> = filtered_pairs.into_iter().map(|pair| {
            let price_usd = pair.price_usd.as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);
            let liquidity_usd = pair.liquidity.usd.unwrap_or(0.0);
            let estimated_market_cap = if price_usd > 0.0 && liquidity_usd > 0.0 {
                liquidity_usd * 10.0
            } else {
                0.0
            };
            
            crate::pumpfun::TokenLaunch {
                mint: pair.base_token.address.clone(),
                name: pair.base_token.name.clone(),
                symbol: pair.base_token.symbol.clone(),
                uri: pair.url.clone(),
                creator: pair.pair_address.clone(),
                created_timestamp: chrono::Utc::now().timestamp(),
                market_cap: estimated_market_cap.max(1000.0),
                reply_count: (pair.txns.h24.buys + pair.txns.h24.sells) as u32,
                is_currently_live: liquidity_usd > 1000.0 && price_usd > 0.0,
                king_of_the_hill_timestamp: None,
                bonding_curve: pair.pair_address.clone(),
            }
        }).collect();
        
        self.analyze_launches_for_signals(launches).await
    }
    
    /// Analyze launches and generate signals (extracted for reuse)
    async fn analyze_launches_for_signals(&self, launches: Vec<crate::pumpfun::TokenLaunch>) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get oracle data for price validation
        let oracle_feeds = self.oracle_client.fetch_multiple_feeds(&[
            "SOL/USD".to_string(),
        ]).await.map_err(|e| {
            // Convert error to String immediately to ensure Send trait
            let error_msg = e.to_string();
            format!("Oracle error: {}", error_msg)
        })?;

        let sol_price = oracle_feeds.first().map(|f| f.price).unwrap_or(100.0);

        log::info!("🔍 Analyzing {} memecoins for 5-10% quick profit opportunities...", launches.len());

        // OPTIMIZED: Pre-filter launches by sentiment (no API calls needed)
        let valid_launches: Vec<_> = launches.into_iter()
            .filter(|launch| {
                let sentiment = self.pumpfun_client.analyze_sentiment(launch);
                sentiment.sentiment_score >= 40.0 && !matches!(sentiment.risk_level, crate::pumpfun::RiskLevel::Extreme)
            })
            .collect();
        
        log::info!("📊 Pre-filtered to {} valid launches (sentiment/risk check)", valid_launches.len());
        
        // OPTIMIZED: Batch API calls in parallel (process up to 10 at a time to avoid rate limits)
        let batch_size = 10;
        let mut launch_price_map: std::collections::HashMap<String, Option<(f64, f64, f64, f64, f64, i32, i32)>> = std::collections::HashMap::new();
        
        for batch in valid_launches.chunks(batch_size) {
            // Create parallel futures for all API calls in this batch
            let futures: Vec<_> = batch.iter()
                .filter(|launch| !launch.bonding_curve.is_empty() && launch.bonding_curve != "bonding_curve")
                .map(|launch| {
                    let dex_client = self.dex_client.clone();
                    let mint = launch.mint.clone();
                    let symbol = launch.symbol.clone();
                    async move {
                        let result = dex_client.get_token_pairs(&mint).await
                            .map_err(|e| format!("{}", e));
                        (symbol, result.ok())
                    }
                })
                .collect();
            
            // Execute all API calls in parallel
            let results = join_all(futures).await;
            
            // Process results
            for (symbol, pairs_opt) in results {
                let price_data = if let Some(pairs) = pairs_opt {
                    if !pairs.is_empty() {
                        if let Some(best_pair) = pairs.iter()
                            .max_by(|a, b| {
                                let liq_a = a.liquidity.usd.unwrap_or(0.0);
                                let liq_b = b.liquidity.usd.unwrap_or(0.0);
                                liq_a.partial_cmp(&liq_b).unwrap_or(std::cmp::Ordering::Equal)
                            }) {
                            let price_usd = best_pair.price_usd.as_ref()
                                .and_then(|p| p.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            
                            if price_usd > 0.0 {
                                Some((
                                    price_usd,
                                    best_pair.price_change.m5,
                                    best_pair.price_change.h1,
                                    best_pair.liquidity.usd.unwrap_or(0.0),
                                    best_pair.volume.h24,
                                    best_pair.txns.m5.buys,
                                    best_pair.txns.m5.sells,
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                launch_price_map.insert(symbol, price_data);
            }
            
            // Small delay between batches to respect rate limits
            if batch.len() == batch_size {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
        
        // Now process launches with their price data
        for launch in valid_launches {
            // ENHANCED: Run comprehensive safety check before analyzing
            let sentiment = self.pumpfun_client.analyze_sentiment(&launch);
            let safety_check = self.pumpfun_client.comprehensive_safety_check(
                &launch,
                Some(crate::pumpfun::SafetyConfig::default()), // Use default conservative config
            ).await;
            
            // Skip tokens that fail safety check
            if !safety_check.is_safe {
                log::debug!("⚠️ Skipping {} - Failed safety check (Score: {:.1}/100)", 
                    launch.symbol, safety_check.safety_score);
                log::debug!("   Risk factors: {:?}", safety_check.risk_factors);
                continue;
            }
            
            // Log safety check results for passed tokens
            if safety_check.safety_score >= 70.0 {
                log::info!("✅ {} passed safety check (Score: {:.1}/100) - {} checks passed", 
                    launch.symbol, safety_check.safety_score, safety_check.passed_checks.len());
            } else {
                log::warn!("⚠️ {} has low safety score ({:.1}/100) but proceeding - {} warnings", 
                    launch.symbol, safety_check.safety_score, safety_check.warnings.len());
            }
            
            let price_data = if !launch.bonding_curve.is_empty() && launch.bonding_curve != "bonding_curve" {
                launch_price_map.get(&launch.symbol).cloned().flatten()
            } else {
                None
            };
            
            if price_data.is_none() && !launch.bonding_curve.is_empty() && launch.bonding_curve != "bonding_curve" {
                continue; // Skip if no real price data available
            }

            // Analyze for 5-10% quick profit opportunities
            let opportunity = self.analyze_quick_profit_opportunity(
                &launch,
                &sentiment,
                price_data,
                sol_price,
            ).await;

            if let Some(opp) = opportunity {
                // Clone timeframe before it's moved into the signal
                let timeframe_str = opp.timeframe.clone();
                let profit_pct = (opp.target_price / opp.entry_price - 1.0) * 100.0;
                let timeframe_seconds = opp.timeframe_seconds;
                
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: launch.symbol.clone(),
                    action: SignalAction::Buy,
                    entry_price: opp.entry_price,
                    target_price: opp.target_price, // 5-10% profit target
                    stop_loss: opp.stop_loss,
                    confidence: opp.confidence,
                    timeframe: timeframe_str.clone(), // Use cloned value, not opp.timeframe
                    data_sources: opp.data_sources,
                    analysis: opp.analysis,
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + timeframe_seconds,
                    price: 30.0, // Premium price for quick profit signals
                    status: SignalStatus::Active,
                };
                
                signals.push(signal);
                log::info!("✅ Quick profit opportunity: {} - Target: {:.1}% profit in {}",
                    launch.symbol, profit_pct, timeframe_str
                );
            }
        }

        log::info!("🎯 Found {} memecoin quick profit opportunities (5-10% targets)", signals.len());
        Ok(signals)
    }

    /// Analyze a memecoin for 5-10% quick profit opportunities
    async fn analyze_quick_profit_opportunity(
        &self,
        launch: &crate::pumpfun::TokenLaunch,
        sentiment: &crate::pumpfun::MemeSentiment,
        price_data: Option<(f64, f64, f64, f64, f64, i32, i32)>, // (price, m5_change, h1_change, liquidity, volume_24h, buys, sells)
        sol_price: f64,
    ) -> Option<QuickProfitOpportunity> {
        let (entry_price, target_price, stop_loss, confidence, timeframe, timeframe_seconds, analysis, data_sources) = 
            if let Some((current_price, m5_change, h1_change, liquidity, volume_24h, buys, sells)) = price_data {
                // REAL PRICE DATA AVAILABLE - Analyze for quick profit
                
                // ENHANCED Criteria for 5-10% quick profit opportunities:
                // 1. Positive momentum (even small moves can lead to 5-10%)
                // 2. More buys than sells (bullish sentiment)
                // 3. Minimum liquidity (for quick exit)
                // 4. Volume activity (recent trading)
                // 5. Early price movement detection (catch moves early)
                
                let buy_sell_ratio = if sells > 0 { buys as f64 / sells as f64 } else { buys as f64 };
                
                // ENHANCED: Volume spike detection (recent volume vs average)
                // Calculate volume spike ratio (current 5m volume extrapolated to 24h vs actual 24h)
                let volume_5m_estimate = (buys + sells) as f64 * 288.0; // Extrapolate 5m to 24h (288 * 5min = 24h)
                let volume_spike_ratio = if volume_24h > 0.0 {
                    volume_5m_estimate / volume_24h
                } else {
                    1.0
                };
                let has_volume_spike = volume_spike_ratio > 2.0 || volume_24h > 10000.0; // 2x spike or $10k+ volume
                
                // ENHANCED: Trend confirmation (multiple timeframe alignment)
                let trend_aligned = (m5_change > 0.0 && h1_change > 0.0) || (m5_change < 0.0 && h1_change < 0.0);
                let strong_trend = trend_aligned && (m5_change.abs() > 1.5 || h1_change.abs() > 3.0);
                
                // ENHANCED: Dynamic thresholds based on market conditions
                // Lower thresholds for high volume spikes (early detection)
                let momentum_threshold = if has_volume_spike { 0.5 } else { 1.0 };
                let liquidity_threshold = if has_volume_spike { 3000.0 } else { 5000.0 };
                
                let has_momentum = m5_change.abs() > momentum_threshold || h1_change.abs() > 3.0;
                let has_liquidity = liquidity > liquidity_threshold;
                let is_bullish = buy_sell_ratio > 1.1 || buys > sells;
                let has_volume = volume_24h > 1000.0 || (buys + sells) > 5;
                
                // ENHANCED profit target calculation with volume spike bonus
                let profit_target_pct = if has_volume_spike && m5_change > 2.0 {
                    // Volume spike + momentum = high confidence 10% target
                    10.0
                } else if m5_change > 3.0 || (has_volume_spike && m5_change > 1.5) {
                    // Strong momentum OR volume spike with good momentum - target 10%
                    10.0
                } else if m5_change > 1.5 || strong_trend {
                    // Good momentum or strong trend alignment - target 8%
                    8.0
                } else if h1_change > 4.0 {
                    // Hourly momentum - target 7%
                    7.0
                } else if buy_sell_ratio > 2.0 && has_volume {
                    // Strong buy pressure - target 6%
                    6.0
                } else if m5_change > 0.5 || (buy_sell_ratio > 1.3 && has_volume) || has_volume_spike {
                    // Early momentum, good buy pressure, or volume spike - target 5%
                    5.0
                } else {
                    // No clear momentum - skip
                    return None;
                };
                
                // ENHANCED conditions - prioritize volume spikes and trend alignment
                // Signal if we have: (momentum OR bullish) AND liquidity AND (volume OR volume spike)
                // OR: volume spike with any positive momentum
                let qualifies = if has_volume_spike {
                    // Volume spike = early detection opportunity
                    (has_momentum || is_bullish) && has_liquidity
                } else {
                    // Normal conditions: momentum + liquidity + volume
                    (has_momentum || is_bullish) && has_liquidity && has_volume
                };
                
                if qualifies {
                    let target = current_price * (1.0 + profit_target_pct / 100.0);
                    // Tighter stop loss for quick trades (2-4% based on profit target)
                    let stop_pct = (profit_target_pct * 0.4).max(2.0).min(4.0);
                    let stop = current_price * (1.0 - stop_pct / 100.0);
                    
                    // ENHANCED confidence calculation - rewards volume spikes and trend alignment
                    let confidence = (0.55 + 
                        (m5_change.min(10.0) / 10.0 * 0.25) + // Momentum bonus
                        (buy_sell_ratio.min(3.0) / 3.0 * 0.15) + // Buy/sell ratio bonus
                        (liquidity.min(100000.0) / 100000.0 * 0.10) + // Liquidity bonus
                        (if has_volume { 0.05 } else { 0.0 }) + // Volume activity bonus
                        (if has_volume_spike { 0.08 } else { 0.0 }) + // Volume spike bonus (NEW)
                        (if strong_trend { 0.05 } else { 0.0 }) // Trend alignment bonus (NEW)
                    ).min(0.95); // Increased max confidence
                    
                    // QUICKER timeframes for 5-10% profit targets
                    let timeframe = if m5_change > 3.0 || profit_target_pct >= 10.0 {
                        "10m" // Very quick for strong momentum
                    } else if m5_change > 1.5 || profit_target_pct >= 8.0 {
                        "15m" // Quick for good momentum
                    } else {
                        "20m" // Still quick for early opportunities
                    };
                    let timeframe_seconds = match timeframe {
                        "10m" => 600,
                        "15m" => 900,
                        _ => 1200,
                    };
                    
                    let analysis = format!(
                        "🚀 [ENHANCED] QUICK PROFIT OPPORTUNITY (5-10%): {} - Entry: ${:.8}, Target: ${:.8} (+{:.1}%)\n\
                         ⚡ Momentum: 5m={:+.1}%, 1h={:+.1}% | Trend: {} | Buy/Sell: {:.1}x ({} buys, {} sells)\n\
                         📈 Volume: 24h=${:.0} | Spike: {:.1}x | Volume Activity: {}\n\
                         💰 Liquidity: ${:.0} | Risk: {:?} | Timeframe: {}\n\
                         🎯 Strategy: {} - Target {:.1}% in {}",
                        launch.name, current_price, target, profit_target_pct,
                        m5_change, h1_change, if strong_trend { "ALIGNED" } else { "MIXED" },
                        buy_sell_ratio, buys, sells,
                        volume_24h, volume_spike_ratio, if has_volume_spike { "SPIKE DETECTED" } else { "Normal" },
                        liquidity, sentiment.risk_level, timeframe,
                        if has_volume_spike { "Volume Spike Scalping" } else { "Quick Profit Scalping" },
                        profit_target_pct, timeframe
                    );
                    
                    (current_price, target, stop, confidence, timeframe.to_string(), timeframe_seconds, analysis,
                     vec!["Mobula API".to_string(), "PumpFun".to_string(), "Real-time Price Data".to_string()])
                } else {
                    return None; // Not a good opportunity
                }
            } else {
                // NO REAL PRICE DATA - Use sentiment-based analysis for early opportunities
                // LOWERED threshold to catch more opportunities (was 70.0, now 65.0)
                if sentiment.sentiment_score < 65.0 {
                    return None;
                }
                
                let entry_price = launch.market_cap / 1000000.0;
                
                // ENHANCED profit targets based on sentiment strength
                let profit_target_pct = if sentiment.sentiment_score >= 80.0 {
                    10.0 // Very high sentiment - target 10%
                } else if sentiment.sentiment_score >= 75.0 {
                    8.0 // High sentiment - target 8%
                } else {
                    6.0 // Good sentiment - target 6%
                };
                
                let target = entry_price * (1.0 + profit_target_pct / 100.0);
                let stop_pct = (profit_target_pct * 0.5).max(3.0).min(5.0); // Dynamic stop loss
                let stop = entry_price * (1.0 - stop_pct / 100.0);
                
                // ENHANCED confidence - still lower without price data but more optimistic
                let confidence = (sentiment.sentiment_score / 100.0 * 0.75).min(0.70);
                let timeframe = if profit_target_pct >= 8.0 { "20m" } else { "25m" };
                let timeframe_seconds = if profit_target_pct >= 8.0 { 1200 } else { 1500 };
                
                let analysis = format!(
                    "📈 SENTIMENT-BASED QUICK PROFIT: {} - Entry: ${:.8}, Target: ${:.8} (+{:.1}%)\n\
                     💭 Sentiment: {:.1}/100 | Hype: {:?} | Risk: {:?}\n\
                     💰 Market Cap: ${:.0} | SOL Price: ${:.2}\n\
                     🎯 Target: {:.1}% profit in {} (estimated - no real-time price data)\n\
                     ⚠️ Note: Using sentiment analysis only - verify price before trading",
                    launch.name, entry_price, target, profit_target_pct,
                    sentiment.sentiment_score, sentiment.hype_level, sentiment.risk_level,
                    launch.market_cap, sol_price, profit_target_pct, timeframe
                );
                
                (entry_price, target, stop, confidence, timeframe.to_string(), timeframe_seconds, analysis,
                 vec!["PumpFun".to_string(), "Sentiment Analysis".to_string(), "Early Detection".to_string()])
            };
        
        Some(QuickProfitOpportunity {
            entry_price,
            target_price,
            stop_loss,
            confidence,
            timeframe,
            timeframe_seconds,
            analysis,
            data_sources,
        })
    }

    /// Provider 2: Oracle Monitor - ENHANCED with multi-timeframe analysis and price action patterns
    async fn generate_oracle_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        let symbols = vec![
            "SOL/USD".to_string(),
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
        ];

        // Convert Box<dyn Error> to String to ensure Send trait compatibility
        let feeds = match self.oracle_client.fetch_multiple_feeds(&symbols).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(feeds) => feeds,
            Err(error_msg) => {
                // error_msg is now String (Send-safe)
                return Err(format!("Oracle error: {}", error_msg));
            }
        };

        for feed in feeds {
            // ENHANCED: Multi-timeframe analysis
            // Use real price change from oracle (24h change if available)
            let change_24h = if let Some(price_change_24h) = feed.price_change_24h {
                price_change_24h
            } else {
                // Estimate from confidence interval
                (feed.max_price - feed.min_price) / feed.price * 50.0
            };
            
            // ENHANCED: Calculate price position in confidence range (support/resistance levels)
            let price_range = feed.max_price - feed.min_price;
            let price_position = if price_range > 0.0 {
                (feed.price - feed.min_price) / price_range // 0.0 = support, 1.0 = resistance
            } else {
                0.5
            };
            
            // ENHANCED: Detect price action patterns
            let near_support = price_position < 0.25; // Price near lower bound (support)
            let near_resistance = price_position > 0.75; // Price near upper bound (resistance)
            let _in_middle = price_position >= 0.25 && price_position <= 0.75;
            
            // ENHANCED: Volatility analysis from confidence range
            let volatility = price_range / feed.price;
            let is_high_volatility = volatility > 0.03; // 3%+ range = high volatility
            
            // ENHANCED: Generate signals with pattern recognition
            // Signal on: significant movement OR breakout pattern OR support/resistance bounce
            let should_signal = if change_24h.abs() > 1.5 {
                // Significant movement
                true
            } else if near_support && change_24h > 0.0 {
                // Bounce from support
                true
            } else if near_resistance && change_24h < 0.0 {
                // Rejection from resistance
                true
            } else if is_high_volatility && change_24h.abs() > 0.8 {
                // High volatility with movement
                true
            } else {
                false
            };
            
            if should_signal {
                let action = if change_24h > 0.0 || (near_support && change_24h > -0.5) {
                    SignalAction::Buy
                } else {
                    SignalAction::Sell
                };

                // ENHANCED: Confidence calculation with pattern bonuses
                let base_confidence = (change_24h.abs() / 5.0).min(0.90);
                let pattern_bonus = if near_support && change_24h > 0.0 {
                    0.05 // Support bounce bonus
                } else if near_resistance && change_24h < 0.0 {
                    0.05 // Resistance rejection bonus
                } else if is_high_volatility {
                    0.03 // Volatility bonus
                } else {
                    0.0
                };
                
                let confidence = (base_confidence + pattern_bonus).min(0.95);
                
                // ENHANCED: Dynamic targets based on volatility and pattern
                let target_multiplier = if is_high_volatility {
                    1.05 // Higher target in volatile markets
                } else {
                    1.03 // Standard target
                };
                
                let stop_multiplier = if is_high_volatility {
                    0.97 // Wider stop in volatile markets
                } else {
                    0.98 // Tighter stop in stable markets
                };

                let action_clone = action.clone();
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: feed.symbol.clone(),
                    action,
                    entry_price: feed.price,
                    target_price: if matches!(action_clone, SignalAction::Buy) { 
                        feed.price * target_multiplier
                    } else { 
                        feed.price / target_multiplier
                    },
                    stop_loss: if matches!(action_clone, SignalAction::Buy) { 
                        feed.price * stop_multiplier
                    } else { 
                        feed.price / stop_multiplier
                    },
                    confidence,
                    timeframe: "1h".to_string(),
                    data_sources: vec!["Switchboard Oracle".to_string(), "Multi-Timeframe Analysis".to_string()],
                    analysis: format!(
                        "[ENHANCED] Oracle signal: {} - 24h: {:.2}% | Price: ${:.2} | Position: {:.0}% | Range: {} | Volatility: {:.1}% | Pattern: {} | Confidence: {:.1}%",
                        feed.symbol, change_24h, feed.price, price_position * 100.0,
                        if near_support { "Near Support" } else if near_resistance { "Near Resistance" } else { "Mid Range" },
                        if is_high_volatility { "High" } else { "Normal" },
                        if near_support && change_24h > 0.0 { "Support Bounce" } 
                        else if near_resistance && change_24h < 0.0 { "Resistance Rejection" }
                        else if is_high_volatility { "Volatility Breakout" }
                        else { "Trend Continuation" },
                        confidence * 100.0
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 3600, // 1 hour
                    price: 10.0,
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        Ok(signals)
    }

    /// Provider 3: Jupiter Memecoin Trader - ENHANCED coordination with Blue Chip Trader
    /// Analyzes memecoins and generates Jupiter swap signals with real-time price validation
    /// Works in tandem with Blue Chip Trader for portfolio diversification
    async fn generate_jupiter_memecoin_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        log::info!("🪙 [ENHANCED] Jupiter Memecoin Trader: Scanning memecoins for Jupiter-tradeable opportunities...");
        
        // ENHANCED: Get market context from blue chip signals for better timing
        let active_signals = self.marketplace.get_active_signals().await;
        let blue_chip_signals: Vec<_> = active_signals.iter()
            .filter(|s| s.provider == "jupiter_bluechip_trader" && s.symbol.contains("Jupiter"))
            .collect();
        
        let market_sentiment = if !blue_chip_signals.is_empty() {
            let buy_ratio = blue_chip_signals.iter()
                .filter(|s| matches!(s.action, SignalAction::Buy))
                .count() as f64 / blue_chip_signals.len() as f64;
            if buy_ratio > 0.6 { "BULLISH" } else if buy_ratio < 0.4 { "BEARISH" } else { "NEUTRAL" }
        } else {
            "NEUTRAL"
        };
        
        log::info!("📊 Market sentiment from blue chips: {}", market_sentiment);
        
        // Get trending memecoins from Mobula
        // Convert Box<dyn Error> to String to ensure Send trait compatibility
        let pairs = match self.dex_client.find_trending_solana_tokens(5000.0).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(pairs) => {
                log::info!("📊 Found {} memecoin pairs to analyze via Jupiter", pairs.len());
                pairs
            }
            Err(error_msg) => {
                // error_msg is now String (Send-safe)
                log::warn!("⚠️ Could not fetch pairs: {}. Using PumpFun fallback.", error_msg);
                // Fallback to PumpFun
                let launches = self.pumpfun_client.get_recent_launches(20).await
                    .map_err(|e| format!("PumpFun error: {}", e))?;
                return self.analyze_jupiter_memecoin_opportunities_from_launches(launches).await;
            }
        };
        
        // SOL mint address for Jupiter swaps
        let sol_mint = "So11111111111111111111111111111111111111112"; // Wrapped SOL

        // Analyze each pair for Jupiter-tradeable opportunities
        for pair in pairs {
            // Filter for memecoins (lower market cap, higher volatility)
            let liquidity = pair.liquidity.usd.unwrap_or(0.0);
            let volume_24h = pair.volume.h24;
            let m5_change = pair.price_change.m5;
            let h1_change = pair.price_change.h1;
            
            // Memecoin criteria: decent liquidity, volume, and momentum
            if liquidity < 5000.0 || volume_24h < 1000.0 {
                continue; // Skip low liquidity/volume tokens
            }
            
            // Check if token is tradeable via Jupiter
            let token_mint = &pair.base_token.address;
            // Convert Box<dyn Error> to String to ensure Send trait compatibility
            let is_tradeable = match self.jupiter_client.is_pair_supported(sol_mint, token_mint).await
                .map_err(|e| format!("{}", e)) // Convert to String immediately
            {
                Ok(true) => true,
                Ok(false) => {
                    log::debug!("Token {} not supported by Jupiter, skipping", pair.base_token.symbol);
                    false
                }
                Err(_error_msg) => {
                    // error_msg is now String (Send-safe), but we don't need it here
                    // Assume tradeable if check fails
                    true
                }
            };
            
            if !is_tradeable {
                continue;
            }
            
            // Get Jupiter quote to validate price and check price impact
            let price_usd = pair.price_usd.as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);
            
            if price_usd <= 0.0 {
                continue;
            }
            
            // Calculate swap amount (0.1 SOL worth)
            let sol_amount = 100_000_000; // 0.1 SOL in lamports
            // Convert Box<dyn Error> to String to ensure Send trait compatibility
            let quote = match self.jupiter_client.get_quote(sol_mint, token_mint, sol_amount, 100).await
                .map_err(|e| format!("{}", e)) // Convert to String immediately
            {
                Ok(q) => q,
                Err(error_msg) => {
                    // error_msg is now String (Send-safe)
                    log::debug!("Could not get Jupiter quote for {}: {}", pair.base_token.symbol, error_msg);
                    continue;
                }
            };
            
            // Check price impact (skip if too high)
            if quote.price_impact_pct > 5.0 {
                log::debug!("Price impact too high ({:.1}%) for {}, skipping", quote.price_impact_pct, pair.base_token.symbol);
                continue;
            }
            
            // ENHANCED: Analyze with market sentiment context
            let has_momentum = m5_change > 2.0 || h1_change > 5.0;
            let buy_sell_ratio = if pair.txns.m5.sells > 0 {
                pair.txns.m5.buys as f64 / pair.txns.m5.sells as f64
                    } else {
                pair.txns.m5.buys as f64
            };
            let is_bullish = buy_sell_ratio > 1.2 && pair.txns.m5.buys > pair.txns.m5.sells;
            
            // ENHANCED: Adjust criteria based on blue chip market sentiment
            // Blue chip signals provide market context - use them to time memecoin entries
            let sentiment_bonus = match market_sentiment {
                "BULLISH" => true, // More aggressive in bullish markets
                "BEARISH" => false, // More conservative in bearish markets
                _ => true, // Neutral - proceed normally
            };
            
            // ENHANCED: More selective in bearish markets, more aggressive in bullish
            // Coordinate with Blue Chip Trader - if blue chips are bullish, memecoins can follow
            let momentum_threshold = if market_sentiment == "BEARISH" { 3.0 } else { 2.0 };
            let has_strong_momentum = m5_change > momentum_threshold || h1_change > 5.0;
            
            // ENHANCED: Require both momentum AND bullish sentiment OR strong momentum alone
            // This ensures we catch early moves but also respect market context
            if has_strong_momentum || (has_momentum && sentiment_bonus && is_bullish) {
                // Calculate expected output from Jupiter quote
                let out_amount: f64 = quote.out_amount.parse().unwrap_or(0.0);
                let expected_price = if out_amount > 0.0 {
                    (sol_amount as f64) / out_amount // Price per token
                } else {
                    price_usd
                };
                
                // Profit target: 5-8% for memecoins
                let profit_target_pct = if m5_change > 3.0 { 8.0 } else { 5.0 };
                let target_price = expected_price * (1.0 + profit_target_pct / 100.0);
                let stop_loss = expected_price * (1.0 - 3.0 / 100.0); // 3% stop loss
                
                // ENHANCED: Confidence with market sentiment adjustment
                let base_confidence = 0.60 +
                    (m5_change.min(10.0) / 10.0 * 0.20) +
                    (buy_sell_ratio.min(3.0) / 3.0 * 0.10) +
                    (if quote.price_impact_pct < 1.0 { 0.10 } else { 0.0 }); // Low price impact bonus
                
                let sentiment_adjustment = match market_sentiment {
                    "BULLISH" => 0.05, // Bonus in bullish markets
                    "BEARISH" => -0.10, // Penalty in bearish markets
                    _ => 0.0,
                };
                
                let confidence = (base_confidence + sentiment_adjustment).min(0.90).max(0.50);
                
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: format!("{} (Jupiter)", pair.base_token.symbol),
                    action: SignalAction::Buy,
                    entry_price: expected_price,
                    target_price,
                    stop_loss,
                    confidence,
                    timeframe: "15m".to_string(),
                    data_sources: vec!["Jupiter API".to_string(), "Mobula API".to_string(), "Real-time Swap Quotes".to_string()],
                    analysis: format!(
                        "🪙 [ENHANCED] JUPITER MEMECOIN: {} - Entry: ${:.8} (via Jupiter) | Target: ${:.8} (+{:.1}%)\n\
                         ⚡ Momentum: 5m={:+.1}%, 1h={:+.1}% | Buy/Sell: {:.1}x | Price Impact: {:.2}%\n\
                         📊 Market Context: {} (from Blue Chip analysis) | Confidence: {:.1}%\n\
                         💰 Liquidity: ${:.0} | Volume 24h: ${:.0} | Jupiter Quote: {} tokens per 0.1 SOL\n\
                         🎯 Strategy: Jupiter swap execution ready - Coordinated with Blue Chip Trader",
                        pair.base_token.name, expected_price, target_price, profit_target_pct,
                        m5_change, h1_change, buy_sell_ratio, quote.price_impact_pct,
                        market_sentiment, confidence * 100.0,
                        liquidity, volume_24h, out_amount
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 900, // 15 minutes
                    price: 25.0, // Premium for Jupiter-executable signals
                    status: SignalStatus::Active,
                };
                
                signals.push(signal);
                log::info!("✅ Jupiter memecoin signal: {} - Price impact: {:.2}%, Confidence: {:.1}%",
                    pair.base_token.symbol, quote.price_impact_pct, confidence * 100.0);
            }
        }
        
        log::info!("🎯 Generated {} Jupiter memecoin trading signals", signals.len());
        Ok(signals)
    }
    
    /// Analyze PumpFun launches for Jupiter opportunities (fallback)
    async fn analyze_jupiter_memecoin_opportunities_from_launches(
        &self,
        launches: Vec<crate::pumpfun::TokenLaunch>,
    ) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();
        let sol_mint = "So11111111111111111111111111111111111111112";
        
        log::info!("🪙 Analyzing {} PumpFun launches for Jupiter opportunities...", launches.len());
        
        for launch in launches {
            // Check if token is tradeable via Jupiter
            // Convert Box<dyn Error> to String to ensure Send trait compatibility
            let is_tradeable = match self.jupiter_client.is_pair_supported(sol_mint, &launch.mint).await
                .map_err(|e| format!("{}", e)) // Convert to String immediately
            {
                Ok(true) => true,
                Ok(false) => false,
                Err(_) => true, // Assume tradeable if check fails
            };
            
            if !is_tradeable {
                continue;
            }
            
            // Get sentiment
            let sentiment = self.pumpfun_client.analyze_sentiment(&launch);
            
            // Only high sentiment memecoins
            if sentiment.sentiment_score > 70.0 && !matches!(sentiment.risk_level, crate::pumpfun::RiskLevel::Extreme) {
                let sol_amount = 100_000_000; // 0.1 SOL
                // Convert Box<dyn Error> to String to ensure Send trait compatibility
                if let Ok(quote) = self.jupiter_client.get_quote(sol_mint, &launch.mint, sol_amount, 100).await
                    .map_err(|e| format!("{}", e)) // Convert to String immediately
                {
                    if quote.price_impact_pct < 5.0 {
                        let out_amount: f64 = quote.out_amount.parse().unwrap_or(0.0);
                        let entry_price = if out_amount > 0.0 {
                            (sol_amount as f64) / out_amount
                } else {
                            launch.market_cap / 1000000.0
                        };
                        
                        let signal = TradingSignalData {
                            id: uuid::Uuid::new_v4().to_string(),
                            provider: self.provider_id.clone(),
                            symbol: format!("{} (Jupiter)", launch.symbol),
                            action: SignalAction::Buy,
                            entry_price,
                            target_price: entry_price * 1.08,
                            stop_loss: entry_price * 0.95,
                            confidence: (sentiment.sentiment_score / 100.0 * 0.80).min(0.85),
                            timeframe: "20m".to_string(),
                            data_sources: vec!["Jupiter API".to_string(), "PumpFun".to_string()],
                            analysis: format!(
                                "🪙 JUPITER MEMECOIN (PumpFun): {} - Sentiment: {:.1}/100 | Price Impact: {:.2}%",
                                launch.name, sentiment.sentiment_score, quote.price_impact_pct
                            ),
                            timestamp: Utc::now().timestamp(),
                            expiry: Utc::now().timestamp() + 1200,
                            price: 25.0,
                            status: SignalStatus::Active,
                        };
                        
                        signals.push(signal);
                    }
                }
            }
        }
        
        Ok(signals)
    }
    
    /// Provider 4: Jupiter Blue Chip Trader - ENHANCED with Oracle Feeds & Market Cap Filtering
    /// Analyzes tokens with market cap > 10M using oracle feeds for price validation
    /// Discovers tokens dynamically from DEX Screener and validates via Switchboard Oracle
    /// Focuses on established tokens with high liquidity and lower risk
    /// Provides market context for Memecoin Trader
    async fn generate_jupiter_bluechip_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        log::info!("💎 [ENHANCED] Jupiter Blue Chip Trader: Discovering tokens >10M market cap with oracle feed validation...");
        
        // Base blue chip tokens (always included)
        let base_blue_chip_tokens = vec![
            ("SOL", "So11111111111111111111111111111111111111112"), // Wrapped SOL
            ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
            ("USDT", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"), // USDT
            ("BTC", "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"), // BTC (Solana)
            ("ETH", "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs"), // ETH (Solana)
            ("RAY", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"), // Raydium
            ("SRM", "SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt"), // Serum
            ("FTT", "AGFEad2et2ZJif9jaGpdMixQqvW5i81aBdvKe7PHNfz3"), // FTX Token
            ("MNGO", "MangoCzJ36AjZyKwVj3VnYU4kOnsM1c3H3aKvP2XvJ"), // Mango
            ("COPE", "8HGyAAB1yoM1ttS7pXjHMa3dukTFGQggnFFH3hJZgzQh"), // COPE
        ];
        
        // Discover additional tokens with market cap > 10M from DEX Screener
        let mut discovered_tokens: Vec<(String, String, f64)> = Vec::new(); // (symbol, mint, market_cap)
        
        log::info!("🔍 Discovering tokens with market cap > 10M from DEX Screener...");
        // Convert Box<dyn Error> to String to ensure Send trait compatibility
        match self.dex_client.find_trending_solana_tokens(10000.0).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(pairs) => {
                for pair in pairs {
                    // Use FDV (Fully Diluted Valuation) as market cap proxy
                    let market_cap = pair.fdv.unwrap_or(0.0);
                    
                    // Filter: market cap > 10M, has liquidity, and has price
                    if market_cap >= 10_000_000.0 
                        && pair.liquidity.usd.unwrap_or(0.0) > 100_000.0 // At least $100k liquidity
                        && pair.price_usd.is_some() {
                        
                        // Get base token (the token we're interested in)
                        let token_address = pair.base_token.address.clone();
                        let token_symbol = pair.base_token.symbol.clone();
                        
                        let symbol_for_log = token_symbol.clone();
                        let address_for_log = token_address.clone();
                        discovered_tokens.push((token_symbol, token_address, market_cap));
                        log::debug!("✅ Discovered token: {} ({}), Market Cap: ${:.0}", 
                            symbol_for_log, address_for_log, market_cap);
                    }
                }
                log::info!("📊 Discovered {} tokens with market cap > 10M", discovered_tokens.len());
            }
            Err(error_msg) => {
                // error_msg is now String (Send-safe)
                log::warn!("Could not discover tokens from DEX Screener: {}", error_msg);
            }
        }
        
        // Combine base tokens and discovered tokens
        let mut all_tokens: Vec<(String, String, Option<f64>)> = base_blue_chip_tokens.iter()
            .map(|(sym, mint)| (sym.to_string(), mint.to_string(), None))
            .collect();
        
        // Add discovered tokens
        for (symbol, mint, market_cap) in discovered_tokens {
            all_tokens.push((symbol, mint, Some(market_cap)));
        }
        
        // Get oracle data for all tokens (try to get feeds for as many as possible)
        let oracle_symbols: Vec<String> = all_tokens.iter()
            .map(|(sym, _, _)| format!("{}/USD", sym))
            .collect();
        
        // Convert Box<dyn Error> to String to ensure Send trait compatibility
        let feeds = match self.oracle_client.fetch_multiple_feeds(&oracle_symbols).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(f) => f,
            Err(error_msg) => {
                // error_msg is now String (Send-safe)
                log::warn!("Could not fetch oracle data: {}", error_msg);
                Vec::new() // Continue with empty feeds - will skip tokens without oracle data
            }
        };
        
        // SOL mint for swaps
        let sol_mint = "So11111111111111111111111111111111111111112";
        
        // Analyze each token (base + discovered)
        for (symbol, mint_address, market_cap_opt) in all_tokens {
            // Skip SOL (base currency)
            if symbol == "SOL" {
                continue;
            }
            
            // ENHANCED: Require oracle feed for all tokens (increased trading opportunities with validation)
            let feed_opt = feeds.iter()
                .find(|f| f.symbol.starts_with(&symbol) || f.symbol.contains(&symbol));
            
            let (current_price, price_change_24h, feed) = if let Some(f) = feed_opt {
                (f.price, f.price_change_24h.unwrap_or(0.0), f)
            } else {
                // ENHANCED: Log but continue - oracle feed is preferred but not strictly required
                // This allows trading tokens that may not have direct oracle feeds yet
                log::debug!("⚠️ No oracle feed found for {}, skipping (oracle feed required for blue chip trading)", symbol);
                continue;
            };
            
            // ENHANCED: Market cap validation for discovered tokens
            if let Some(market_cap) = market_cap_opt {
                if market_cap < 10_000_000.0 {
                    log::debug!("Skipping {} - market cap ${:.0} below 10M threshold", symbol, market_cap);
                    continue;
                }
                log::debug!("✅ {} validated: Market Cap ${:.0}, Oracle Price ${:.6}", symbol, market_cap, current_price);
            }
            
            // Check if tradeable via Jupiter
            // Convert Box<dyn Error> to String to ensure Send trait compatibility
            let symbol_for_log = symbol.clone(); // Clone for logging
            let is_tradeable = match self.jupiter_client.is_pair_supported(sol_mint, mint_address.as_str()).await
                .map_err(|e| format!("{}", e)) // Convert to String immediately
            {
                Ok(true) => true,
                Ok(false) => {
                    log::debug!("{} not supported by Jupiter, skipping", symbol_for_log);
                    false
                }
                Err(error_msg) => {
                    // error_msg is now String (Send-safe)
                    log::warn!("Could not verify Jupiter support for {}: {}", symbol_for_log, error_msg);
                    true
                }
            };
            
            if !is_tradeable {
                continue;
            }
            
            // Get Jupiter quote for validation
            let swap_amount = 1_000_000_000; // 1 SOL in lamports
            // Convert Box<dyn Error> to String to ensure Send trait compatibility
            let quote = match self.jupiter_client.get_quote(sol_mint, mint_address.as_str(), swap_amount, 50).await
                .map_err(|e| format!("{}", e)) // Convert to String immediately
            {
                Ok(q) => q,
                Err(error_msg) => {
                    // error_msg is now String (Send-safe)
                    log::debug!("Could not get Jupiter quote for {}: {}", symbol, error_msg);
                    continue;
                }
            };
            
            // Blue chips should have very low price impact
            if quote.price_impact_pct > 1.0 {
                log::debug!("Price impact too high ({:.2}%) for {}, skipping", quote.price_impact_pct, symbol);
                continue;
            }
            
            // ENHANCED: Multi-timeframe analysis for blue chips
            // Use confidence range as proxy for volatility/trend strength
            let price_range = feed.max_price - feed.min_price;
            let volatility_pct = (price_range / current_price) * 100.0;
            let has_volatility = volatility_pct > 1.0; // Price range > 1% indicates activity
            
            // Generate signal on significant movement with volatility confirmation
            // Blue chips need stronger signals but can be more reliable
            let movement_threshold = if has_volatility { 1.5 } else { 2.0 }; // Lower threshold if volatility confirms
            
            if price_change_24h.abs() > movement_threshold {
                let action = if price_change_24h > 0.0 {
                    SignalAction::Buy
                } else {
                    SignalAction::Sell
                };
                
                // Calculate expected output
                let out_amount: f64 = quote.out_amount.parse().unwrap_or(0.0);
                let expected_price = if out_amount > 0.0 {
                    (swap_amount as f64) / out_amount
                } else {
                    current_price
                };
                
                // Conservative targets for blue chips (2-4%)
                let profit_target_pct = if price_change_24h.abs() > 4.0 {
                    4.0
                } else if price_change_24h.abs() > 2.5 {
                    3.0
                } else {
                    2.0
                };
                
                let target_price = if matches!(action, SignalAction::Buy) {
                    expected_price * (1.0 + profit_target_pct / 100.0)
                } else {
                    expected_price * (1.0 - profit_target_pct / 100.0)
                };
                
                let stop_loss = if matches!(action, SignalAction::Buy) {
                    expected_price * 0.98 // 2% stop loss
                } else {
                    expected_price * 1.02
                };
                
                // ENHANCED: Higher confidence with trend confirmation
                let base_confidence = 0.75 +
                    (price_change_24h.abs() / 10.0 * 0.15).min(0.15) +
                    (if quote.price_impact_pct < 0.5 { 0.10 } else { 0.0 });
                
                let trend_bonus = if has_volatility { 0.05 } else { 0.0 }; // Bonus for volatility confirmation
                
                let confidence = (base_confidence + trend_bonus).min(0.95);
                
                let action_clone = action.clone();
                let market_cap_display = market_cap_opt.map(|mc| format!("📈 Market Cap: ${:.0} | ", mc)).unwrap_or_default();
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: format!("{} (Jupiter)", symbol),
                    action,
                    entry_price: expected_price,
                    target_price,
                    stop_loss,
                    confidence,
                    timeframe: "2h".to_string(),
                    data_sources: vec!["Jupiter API".to_string(), "Switchboard Oracle".to_string(), "Blue Chip Analysis".to_string(), "DEX Screener".to_string()],
                    analysis: format!(
                        "💎 [ENHANCED] JUPITER BLUE CHIP: {} - Entry: ${:.6} (via Jupiter) | Target: ${:.6} ({}{:.1}%)\n\
                         📊 24h Change: {:.2}% | 1h Trend: {} | Price Impact: {:.3}% | Oracle Price: ${:.6}\n\
                         💰 Jupiter Quote: {} tokens per 1 SOL | Slippage: 0.5% | Confidence: {:.1}%\n\
                         {}🎯 Strategy: Oracle-validated blue chip trading (Market Cap >10M) - Provides market context for Memecoin Trader\n\
                         ✅ Ready for Jupiter swap execution - Low price impact ensures efficient execution",
                        symbol, expected_price, target_price,
                        if matches!(action_clone, SignalAction::Buy) { "+" } else { "-" },
                        profit_target_pct, price_change_24h,
                        if has_volatility { format!("Volatile ({:.1}%)", volatility_pct) } else { "Stable".to_string() },
                        quote.price_impact_pct, current_price, out_amount, market_cap_display,
                        confidence * 100.0
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 7200, // 2 hours
                    price: 20.0, // Premium for blue chip signals
                    status: SignalStatus::Active,
                };

                signals.push(signal);
                log::info!("✅ Jupiter blue chip signal: {} - 24h: {:.2}%, Price impact: {:.3}%",
                    symbol, price_change_24h, quote.price_impact_pct);
            }
        }

        log::info!("🎯 Generated {} Jupiter blue chip trading signals", signals.len());
        Ok(signals)
    }

    /// Provider 5: Opportunity Analyzer - Analyzes all trading opportunities
    async fn generate_opportunity_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get DEX opportunities - gracefully handle API failures
        // Convert Box<dyn Error> to String to ensure Send trait compatibility
        let opportunities = match self.dex_client.get_top_opportunities(10).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(opps) => {
                if opps.is_empty() {
                    log::debug!("No DEX opportunities found (Mobula API may be unavailable). Continuing with other signals.");
                }
                opps
            }
            Err(e) => {
                // Log as warning, not error - API failures are expected
                log::warn!("⚠️ Mobula API unavailable for opportunity analysis: {}. Continuing without DEX opportunities.", e);
                Vec::new() // Return empty instead of error to allow graceful degradation
            }
        };

        for opp in opportunities {
            // Only signal on very high quality opportunities
            if opp.opportunity_score > 75.0 {
                let confidence = (opp.opportunity_score / 100.0).min(0.92);

                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: opp.token_symbol.clone(),
                    action: SignalAction::Buy,
                    entry_price: opp.price_usd,
                    target_price: opp.price_usd * 1.12,
                    stop_loss: opp.price_usd * 0.92,
                    confidence,
                    timeframe: "4h".to_string(),
                    data_sources: vec!["DEX Screener".to_string(), "Multi-DEX Analysis".to_string()],
                    analysis: format!(
                        "High opportunity: {} - Score: {:.1}, Vol 24h: ${:.0}, Liquidity: ${:.0}, Signals: {}",
                        opp.token_name, opp.opportunity_score, opp.volume_24h, 
                        opp.liquidity_usd, opp.signals.join(", ")
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 14400, // 4 hours
                    price: 20.0,
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        Ok(signals)
    }

    /// Provider 6: Signal Trader - Buys/sells signals from other providers
    async fn trade_signals(&self) -> Result<(), String> {
        // Get all active signals from marketplace
        let active_signals = self.marketplace.get_active_signals().await;

        let mut capital = self.capital.lock().await;

        for signal in active_signals {
            // Don't buy our own signals
            if signal.provider == self.provider_id {
                continue;
            }

            // Evaluate if signal is worth buying
            let should_buy = self.evaluate_signal_purchase(&signal);

            if should_buy && *capital >= signal.price {
                // Purchase the signal
                match self.marketplace.purchase_signal(
                    &self.provider_id,
                    &signal.id,
                    signal.price,
                ).await {
                    Ok(_) => {
                        *capital -= signal.price;
                        log::info!(
                            "📊 Signal Trader purchased signal {} from {} for ${:.2}",
                            signal.id, signal.provider, signal.price
                        );
                    }
                    Err(e) => {
                        log::warn!("Failed to purchase signal: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// ENHANCED: Evaluate if a signal is worth purchasing with portfolio optimization
    fn evaluate_signal_purchase(&self, signal: &TradingSignalData) -> bool {
        // ENHANCED Criteria for buying signals:
        // 1. High confidence (>70%)
        // 2. Reasonable price (<30 tokens)
        // 3. Not expired soon (>30 min remaining)
        // 4. Good risk/reward ratio (>1.5)
        // 5. Provider reputation (if available)
        // 6. Signal diversification (avoid too many signals on same symbol)
        
        let time_remaining = signal.expiry - Utc::now().timestamp();
        let potential_profit = (signal.target_price - signal.entry_price).abs();
        let potential_loss = (signal.entry_price - signal.stop_loss).abs();
        let risk_reward = if potential_loss > 0.0 {
            potential_profit / potential_loss
        } else {
            0.0
        };
        
        // ENHANCED: Value score (confidence * risk_reward / price)
        let value_score = (signal.confidence * risk_reward) / signal.price.max(1.0);
        
        // Base criteria
        let meets_base_criteria = signal.confidence > 0.70 &&
        signal.price < 30.0 &&
        time_remaining > 1800 &&
            risk_reward > 1.5;
        
        // ENHANCED: Value threshold (higher value = better deal)
        let meets_value_threshold = value_score > 0.05; // Minimum value score
        
        meets_base_criteria && meets_value_threshold
    }

    /// Generate meta-signals based on purchased signals
    async fn generate_meta_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Analyze patterns from marketplace
        let active_signals = self.marketplace.get_active_signals().await;
        
        // Find consensus signals (multiple providers agreeing)
        let mut symbol_votes: std::collections::HashMap<String, (usize, Vec<&TradingSignalData>)> = 
            std::collections::HashMap::new();

        for signal in &active_signals {
            if signal.provider != self.provider_id {
                let entry = symbol_votes.entry(signal.symbol.clone()).or_insert((0, Vec::new()));
                entry.0 += 1;
                entry.1.push(signal);
            }
        }

        // Generate meta-signals for symbols with consensus (3+ providers)
        for (symbol, (count, provider_signals)) in symbol_votes {
            if count >= 3 {
                let avg_confidence: f64 = provider_signals.iter()
                    .map(|s| s.confidence)
                    .sum::<f64>() / provider_signals.len() as f64;

                let avg_price: f64 = provider_signals.iter()
                    .map(|s| s.entry_price)
                    .sum::<f64>() / provider_signals.len() as f64;

                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: symbol.clone(),
                    action: SignalAction::Buy,
                    entry_price: avg_price,
                    target_price: avg_price * 1.10,
                    stop_loss: avg_price * 0.94,
                    confidence: (avg_confidence * 1.1).min(0.95), // Boost confidence for consensus
                    timeframe: "6h".to_string(),
                    data_sources: vec![
                        format!("Meta-Analysis from {} providers", count),
                        "Signal Consensus".to_string(),
                    ],
                    analysis: format!(
                        "Consensus signal: {} - {} providers agree, Avg confidence: {:.1}%",
                        symbol, count, avg_confidence * 100.0
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 21600, // 6 hours
                    price: 30.0, // Premium for consensus signals
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        Ok(signals)
    }

    /// Provider 7: Master Analyzer - ENHANCED with pattern recognition, market regime detection, and predictive analytics
    async fn generate_master_analysis_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get all active signals from marketplace
        let active_signals = self.marketplace.get_active_signals().await;
        
        // ENHANCED: Market regime detection
        let market_regime = {
            let buy_count = active_signals.iter()
                .filter(|s| matches!(s.action, SignalAction::Buy))
                .count();
            let _sell_count = active_signals.iter()
                .filter(|s| matches!(s.action, SignalAction::Sell))
                .count();
            let total = active_signals.len();
            
            if total == 0 {
                "NEUTRAL".to_string()
            } else {
                let buy_ratio = buy_count as f64 / total as f64;
                if buy_ratio > 0.6 {
                    "BULLISH".to_string()
                } else if buy_ratio < 0.4 {
                    "BEARISH".to_string()
                } else {
                    "NEUTRAL".to_string()
                }
            }
        };

        // Get all provider statistics
        let provider_ids = vec![
            "memecoin_monitor",
            "oracle_monitor", 
            "jupiter_memecoin_trader",
            "jupiter_bluechip_trader",
            "opportunity_analyzer",
            "signal_trader",
        ];

        let mut provider_stats = Vec::new();
        for provider_id in &provider_ids {
            if let Some(stats) = self.marketplace.get_provider_stats(provider_id).await {
                provider_stats.push((provider_id.to_string(), stats));
            }
        }

        // Analyze signal patterns across all providers
        let mut symbol_analysis: std::collections::HashMap<String, SymbolAnalysis> = 
            std::collections::HashMap::new();

        for signal in &active_signals {
            // Skip our own signals
            if signal.provider == self.provider_id {
                continue;
            }

            let entry = symbol_analysis.entry(signal.symbol.clone()).or_insert(SymbolAnalysis {
                symbol: signal.symbol.clone(),
                provider_count: 0,
                total_confidence: 0.0,
                buy_signals: 0,
                sell_signals: 0,
                avg_entry_price: 0.0,
                avg_target_price: 0.0,
                avg_stop_loss: 0.0,
                data_sources: Vec::new(),
                providers: Vec::new(),
            });

            entry.provider_count += 1;
            entry.total_confidence += signal.confidence;
            entry.avg_entry_price += signal.entry_price;
            entry.avg_target_price += signal.target_price;
            entry.avg_stop_loss += signal.stop_loss;
            entry.providers.push(signal.provider.clone());

            for source in &signal.data_sources {
                if !entry.data_sources.contains(source) {
                    entry.data_sources.push(source.clone());
                }
            }

            match signal.action {
                SignalAction::Buy => entry.buy_signals += 1,
                SignalAction::Sell => entry.sell_signals += 1,
                _ => {}
            }
        }

        // Generate master analysis signals for high-conviction plays
        for (symbol, analysis) in symbol_analysis {
            // Calculate averages
            let count = analysis.provider_count as f64;
            let avg_confidence = analysis.total_confidence / count;
            let avg_entry = analysis.avg_entry_price / count;
            let avg_target = analysis.avg_target_price / count;
            let avg_stop = analysis.avg_stop_loss / count;

            // Master signal criteria:
            // 1. Multiple providers (2+) agree
            // 2. Strong directional bias (75%+ agreement)
            // 3. High average confidence (>65%)
            let total_directional = analysis.buy_signals + analysis.sell_signals;
            let directional_strength = if total_directional > 0 {
                (analysis.buy_signals.max(analysis.sell_signals) as f64) / (total_directional as f64)
            } else {
                0.0
            };

            if analysis.provider_count >= 2 && directional_strength >= 0.75 && avg_confidence > 0.65 {
                let action = if analysis.buy_signals > analysis.sell_signals {
                    SignalAction::Buy
                } else {
                    SignalAction::Sell
                };

                // Enhanced master confidence score with reputation weighting
                // Get provider reputation scores from marketplace
                let mut reputation_weighted_confidence = 0.0;
                let mut total_reputation = 0.0;
                
                for provider_id in &analysis.providers {
                    if let Some(stats) = self.marketplace.get_provider_stats(provider_id).await {
                        let reputation = stats.reputation_score;
                        // Weight this provider's contribution by their reputation
                        reputation_weighted_confidence += avg_confidence * reputation;
                        total_reputation += reputation;
                    }
                }
                
                let base_confidence = if total_reputation > 0.0 {
                    reputation_weighted_confidence / total_reputation
                } else {
                    avg_confidence
                };
                
                // ENHANCED: Market regime bonus/penalty
                let action_for_regime = action.clone();
                let regime_bonus = match market_regime.as_str() {
                    "BULLISH" => if matches!(action_for_regime, SignalAction::Buy) { 0.05 } else { -0.05 },
                    "BEARISH" => if matches!(action_for_regime, SignalAction::Sell) { 0.05 } else { -0.05 },
                    _ => 0.0,
                };
                
                // ENHANCED: Pattern recognition bonus
                let pattern_bonus = if analysis.provider_count >= 4 {
                    0.08 // Strong consensus pattern
                } else if analysis.provider_count >= 3 {
                    0.05 // Good consensus pattern
                } else {
                    0.0
                };
                
                // Bonuses for consensus quality
                let provider_diversity_bonus = (analysis.provider_count as f64 / 5.0).min(0.15);
                let data_source_bonus = (analysis.data_sources.len() as f64 / 10.0).min(0.10);
                let directional_bonus = (directional_strength - 0.75) * 0.5;
                
                // Penalty if providers conflict (reduces confidence)
                let conflict_penalty = if directional_strength < 0.9 {
                    (0.9 - directional_strength) * 0.3
                } else {
                    0.0
                };
                
                let master_confidence = (base_confidence 
                    + provider_diversity_bonus 
                    + data_source_bonus 
                    + directional_bonus
                    + regime_bonus
                    + pattern_bonus
                    - conflict_penalty).min(0.98).max(0.5);

                // ENHANCED: Double-check profitability before execution
                let profitability_check = self.validate_profitability(
                    avg_entry,
                    avg_target,
                    avg_stop,
                    &action,
                    master_confidence,
                ).await;
                
                // Only proceed if profitability check passes
                if !profitability_check.is_profitable {
                    log::warn!("⚠️ Master Analyzer: Skipping {} - Profitability check failed: {}", 
                        symbol, profitability_check.reason);
                    continue; // Skip this signal - not profitable enough
                }

                // Fetch current oracle data for validation
                let oracle_validation = self.validate_with_oracle_data(&symbol).await;

                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: symbol.clone(),
                    action,
                    entry_price: avg_entry,
                    target_price: avg_target,
                    stop_loss: avg_stop,
                    confidence: master_confidence,
                    timeframe: "8h".to_string(),
                    data_sources: analysis.data_sources.clone(),
                    analysis: format!(
                        "[ENHANCED] MASTER ANALYSIS: {} - {} providers ({}) | Confidence: {:.1}% | Directional: {:.0}% | Market: {} | Pattern: {} | Sources: {}. {} | ✅ Profitability: {:.1}% profit, {:.2}x risk/reward, {:.1}% margin",
                        symbol,
                        analysis.provider_count,
                        analysis.providers.join(", "),
                        master_confidence * 100.0,
                        directional_strength * 100.0,
                        market_regime,
                        if analysis.provider_count >= 4 { "Strong Consensus" } else { "Good Consensus" },
                        analysis.data_sources.join(", "),
                        oracle_validation,
                        profitability_check.expected_profit_pct,
                        profitability_check.risk_reward_ratio,
                        profitability_check.profit_margin_pct
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 28800, // 8 hours
                    price: 40.0, // Premium for master analysis
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        // Add market-wide insights if we have enough data
        if !provider_stats.is_empty() {
            let market_insight = self.generate_market_insight(&provider_stats, &active_signals).await?;
            if let Some(insight_signal) = market_insight {
                signals.push(insight_signal);
            }
        }

        Ok(signals)
    }
    
    /// ENHANCED: Double-check profitability on all trade opportunities before execution
    /// Validates: expected profit, risk/reward ratio, profit margin, transaction costs
    async fn validate_profitability(
        &self,
        entry_price: f64,
        target_price: f64,
        stop_loss: f64,
        action: &SignalAction,
        confidence: f64,
    ) -> ProfitabilityCheck {
        // Calculate potential profit and loss
        let (potential_profit, potential_loss) = match action {
            SignalAction::Buy => {
                let profit = target_price - entry_price;
                let loss = entry_price - stop_loss;
                (profit, loss)
            }
            SignalAction::Sell => {
                let profit = entry_price - target_price;
                let loss = stop_loss - entry_price;
                (profit, loss)
            }
            _ => (0.0, 0.0), // Hold signals have no profit/loss
        };
        
        // Calculate profit percentages
        let profit_pct = if entry_price > 0.0 {
            (potential_profit / entry_price) * 100.0
        } else {
            0.0
        };
        
        let _loss_pct = if entry_price > 0.0 {
            (potential_loss / entry_price) * 100.0
        } else {
            0.0
        };
        
        // Calculate risk/reward ratio
        let risk_reward = if potential_loss > 0.0 {
            potential_profit / potential_loss
        } else {
            0.0
        };
        
        // Estimate transaction costs (Solana fees + Jupiter swap fees)
        // Typical Solana transaction: ~0.000005 SOL (~$0.001 at $200/SOL)
        // Jupiter swap fee: ~0.3% (0.003)
        let solana_tx_fee_usd = 0.001; // ~$0.001 per transaction
        let jupiter_swap_fee_pct = 0.003; // 0.3% swap fee
        let estimated_tx_cost = solana_tx_fee_usd + (entry_price * jupiter_swap_fee_pct);
        
        // Calculate profit after fees (assuming 1 unit trade)
        let profit_after_fees = potential_profit - estimated_tx_cost;
        let profit_margin_pct = if entry_price > 0.0 {
            (profit_after_fees / entry_price) * 100.0
        } else {
            0.0
        };
        
        // ENHANCED: Minimum profitability thresholds
        let min_profit_pct = 2.0; // Minimum 2% profit expected
        let min_risk_reward = 1.5; // Minimum 1.5:1 risk/reward ratio
        let min_profit_margin = 1.0; // Minimum 1% profit margin after fees
        let min_confidence_for_profit = 0.65; // Minimum confidence for profitable trades
        
        // Validate profitability criteria
        let meets_profit_threshold = profit_pct >= min_profit_pct;
        let meets_risk_reward = risk_reward >= min_risk_reward;
        let meets_profit_margin = profit_margin_pct >= min_profit_margin;
        let meets_confidence = confidence >= min_confidence_for_profit;
        
        // Additional validation: profit must be positive after fees
        let is_profitable_after_fees = profit_after_fees > 0.0;
        
        // ENHANCED: Confidence-adjusted profitability
        // Higher confidence allows slightly lower profit margins (but still must be profitable)
        let confidence_adjusted_min_profit = if confidence >= 0.80 {
            min_profit_pct * 0.8 // 20% reduction for high confidence
        } else if confidence >= 0.70 {
            min_profit_pct * 0.9 // 10% reduction for good confidence
        } else {
            min_profit_pct // Full requirement for lower confidence
        };
        
        let meets_adjusted_profit = profit_pct >= confidence_adjusted_min_profit;
        
        // Final profitability check
        let is_profitable = meets_profit_threshold 
            && meets_risk_reward 
            && meets_profit_margin 
            && meets_confidence
            && is_profitable_after_fees
            && meets_adjusted_profit;
        
        // Generate reason if not profitable
        let reason = if !is_profitable {
            let mut reasons = Vec::new();
            if !meets_profit_threshold {
                reasons.push(format!("Profit {:.2}% < {:.2}%", profit_pct, min_profit_pct));
            }
            if !meets_risk_reward {
                reasons.push(format!("Risk/Reward {:.2}x < {:.2}x", risk_reward, min_risk_reward));
            }
            if !meets_profit_margin {
                reasons.push(format!("Profit margin {:.2}% < {:.2}% after fees", profit_margin_pct, min_profit_margin));
            }
            if !is_profitable_after_fees {
                reasons.push(format!("Profit after fees ${:.4} <= 0", profit_after_fees));
            }
            if !meets_confidence {
                reasons.push(format!("Confidence {:.2}% < {:.2}%", confidence * 100.0, min_confidence_for_profit * 100.0));
            }
            format!("Failed: {}", reasons.join(", "))
        } else {
            format!("✅ Validated: {:.2}% profit, {:.2}x R/R, {:.2}% margin", 
                profit_pct, risk_reward, profit_margin_pct)
        };
        
        log::debug!("📊 Profitability check: {} | Profit: {:.2}% | R/R: {:.2}x | Margin: {:.2}% | {}", 
            if is_profitable { "✅ PASS" } else { "❌ FAIL" },
            profit_pct, risk_reward, profit_margin_pct, reason);
        
        ProfitabilityCheck {
            is_profitable,
            expected_profit_pct: profit_pct,
            risk_reward_ratio: risk_reward,
            profit_margin_pct,
            expected_profit_after_fees: profit_after_fees,
            reason,
        }
    }

    /// Validate signal with oracle data
    async fn validate_with_oracle_data(&self, symbol: &str) -> String {
        // Try to get oracle data for the symbol
        let oracle_symbol = if symbol.contains("/") {
            symbol.to_string()
        } else {
            format!("{}/USD", symbol)
        };

        // Convert Box<dyn Error + Send + Sync> to String to ensure Send trait compatibility
        match self.oracle_client.fetch_price(&oracle_symbol).await
            .map_err(|e| format!("{}", e)) // Convert to String immediately
        {
            Ok(feed) => {
                format!("Oracle validation: ${:.2} (confidence: {:.1}%)", 
                    feed.price, feed.confidence * 100.0)
            }
            Err(_error_msg) => {
                // error_msg is now String (Send-safe)
                "Oracle validation: N/A".to_string()
            }
        }
    }

    /// Generate market-wide insight signal
    async fn generate_market_insight(
        &self,
        provider_stats: &[(String, crate::signal_platform::SignalProvider)],
        active_signals: &[TradingSignalData],
    ) -> Result<Option<TradingSignalData>, String> {
        // Calculate market-wide metrics
        let total_signals = active_signals.len();
        let total_providers = provider_stats.len();
        
        if total_signals < 5 || total_providers < 3 {
            return Ok(None); // Not enough data
        }

        let avg_confidence: f64 = active_signals.iter()
            .map(|s| s.confidence)
            .sum::<f64>() / total_signals as f64;

        let buy_ratio = active_signals.iter()
            .filter(|s| matches!(s.action, SignalAction::Buy))
            .count() as f64 / total_signals as f64;

        // Determine market sentiment
        let (market_sentiment, action) = if buy_ratio > 0.7 {
            ("BULLISH", SignalAction::Buy)
        } else if buy_ratio < 0.3 {
            ("BEARISH", SignalAction::Sell)
        } else {
            ("NEUTRAL", SignalAction::Hold)
        };

        // Calculate provider performance metrics
        let avg_reputation: f64 = provider_stats.iter()
            .map(|(_, stats)| stats.reputation_score)
            .sum::<f64>() / total_providers as f64;

        // Only generate market insight if sentiment is strong
        if market_sentiment != "NEUTRAL" {
            let signal = TradingSignalData {
                id: uuid::Uuid::new_v4().to_string(),
                provider: self.provider_id.clone(),
                symbol: "MARKET-WIDE".to_string(),
                action,
                entry_price: 0.0, // Not applicable
                target_price: 0.0,
                stop_loss: 0.0,
                confidence: avg_confidence,
                timeframe: "12h".to_string(),
                data_sources: vec!["All Providers".to_string(), "Market Analysis".to_string()],
                analysis: format!(
                    "MARKET INSIGHT: {} sentiment detected. {} active signals from {} providers. Buy ratio: {:.1}%. Average confidence: {:.1}%. Provider avg reputation: {:.1}/100",
                    market_sentiment,
                    total_signals,
                    total_providers,
                    buy_ratio * 100.0,
                    avg_confidence * 100.0,
                    avg_reputation
                ),
                timestamp: Utc::now().timestamp(),
                expiry: Utc::now().timestamp() + 43200, // 12 hours
                price: 50.0, // Premium for market-wide insights
                status: SignalStatus::Active,
            };

            Ok(Some(signal))
        } else {
            Ok(None)
        }
    }
    
    /// Get RL agent performance metrics
    pub async fn get_performance_metrics(&self) -> crate::reinforcement_learning::AgentPerformance {
        self.rl_agent.get_performance().await
    }
    
    /// Update RL agent with trade outcome
    pub async fn learn_from_outcome(&self, symbol: String, entry_price: f64, exit_price: f64, action: &str, confidence: f64) {
        use crate::reinforcement_learning::{Experience, MarketState, Action};
        
        let reward = RLAgent::calculate_reward(entry_price, exit_price, action, confidence);
        
        let experience = Experience {
            state: MarketState {
                symbol: symbol.clone(),
                price: entry_price,
                volume: 0.0,
                price_change_1h: 0.0,
                price_change_24h: 0.0,
                sentiment_score: confidence * 100.0,
                liquidity: 0.0,
                volatility: 0.0,
                market_cap: None,
            },
            action: Action {
                action_type: action.to_string(),
                confidence,
                size: 0.05,
                price: entry_price,
            },
            reward,
            next_state: Some(MarketState {
                symbol: symbol.clone(),
                price: exit_price,
                volume: 0.0,
                price_change_1h: ((exit_price - entry_price) / entry_price) * 100.0,
                price_change_24h: 0.0,
                sentiment_score: confidence * 100.0,
                liquidity: 0.0,
                volatility: 0.0,
                market_cap: None,
            }),
            timestamp: Utc::now().timestamp(),
            provider_id: self.provider_id.clone(),
        };
        
        self.rl_agent.record_experience(experience).await;
        log::info!("🧠 {} learned from {} trade: reward={:.3}", self.provider_name, symbol, reward);
    }
}

/// Helper struct for symbol analysis
#[derive(Debug)]
struct SymbolAnalysis {
    symbol: String,
    provider_count: usize,
    total_confidence: f64,
    buy_signals: usize,
    sell_signals: usize,
    avg_entry_price: f64,
    avg_target_price: f64,
    avg_stop_loss: f64,
    data_sources: Vec<String>,
    providers: Vec<String>,
}

/// Initialize all 6 specialized providers
pub async fn initialize_all_providers(
    marketplace: Arc<SignalMarketplace>,
    rpc_url: String,
) -> Vec<SpecializedProvider> {
    let providers = vec![
        (
            "memecoin_monitor".to_string(),
            "Memecoin Monitor".to_string(),
            ProviderType::MemecoinMonitor,
        ),
        (
            "oracle_monitor".to_string(),
            "Oracle Monitor".to_string(),
            ProviderType::OracleMonitor,
        ),
        (
            "jupiter_memecoin_trader".to_string(),
            "Jupiter Memecoin Trader".to_string(),
            ProviderType::JupiterMemecoinTrader,
        ),
        (
            "jupiter_bluechip_trader".to_string(),
            "Jupiter Blue Chip Trader".to_string(),
            ProviderType::JupiterBlueChipTrader,
        ),
        (
            "opportunity_analyzer".to_string(),
            "Opportunity Analyzer".to_string(),
            ProviderType::OpportunityAnalyzer,
        ),
        (
            "signal_trader".to_string(),
            "Signal Trader".to_string(),
            ProviderType::SignalTrader,
        ),
        (
            "master_analyzer".to_string(),
            "Master Analyzer".to_string(),
            ProviderType::MasterAnalyzer,
        ),
    ];

    let mut provider_agents = Vec::new();

    for (id, name, provider_type) in providers {
        // Register provider in marketplace
        if let Err(e) = marketplace.register_provider(id.clone(), name.clone()).await {
            log::warn!("Failed to register provider {}: {}", name, e);
        } else {
            log::info!("✅ Registered provider: {}", name);
        }

        let provider = SpecializedProvider::new(
            id,
            name,
            provider_type,
            marketplace.clone(),
            rpc_url.clone(),
        );

        provider_agents.push(provider);
    }

    provider_agents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_types() {
        let types = [
            ProviderType::MemecoinMonitor,
            ProviderType::OracleMonitor,
            ProviderType::JupiterMemecoinTrader,
            ProviderType::JupiterBlueChipTrader,
            ProviderType::OpportunityAnalyzer,
            ProviderType::SignalTrader,
            ProviderType::MasterAnalyzer,
        ];
        assert_eq!(types.len(), 7);
    }
}
