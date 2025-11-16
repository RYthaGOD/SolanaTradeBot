use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::dex_screener::DexScreenerClient;
use crate::jupiter_integration::JupiterClient;
use crate::pumpfun::PumpFunClient;
use crate::reinforcement_learning::{LearningCoordinator, RLAgent};
use crate::signal_platform::{SignalAction, SignalMarketplace, SignalStatus, TradingSignalData};
use crate::switchboard_oracle::SwitchboardClient;

/// Provider specialization type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    MemecoinMonitor,
    OracleMonitor,
    PerpsMonitor,
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
            oracle_client: Arc::new(SwitchboardClient::new(
                rpc_url.clone(),
                std::env::var("SOLANA_RPC_URL").is_ok(),
            )),
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
        self.rl_coordinator = Some(coordinator);
        self
    }

    /// Main provider loop
    pub async fn run(&self) {
        log::info!(
            "🤖 Starting {} provider: {}",
            self.provider_name,
            self.provider_id
        );

        loop {
            match self.generate_and_publish_signals().await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("✅ {} published {} signals", self.provider_name, count);
                    }
                }
                Err(e) => {
                    log::error!("❌ {} error: {}", self.provider_name, e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(self.check_interval_secs)).await;
        }
    }

    /// Generate and publish signals based on provider type
    async fn generate_and_publish_signals(&self) -> Result<usize, String> {
        let signals = match &self.provider_type {
            ProviderType::MemecoinMonitor => self.generate_memecoin_signals().await?,
            ProviderType::OracleMonitor => self.generate_oracle_signals().await?,
            ProviderType::PerpsMonitor => self.generate_perps_signals().await?,
            ProviderType::OpportunityAnalyzer => self.generate_opportunity_signals().await?,
            ProviderType::SignalTrader => {
                // Signal trader both generates and trades signals
                self.trade_signals().await?;
                self.generate_meta_signals().await?
            }
            ProviderType::MasterAnalyzer => {
                // Master analyzer analyzes all provider data
                self.generate_master_analysis_signals().await?
            }
        };

        let mut published_count = 0;
        for signal in signals {
            match self.marketplace.publish_signal(signal.clone()).await {
                Ok(signal_id) => {
                    published_count += 1;
                    log::debug!("Published signal {} for {}", signal_id, signal.symbol);

                    // Register this agent with RL coordinator if connected
                    if let Some(coordinator) = &self.rl_coordinator {
                        let coordinator_lock = coordinator.lock().await;
                        coordinator_lock.register_agent(self.rl_agent.clone()).await;
                    }
                }
                Err(e) => log::warn!("Failed to publish signal: {}", e),
            }
        }

        Ok(published_count)
    }

    /// Provider 1: Memecoin Monitor - Analyzes memecoins using oracle data
    async fn generate_memecoin_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get recent meme launches
        let launches = self
            .pumpfun_client
            .get_recent_launches(10)
            .await
            .map_err(|e| format!("PumpFun error: {}", e))?;

        // Get oracle data for price validation
        let oracle_feeds = self
            .oracle_client
            .fetch_multiple_feeds(&["SOL/USD".to_string()])
            .await
            .map_err(|e| format!("Oracle error: {}", e))?;

        let sol_price = oracle_feeds.first().map(|f| f.price).unwrap_or(100.0);

        for launch in launches {
            let sentiment = self.pumpfun_client.analyze_sentiment(&launch);

            // Only signal on high confidence memecoins
            if sentiment.sentiment_score > 60.0 {
                let confidence = (sentiment.sentiment_score / 100.0).min(0.95);

                // Calculate realistic entry based on SOL price
                let entry_price = launch.market_cap / 1000000.0;

                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: launch.symbol.clone(),
                    action: SignalAction::Buy,
                    entry_price,
                    target_price: entry_price * 1.20, // 20% target
                    stop_loss: entry_price * 0.85, // 15% stop
                    confidence,
                    timeframe: "30m".to_string(),
                    data_sources: vec!["PumpFun".to_string(), "Oracle SOL Price".to_string()],
                    analysis: format!(
                        "Memecoin {} - Sentiment: {:.1}/100, Hype: {:?}, Market Cap: ${:.0}, SOL Price: ${:.2}",
                        launch.name, sentiment.sentiment_score, sentiment.hype_level, launch.market_cap, sol_price
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 1800, // 30 min expiry
                    price: 25.0, // Premium price for meme signals
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        Ok(signals)
    }

    /// Provider 2: Oracle Monitor - Pure oracle data analysis
    async fn generate_oracle_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        let symbols = vec![
            "SOL/USD".to_string(),
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
        ];

        let feeds = self
            .oracle_client
            .fetch_multiple_feeds(&symbols)
            .await
            .map_err(|e| format!("Oracle error: {}", e))?;

        for feed in feeds {
            // Simulate price history for trend detection
            let prev_price = feed.price * (1.0 - (rand::random::<f64>() * 0.04 - 0.02));
            let change = SwitchboardClient::calculate_price_change(prev_price, feed.price);

            // Generate signals on significant movements
            if change.abs() > 1.5 {
                let action = if change > 0.0 {
                    SignalAction::Buy
                } else {
                    SignalAction::Sell
                };

                let confidence = (change.abs() / 5.0).min(0.90);

                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: feed.symbol.clone(),
                    action,
                    entry_price: feed.price,
                    target_price: if change > 0.0 {
                        feed.price * 1.03
                    } else {
                        feed.price * 0.97
                    },
                    stop_loss: if change > 0.0 {
                        feed.price * 0.98
                    } else {
                        feed.price * 1.02
                    },
                    confidence,
                    timeframe: "1h".to_string(),
                    data_sources: vec!["Switchboard Oracle".to_string()],
                    analysis: format!(
                        "Oracle-based signal: {} movement of {:.2}%, Price: ${:.2}, Confidence: {:.1}%",
                        feed.symbol, change, feed.price, feed.confidence * 100.0
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

    /// Provider 3: Perps Monitor - Jupiter perps using oracle data
    async fn generate_perps_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get oracle data for perps analysis
        let feeds = self
            .oracle_client
            .fetch_multiple_feeds(&[
                "SOL/USD".to_string(),
                "BTC/USD".to_string(),
                "ETH/USD".to_string(),
            ])
            .await
            .map_err(|e| format!("Oracle error: {}", e))?;

        // Analyze each asset for perps opportunities
        for feed in feeds {
            // Simulate volatility analysis
            let volatility = rand::random::<f64>() * 0.1; // 0-10% volatility

            // High volatility = good for perps trading
            if volatility > 0.05 {
                let confidence = (volatility * 10.0).min(0.85);

                // Determine direction based on oracle confidence
                let action = if feed.confidence < 0.5 {
                    // Low confidence = possible breakout
                    if rand::random::<bool>() {
                        SignalAction::Buy
                    } else {
                        SignalAction::Sell
                    }
                } else {
                    SignalAction::Buy
                };

                let leverage = 2.0; // 2x leverage suggestion
                let is_buy = matches!(action, SignalAction::Buy);

                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: self.provider_id.clone(),
                    symbol: format!("{}-PERP", feed.symbol.replace("/USD", "")),
                    action,
                    entry_price: feed.price,
                    target_price: feed.price * if is_buy {
                        1.05 * leverage
                    } else {
                        0.95 / leverage
                    },
                    stop_loss: feed.price * if is_buy {
                        0.97
                    } else {
                        1.03
                    },
                    confidence,
                    timeframe: "2h".to_string(),
                    data_sources: vec!["Switchboard Oracle".to_string(), "Jupiter Perps".to_string()],
                    analysis: format!(
                        "Perps signal: {} - Volatility: {:.1}%, Suggested Leverage: {}x, Oracle Confidence: {:.1}%",
                        feed.symbol, volatility * 100.0, leverage, feed.confidence * 100.0
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 7200, // 2 hours
                    price: 15.0,
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        Ok(signals)
    }

    /// Provider 4: Opportunity Analyzer - Analyzes all trading opportunities
    async fn generate_opportunity_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get DEX opportunities
        let opportunities = self
            .dex_client
            .get_top_opportunities(10)
            .await
            .map_err(|e| format!("DEX error: {}", e))?;

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

    /// Provider 5: Signal Trader - Buys/sells signals from other providers
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
                match self
                    .marketplace
                    .purchase_signal(&self.provider_id, &signal.id, signal.price)
                    .await
                {
                    Ok(_) => {
                        *capital -= signal.price;
                        log::info!(
                            "📊 Signal Trader purchased signal {} from {} for ${:.2}",
                            signal.id,
                            signal.provider,
                            signal.price
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

    /// Evaluate if a signal is worth purchasing
    fn evaluate_signal_purchase(&self, signal: &TradingSignalData) -> bool {
        // Criteria for buying signals:
        // 1. High confidence (>70%)
        // 2. Reasonable price (<30 tokens)
        // 3. Not expired soon (>30 min remaining)
        // 4. Good risk/reward ratio

        let time_remaining = signal.expiry - Utc::now().timestamp();
        let risk_reward = (signal.target_price - signal.entry_price).abs()
            / (signal.entry_price - signal.stop_loss).abs();

        signal.confidence > 0.70
            && signal.price < 30.0
            && time_remaining > 1800
            && risk_reward > 1.5
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
                let entry = symbol_votes
                    .entry(signal.symbol.clone())
                    .or_insert((0, Vec::new()));
                entry.0 += 1;
                entry.1.push(signal);
            }
        }

        // Generate meta-signals for symbols with consensus (3+ providers)
        for (symbol, (count, provider_signals)) in symbol_votes {
            if count >= 3 {
                let avg_confidence: f64 =
                    provider_signals.iter().map(|s| s.confidence).sum::<f64>()
                        / provider_signals.len() as f64;

                let avg_price: f64 = provider_signals.iter().map(|s| s.entry_price).sum::<f64>()
                    / provider_signals.len() as f64;

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
                        symbol,
                        count,
                        avg_confidence * 100.0
                    ),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 21600, // 6 hours
                    price: 30.0,                            // Premium for consensus signals
                    status: SignalStatus::Active,
                };

                signals.push(signal);
            }
        }

        Ok(signals)
    }

    /// Provider 6: Master Analyzer - Analyzes all provider data and market correlations
    async fn generate_master_analysis_signals(&self) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Get all active signals from marketplace
        let active_signals = self.marketplace.get_active_signals().await;

        // Get all provider statistics
        let provider_ids = vec![
            "memecoin_monitor",
            "oracle_monitor",
            "perps_monitor",
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

            let entry = symbol_analysis
                .entry(signal.symbol.clone())
                .or_insert(SymbolAnalysis {
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
                (analysis.buy_signals.max(analysis.sell_signals) as f64)
                    / (total_directional as f64)
            } else {
                0.0
            };

            if analysis.provider_count >= 2 && directional_strength >= 0.75 && avg_confidence > 0.65
            {
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
                    - conflict_penalty)
                    .min(0.98)
                    .max(0.5);

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
                        "MASTER ANALYSIS: {} - {} providers ({}) with {:.1}% confidence, {:.0}% directional agreement. Sources: {}. {}",
                        symbol,
                        analysis.provider_count,
                        analysis.providers.join(", "),
                        master_confidence * 100.0,
                        directional_strength * 100.0,
                        analysis.data_sources.join(", "),
                        oracle_validation
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
            let market_insight = self
                .generate_market_insight(&provider_stats, &active_signals)
                .await?;
            if let Some(insight_signal) = market_insight {
                signals.push(insight_signal);
            }
        }

        Ok(signals)
    }

    /// Validate signal with oracle data
    async fn validate_with_oracle_data(&self, symbol: &str) -> String {
        // Try to get oracle data for the symbol
        let oracle_symbol = if symbol.contains("/") {
            symbol.to_string()
        } else {
            format!("{}/USD", symbol)
        };

        match self.oracle_client.fetch_price(&oracle_symbol).await {
            Ok(feed) => {
                format!(
                    "Oracle validation: ${:.2} (confidence: {:.1}%)",
                    feed.price,
                    feed.confidence * 100.0
                )
            }
            Err(_) => "Oracle validation: N/A".to_string(),
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

        let avg_confidence: f64 =
            active_signals.iter().map(|s| s.confidence).sum::<f64>() / total_signals as f64;

        let buy_ratio = active_signals
            .iter()
            .filter(|s| matches!(s.action, SignalAction::Buy))
            .count() as f64
            / total_signals as f64;

        // Determine market sentiment
        let (market_sentiment, action) = if buy_ratio > 0.7 {
            ("BULLISH", SignalAction::Buy)
        } else if buy_ratio < 0.3 {
            ("BEARISH", SignalAction::Sell)
        } else {
            ("NEUTRAL", SignalAction::Hold)
        };

        // Calculate provider performance metrics
        let avg_reputation: f64 = provider_stats
            .iter()
            .map(|(_, stats)| stats.reputation_score)
            .sum::<f64>()
            / total_providers as f64;

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
    pub async fn learn_from_outcome(
        &self,
        symbol: String,
        entry_price: f64,
        exit_price: f64,
        action: &str,
        confidence: f64,
    ) {
        use crate::reinforcement_learning::{Action, Experience, MarketState};

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
        log::info!(
            "🧠 {} learned from {} trade: reward={:.3}",
            self.provider_name,
            symbol,
            reward
        );
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
            "perps_monitor".to_string(),
            "Perps Monitor".to_string(),
            ProviderType::PerpsMonitor,
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
        if let Err(e) = marketplace
            .register_provider(id.clone(), name.clone())
            .await
        {
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
            ProviderType::PerpsMonitor,
            ProviderType::OpportunityAnalyzer,
            ProviderType::SignalTrader,
        ];
        assert_eq!(types.len(), 5);
    }
}
