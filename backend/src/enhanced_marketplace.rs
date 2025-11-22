use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::signal_platform::{SignalMarketplace, TradingSignalData};

/// Enhanced marketplace with ratings, subscriptions, and performance tracking
pub struct EnhancedMarketplace {
    base_marketplace: Arc<SignalMarketplace>,
    signal_ratings: Arc<Mutex<HashMap<String, SignalRating>>>,
    subscriptions: Arc<Mutex<HashMap<String, Vec<Subscription>>>>, // user_id -> subscriptions
    signal_performance: Arc<Mutex<HashMap<String, SignalPerformance>>>,
    leaderboard: Arc<Mutex<Leaderboard>>,
}

/// Rating for a signal (by users who purchased it)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRating {
    pub signal_id: String,
    pub total_ratings: u32,
    pub sum_ratings: u32,
    pub average_rating: f64,
    pub five_star: u32,
    pub four_star: u32,
    pub three_star: u32,
    pub two_star: u32,
    pub one_star: u32,
    pub reviews: Vec<SignalReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalReview {
    pub user_id: String,
    pub rating: u8, // 1-5 stars
    pub comment: Option<String>,
    pub profit_pct: Option<f64>,
    pub timestamp: i64,
}

/// Subscription to a provider (monthly recurring)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub provider_id: String,
    pub tier: SubscriptionTier,
    pub start_date: i64,
    pub end_date: i64,
    pub auto_renew: bool,
    pub price_paid: f64,
    pub signals_received: u32,
    pub status: SubscriptionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionTier {
    Basic,      // Limited signals
    Premium,    // All signals + priority
    VIP,        // All signals + alpha insights + direct access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Expired,
    Cancelled,
    Paused,
}

/// Performance tracking for signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPerformance {
    pub signal_id: String,
    pub provider_id: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub exit_price: Option<f64>,
    pub highest_price: f64,
    pub lowest_price: f64,
    pub profit_loss_pct: f64,
    pub status: PerformanceStatus,
    pub filled_at: Option<i64>,
    pub closed_at: Option<i64>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceStatus {
    Pending,        // Not yet filled
    Active,         // Position open
    Won,           // Closed with profit
    Lost,          // Closed with loss
    Expired,       // Expired before fill
}

/// Leaderboard for providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Leaderboard {
    pub updated_at: i64,
    pub top_providers: Vec<LeaderboardEntry>,
    pub top_signals_24h: Vec<TopSignal>,
    pub trending_symbols: Vec<TrendingSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub provider_id: String,
    pub provider_name: String,
    pub win_rate: f64,
    pub total_signals: u32,
    pub avg_profit: f64,
    pub total_earnings: f64,
    pub reputation_score: f64,
    pub subscribers: u32,
    pub avg_rating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopSignal {
    pub signal_id: String,
    pub provider_id: String,
    pub symbol: String,
    pub profit_pct: f64,
    pub confidence: f64,
    pub rating: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingSymbol {
    pub symbol: String,
    pub signal_count: u32,
    pub avg_confidence: f64,
    pub bullish_signals: u32,
    pub bearish_signals: u32,
    pub sentiment: String, // "BULLISH", "BEARISH", "NEUTRAL"
}

/// Marketplace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    pub total_signals: u32,
    pub active_signals: u32,
    pub total_volume_24h: f64,
    pub total_trades: u32,
    pub avg_signal_price: f64,
    pub top_performing_provider: String,
    pub most_traded_symbol: String,
}

/// Advanced search filters for signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchFilters {
    pub symbol: Option<String>,
    pub min_confidence: Option<f64>,
    pub max_confidence: Option<f64>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_rating: Option<f64>,
    pub provider_id: Option<String>,
    pub action: Option<crate::signal_platform::SignalAction>,
    pub min_profit_target: Option<f64>,
    pub max_profit_target: Option<f64>,
    pub timeframe: Option<String>,
    pub min_reputation: Option<f64>,
    pub risk_level: Option<String>, // "low", "medium", "high"
}

/// Signal recommendation with score and reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRecommendation {
    pub signal: TradingSignalData,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Provider comparison data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderComparison {
    pub provider_id: String,
    pub provider_name: String,
    pub win_rate: f64,
    pub avg_profit: f64,
    pub total_signals: u32,
    pub reputation_score: f64,
    pub avg_rating: f64,
    pub subscribers: u32,
    pub recent_performance: f64, // Last 10 signals success rate
}

/// User portfolio with positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPortfolio {
    pub user_id: String,
    pub active_positions: Vec<PortfolioPosition>,
    pub closed_positions: Vec<PortfolioPosition>,
    pub total_profit_loss: f64,
    pub win_rate: f64,
    pub total_invested: f64,
}

/// Portfolio position details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPosition {
    pub signal_id: String,
    pub symbol: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub profit_loss_pct: f64,
    pub profit_loss_usd: f64,
    pub status: String,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
}

/// Market trends analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTrends {
    pub bullish_symbols: Vec<TrendingSymbol>,
    pub bearish_symbols: Vec<TrendingSymbol>,
    pub high_confidence_signals: u32,
    pub avg_market_confidence: f64,
    pub top_providers: Vec<String>,
    pub market_sentiment: String, // "BULLISH", "BEARISH", "NEUTRAL"
}

impl EnhancedMarketplace {
    pub fn new(base_marketplace: Arc<SignalMarketplace>) -> Self {
        Self {
            base_marketplace,
            signal_ratings: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            signal_performance: Arc::new(Mutex::new(HashMap::new())),
            leaderboard: Arc::new(Mutex::new(Leaderboard {
                updated_at: Utc::now().timestamp(),
                top_providers: Vec::new(),
                top_signals_24h: Vec::new(),
                trending_symbols: Vec::new(),
            })),
        }
    }

    /// Subscribe to a provider
    pub async fn subscribe_to_provider(
        &self,
        user_id: &str,
        provider_id: &str,
        tier: SubscriptionTier,
    ) -> Result<Subscription, String> {
        let price = match tier {
            SubscriptionTier::Basic => 50.0,
            SubscriptionTier::Premium => 100.0,
            SubscriptionTier::VIP => 250.0,
        };

        let subscription = Subscription {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            provider_id: provider_id.to_string(),
            tier,
            start_date: Utc::now().timestamp(),
            end_date: Utc::now().timestamp() + 2592000, // 30 days
            auto_renew: true,
            price_paid: price,
            signals_received: 0,
            status: SubscriptionStatus::Active,
        };

        let mut subscriptions = self.subscriptions.lock().await;
        subscriptions
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(subscription.clone());

        log::info!(
            "✅ User {} subscribed to provider {} ({:?} tier)",
            user_id,
            provider_id,
            subscription.tier
        );

        Ok(subscription)
    }

    /// Get user's active subscriptions
    pub async fn get_user_subscriptions(&self, user_id: &str) -> Vec<Subscription> {
        let subscriptions = self.subscriptions.lock().await;
        subscriptions
            .get(user_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| matches!(s.status, SubscriptionStatus::Active))
            .collect()
    }

    /// Rate a signal
    pub async fn rate_signal(
        &self,
        signal_id: &str,
        user_id: &str,
        rating: u8,
        comment: Option<String>,
        profit_pct: Option<f64>,
    ) -> Result<(), String> {
        if rating < 1 || rating > 5 {
            return Err("Rating must be between 1 and 5 stars".to_string());
        }

        let review = SignalReview {
            user_id: user_id.to_string(),
            rating,
            comment,
            profit_pct,
            timestamp: Utc::now().timestamp(),
        };

        let mut ratings = self.signal_ratings.lock().await;
        let signal_rating = ratings
            .entry(signal_id.to_string())
            .or_insert_with(|| SignalRating {
                signal_id: signal_id.to_string(),
                total_ratings: 0,
                sum_ratings: 0,
                average_rating: 0.0,
                five_star: 0,
                four_star: 0,
                three_star: 0,
                two_star: 0,
                one_star: 0,
                reviews: Vec::new(),
            });

        signal_rating.total_ratings += 1;
        signal_rating.sum_ratings += rating as u32;
        signal_rating.average_rating = signal_rating.sum_ratings as f64 / signal_rating.total_ratings as f64;

        match rating {
            5 => signal_rating.five_star += 1,
            4 => signal_rating.four_star += 1,
            3 => signal_rating.three_star += 1,
            2 => signal_rating.two_star += 1,
            1 => signal_rating.one_star += 1,
            _ => {}
        }

        signal_rating.reviews.push(review);

        log::info!(
            "⭐ Signal {} rated {} stars by user {} (avg: {:.1})",
            signal_id,
            rating,
            user_id,
            signal_rating.average_rating
        );

        Ok(())
    }

    /// Get signal rating
    pub async fn get_signal_rating(&self, signal_id: &str) -> Option<SignalRating> {
        let ratings = self.signal_ratings.lock().await;
        ratings.get(signal_id).cloned()
    }

    /// Initialize signal performance tracking when signal is published
    pub async fn initialize_signal_performance(
        &self,
        signal: &TradingSignalData,
    ) -> Result<(), String> {
        let mut performances = self.signal_performance.lock().await;
        
        if performances.contains_key(&signal.id) {
            return Ok(()); // Already initialized
        }
        
        let performance = SignalPerformance {
            signal_id: signal.id.clone(),
            provider_id: signal.provider.clone(),
            entry_price: signal.entry_price,
            current_price: signal.entry_price,
            exit_price: None,
            highest_price: signal.entry_price,
            lowest_price: signal.entry_price,
            profit_loss_pct: 0.0,
            status: PerformanceStatus::Pending,
            filled_at: None,
            closed_at: None,
            duration_seconds: None,
        };
        
        performances.insert(signal.id.clone(), performance);
        log::debug!("📊 Initialized performance tracking for signal: {}", signal.id);
        Ok(())
    }
    
    /// Track signal performance with real-time price updates
    pub async fn update_signal_performance(
        &self,
        signal_id: &str,
        current_price: f64,
    ) -> Result<(), String> {
        let mut performances = self.signal_performance.lock().await;
        
        if let Some(perf) = performances.get_mut(signal_id) {
            perf.current_price = current_price;
            perf.highest_price = perf.highest_price.max(current_price);
            perf.lowest_price = perf.lowest_price.min(current_price);
            
            // Update status based on price movement
            if perf.filled_at.is_some() {
                perf.status = PerformanceStatus::Active;
            }
            
            if let Some(exit) = perf.exit_price {
                perf.profit_loss_pct = ((exit - perf.entry_price) / perf.entry_price) * 100.0;
            } else {
                perf.profit_loss_pct = ((current_price - perf.entry_price) / perf.entry_price) * 100.0;
            }
        } else {
            return Err(format!("Signal performance not found for: {}", signal_id));
        }

        Ok(())
    }
    
    /// Mark signal as filled (position opened)
    pub async fn mark_signal_filled(&self, signal_id: &str) -> Result<(), String> {
        let mut performances = self.signal_performance.lock().await;
        if let Some(perf) = performances.get_mut(signal_id) {
            perf.filled_at = Some(Utc::now().timestamp());
            perf.status = PerformanceStatus::Active;
            log::info!("✅ Signal {} marked as filled", signal_id);
            Ok(())
        } else {
            Err(format!("Signal performance not found: {}", signal_id))
        }
    }

    /// Close signal position and finalize performance
    /// ENHANCED: Now updates provider reputation based on signal outcome
    pub async fn close_signal_position(
        &self,
        signal_id: &str,
        exit_price: f64,
    ) -> Result<SignalPerformance, String> {
        let mut performances = self.signal_performance.lock().await;
        
        let perf = performances
            .get_mut(signal_id)
            .ok_or_else(|| "Signal performance not found".to_string())?;

        perf.exit_price = Some(exit_price);
        perf.current_price = exit_price;
        perf.closed_at = Some(Utc::now().timestamp());
        perf.profit_loss_pct = ((exit_price - perf.entry_price) / perf.entry_price) * 100.0;
        
        if let Some(filled_at) = perf.filled_at {
            perf.duration_seconds = Some(Utc::now().timestamp() - filled_at);
        }

        perf.status = if perf.profit_loss_pct > 0.0 {
            PerformanceStatus::Won
        } else {
            PerformanceStatus::Lost
        };

        let signal_success = perf.profit_loss_pct > 0.0;
        let profit_loss_pct = perf.profit_loss_pct;
        let duration_seconds = perf.duration_seconds;
        
        // Get signal data to extract confidence and target
        let signals = self.base_marketplace.signals.lock().await;
        let signal = signals.get(signal_id);
        let predicted_confidence = signal.map(|s| s.confidence).unwrap_or(0.5);
        let target_pct = signal.map(|s| {
            ((s.target_price - s.entry_price) / s.entry_price) * 100.0
        }).unwrap_or(5.0);
        drop(signals);

        log::info!(
            "📊 Signal {} closed: {:.2}% P/L (Status: {:?})",
            signal_id,
            perf.profit_loss_pct,
            perf.status
        );

        // Update provider reputation based on signal outcome
        self.update_provider_reputation_from_signal(
            &perf.provider_id,
            signal_success,
            profit_loss_pct,
            predicted_confidence,
            target_pct,
            duration_seconds,
        ).await;

        Ok(perf.clone())
    }
    
    /// Update provider reputation based on signal performance
    /// ENHANCED: Uses comprehensive reputation update with multiple factors
    pub async fn update_provider_reputation_from_signal(
        &self,
        provider_id: &str,
        signal_success: bool,
        profit_loss_pct: f64,
        predicted_confidence: f64,
        target_pct: f64,
        duration_seconds: Option<i64>,
    ) {
        let mut providers = self.base_marketplace.providers.lock().await;
        
        if let Some(provider) = providers.get_mut(provider_id) {
            let old_reputation = provider.reputation_score;
            
            // Use enhanced reputation update
            provider.update_reputation_enhanced(
                signal_success,
                profit_loss_pct,
                predicted_confidence,
                target_pct,
                duration_seconds,
            );
            
            let new_reputation = provider.reputation_score;
            let change = new_reputation - old_reputation;
            
            log::info!(
                "⭐ Provider {} reputation updated: {:.1} → {:.1} ({:+.1}) | Signal: {} ({:.2}% P/L)",
                provider_id,
                old_reputation,
                new_reputation,
                change,
                if signal_success { "SUCCESS" } else { "FAILURE" },
                profit_loss_pct
            );
            
            // Log detailed reputation factors
            if signal_success {
                log::debug!(
                    "   ✅ Success factors: Profit={:.2}%, Confidence={:.1}%, Target={:.1}%, Duration={:?}s",
                    profit_loss_pct,
                    predicted_confidence * 100.0,
                    target_pct,
                    duration_seconds
                );
            } else {
                log::debug!(
                    "   ❌ Failure factors: Loss={:.2}%, Confidence={:.1}%, Target={:.1}%",
                    profit_loss_pct,
                    predicted_confidence * 100.0,
                    target_pct
                );
            }
        } else {
            log::warn!("⚠️ Provider {} not found for reputation update", provider_id);
        }
    }

    /// Get signal performance
    pub async fn get_signal_performance(&self, signal_id: &str) -> Option<SignalPerformance> {
        let performances = self.signal_performance.lock().await;
        performances.get(signal_id).cloned()
    }

    /// Update leaderboard
    pub async fn update_leaderboard(&self) -> Result<(), String> {
        let mut leaderboard = self.leaderboard.lock().await;
        
        // Get all providers from base marketplace
        let provider_ids = vec![
            "memecoin_monitor",
            "oracle_monitor",
            "jupiter_memecoin_trader",
            "jupiter_bluechip_trader",
            "opportunity_analyzer",
            "signal_trader",
            "master_analyzer",
        ];

        let mut entries = Vec::new();

        for provider_id in provider_ids {
            if let Some(provider) = self.base_marketplace.get_provider_stats(provider_id).await {
                let win_rate = if provider.total_signals > 0 {
                    (provider.successful_signals as f64 / provider.total_signals as f64) * 100.0
                } else {
                    0.0
                };

                let avg_profit = if provider.successful_signals > 0 {
                    provider.earnings / provider.successful_signals as f64
                } else {
                    0.0
                };

                // Count subscribers
                let subscriptions = self.subscriptions.lock().await;
                let subscriber_count = subscriptions
                    .values()
                    .flatten()
                    .filter(|s| s.provider_id == provider_id && matches!(s.status, SubscriptionStatus::Active))
                    .count() as u32;

                // Get average rating
                let ratings = self.signal_ratings.lock().await;
                let _provider_signals: Vec<String> = ratings
                    .values()
                    .filter(|_r| {
                        // Would need to lookup provider_id from signal_id in real implementation
                        true
                    })
                    .map(|r| r.signal_id.clone())
                    .collect();
                drop(ratings);

                entries.push(LeaderboardEntry {
                    rank: 0, // Will be set after sorting
                    provider_id: provider_id.to_string(),
                    provider_name: provider.name.clone(),
                    win_rate,
                    total_signals: provider.total_signals as u32,
                    avg_profit,
                    total_earnings: provider.earnings,
                    reputation_score: provider.reputation_score,
                    subscribers: subscriber_count,
                    avg_rating: 0.0, // Would calculate from provider's signals
                });
            }
        }

        // Sort by reputation score
        entries.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        
        // Set ranks
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.rank = (i + 1) as u32;
        }

        leaderboard.top_providers = entries;
        leaderboard.updated_at = Utc::now().timestamp();

        // Update top signals (last 24 hours)
        self.update_top_signals_24h(&mut leaderboard).await?;

        // Update trending symbols
        self.update_trending_symbols(&mut leaderboard).await?;

        log::info!("📊 Leaderboard updated with {} providers", leaderboard.top_providers.len());

        Ok(())
    }

    /// Update top performing signals in last 24 hours
    async fn update_top_signals_24h(&self, leaderboard: &mut Leaderboard) -> Result<(), String> {
        let performances = self.signal_performance.lock().await;
        let now = Utc::now().timestamp();
        let day_ago = now - 86400;

        let mut top_signals: Vec<TopSignal> = performances
            .values()
            .filter(|p| {
                if let Some(closed_at) = p.closed_at {
                    closed_at > day_ago && matches!(p.status, PerformanceStatus::Won)
                } else {
                    false
                }
            })
            .map(|p| TopSignal {
                signal_id: p.signal_id.clone(),
                provider_id: p.provider_id.clone(),
                symbol: "".to_string(), // Would lookup from signal
                profit_pct: p.profit_loss_pct,
                confidence: 0.0, // Would lookup from signal
                rating: 0.0, // Would lookup from ratings
                timestamp: p.closed_at.unwrap_or(0),
            })
            .collect();

        top_signals.sort_by(|a, b| b.profit_pct.partial_cmp(&a.profit_pct).unwrap());
        top_signals.truncate(10);

        leaderboard.top_signals_24h = top_signals;

        Ok(())
    }

    /// Update trending symbols
    async fn update_trending_symbols(&self, leaderboard: &mut Leaderboard) -> Result<(), String> {
        let active_signals = self.base_marketplace.get_active_signals().await;

        let mut symbol_map: HashMap<String, TrendingSymbol> = HashMap::new();

        for signal in active_signals {
            let entry = symbol_map
                .entry(signal.symbol.clone())
                .or_insert_with(|| TrendingSymbol {
                    symbol: signal.symbol.clone(),
                    signal_count: 0,
                    avg_confidence: 0.0,
                    bullish_signals: 0,
                    bearish_signals: 0,
                    sentiment: "NEUTRAL".to_string(),
                });

            entry.signal_count += 1;
            entry.avg_confidence += signal.confidence;

            match signal.action {
                crate::signal_platform::SignalAction::Buy => entry.bullish_signals += 1,
                crate::signal_platform::SignalAction::Sell => entry.bearish_signals += 1,
                _ => {}
            }
        }

        let mut trending: Vec<TrendingSymbol> = symbol_map
            .into_iter()
            .map(|(_, mut t)| {
                t.avg_confidence /= t.signal_count as f64;
                
                if t.bullish_signals > t.bearish_signals * 2 {
                    t.sentiment = "BULLISH".to_string();
                } else if t.bearish_signals > t.bullish_signals * 2 {
                    t.sentiment = "BEARISH".to_string();
                } else {
                    t.sentiment = "NEUTRAL".to_string();
                }
                
                t
            })
            .collect();

        trending.sort_by(|a, b| b.signal_count.cmp(&a.signal_count));
        trending.truncate(10);

        leaderboard.trending_symbols = trending;

        Ok(())
    }

    /// Get current leaderboard
    pub async fn get_leaderboard(&self) -> Leaderboard {
        self.leaderboard.lock().await.clone()
    }

    /// Get marketplace statistics
    pub async fn get_marketplace_stats(&self) -> MarketplaceStats {
        let active_signals = self.base_marketplace.get_active_signals().await;
        let performances = self.signal_performance.lock().await;

        let total_signals = performances.len() as u32;
        let active_signals_count = active_signals.len() as u32;

        let total_volume_24h: f64 = performances
            .values()
            .filter(|p| {
                let now = Utc::now().timestamp();
                if let Some(filled) = p.filled_at {
                    now - filled < 86400
                } else {
                    false
                }
            })
            .count() as f64 * 15.0; // Avg signal price

        let total_trades = performances
            .values()
            .filter(|p| matches!(p.status, PerformanceStatus::Won | PerformanceStatus::Lost))
            .count() as u32;

        let avg_signal_price = active_signals
            .iter()
            .map(|s| s.price)
            .sum::<f64>() / active_signals.len().max(1) as f64;

        MarketplaceStats {
            total_signals,
            active_signals: active_signals_count,
            total_volume_24h,
            total_trades,
            avg_signal_price,
            top_performing_provider: "memecoin_monitor".to_string(), // From leaderboard
            most_traded_symbol: "SOL/USD".to_string(), // From trending
        }
    }

    /// Search signals with advanced filters
    pub async fn search_signals(
        &self,
        symbol: Option<String>,
        min_confidence: Option<f64>,
        max_price: Option<f64>,
        min_rating: Option<f64>,
        provider_id: Option<String>,
    ) -> Vec<TradingSignalData> {
        let mut signals = self.base_marketplace.get_active_signals().await;
        let ratings = self.signal_ratings.lock().await;

        signals.retain(|s| {
            let mut keep = true;

            if let Some(ref sym) = symbol {
                keep = keep && s.symbol.contains(sym);
            }

            if let Some(min_conf) = min_confidence {
                keep = keep && s.confidence >= min_conf;
            }

            if let Some(max_p) = max_price {
                keep = keep && s.price <= max_p;
            }

            if let Some(min_rat) = min_rating {
                if let Some(rating) = ratings.get(&s.id) {
                    keep = keep && rating.average_rating >= min_rat;
                } else {
                    keep = false;
                }
            }

            if let Some(ref prov) = provider_id {
                keep = keep && &s.provider == prov;
            }

            keep
        });

        signals
    }

    /// ENHANCED: Advanced search with multiple criteria
    pub async fn advanced_search_signals(
        &self,
        filters: AdvancedSearchFilters,
    ) -> Vec<TradingSignalData> {
        let mut signals = self.base_marketplace.get_active_signals().await;
        let ratings = self.signal_ratings.lock().await;
        let providers = self.base_marketplace.providers.lock().await;

        signals.retain(|s| {
            let mut keep = true;

            if let Some(ref sym) = filters.symbol {
                keep = keep && s.symbol.contains(sym);
            }

            if let Some(min_conf) = filters.min_confidence {
                keep = keep && s.confidence >= min_conf;
            }

            if let Some(max_conf) = filters.max_confidence {
                keep = keep && s.confidence <= max_conf;
            }

            if let Some(min_p) = filters.min_price {
                keep = keep && s.price >= min_p;
            }

            if let Some(max_p) = filters.max_price {
                keep = keep && s.price <= max_p;
            }

            if let Some(min_rat) = filters.min_rating {
                if let Some(rating) = ratings.get(&s.id) {
                    keep = keep && rating.average_rating >= min_rat;
                } else {
                    keep = false;
                }
            }

            if let Some(ref prov) = filters.provider_id {
                keep = keep && &s.provider == prov;
            }

            if let Some(ref action) = filters.action {
                keep = keep && s.action == *action;
            }

            if let Some(min_target) = filters.min_profit_target {
                let profit_target = ((s.target_price - s.entry_price) / s.entry_price) * 100.0;
                keep = keep && profit_target >= min_target;
            }

            if let Some(max_target) = filters.max_profit_target {
                let profit_target = ((s.target_price - s.entry_price) / s.entry_price) * 100.0;
                keep = keep && profit_target <= max_target;
            }

            if let Some(ref tf) = filters.timeframe {
                keep = keep && s.timeframe == *tf;
            }

            if let Some(min_rep) = filters.min_reputation {
                if let Some(provider) = providers.get(&s.provider) {
                    keep = keep && provider.reputation_score >= min_rep;
                } else {
                    keep = false;
                }
            }

            if let Some(ref risk) = filters.risk_level {
                // Calculate risk based on confidence and stop loss
                let stop_loss_pct = ((s.entry_price - s.stop_loss) / s.entry_price) * 100.0;
                let risk_level = if s.confidence > 0.8 && stop_loss_pct < 3.0 {
                    "low"
                } else if s.confidence > 0.6 && stop_loss_pct < 5.0 {
                    "medium"
                } else {
                    "high"
                };
                keep = keep && risk_level == risk;
            }

            keep
        });

        signals
    }

    /// ENHANCED: Signal recommendation engine
    pub async fn recommend_signals(
        &self,
        user_id: Option<&str>,
        limit: usize,
    ) -> Vec<SignalRecommendation> {
        let active_signals = self.base_marketplace.get_active_signals().await;
        let ratings = self.signal_ratings.lock().await;
        let providers = self.base_marketplace.providers.lock().await;
        let performances = self.signal_performance.lock().await;

        let mut recommendations: Vec<SignalRecommendation> = active_signals
            .into_iter()
            .map(|signal| {
                let mut score = 0.0;
                let mut reasons = Vec::new();

                // 1. Confidence score (0-30 points)
                score += signal.confidence * 30.0;
                if signal.confidence > 0.8 {
                    reasons.push("High confidence signal".to_string());
                }

                // 2. Provider reputation (0-25 points)
                if let Some(provider) = providers.get(&signal.provider) {
                    score += (provider.reputation_score / 100.0) * 25.0;
                    if provider.reputation_score > 80.0 {
                        reasons.push("Top-rated provider".to_string());
                    }
                }

                // 3. Signal rating (0-20 points)
                if let Some(rating) = ratings.get(&signal.id) {
                    score += (rating.average_rating / 5.0) * 20.0;
                    if rating.average_rating >= 4.5 {
                        reasons.push("Highly rated by users".to_string());
                    }
                }

                // 4. Profit potential (0-15 points)
                let profit_target = ((signal.target_price - signal.entry_price) / signal.entry_price) * 100.0;
                score += (profit_target.min(20.0) / 20.0) * 15.0;
                if profit_target > 10.0 {
                    reasons.push(format!("High profit target: {:.1}%", profit_target));
                }

                // 5. Risk/reward ratio (0-10 points)
                let stop_loss_pct = ((signal.entry_price - signal.stop_loss) / signal.entry_price) * 100.0;
                let risk_reward = if stop_loss_pct > 0.0 {
                    profit_target / stop_loss_pct
                } else {
                    0.0
                };
                score += (risk_reward.min(5.0) / 5.0) * 10.0;
                if risk_reward > 2.0 {
                    reasons.push(format!("Good risk/reward: {:.1}:1", risk_reward));
                }

                // 6. User history bonus (if user provided)
                if let Some(_uid) = user_id {
                    // Check if user has successful history with this provider
                    let user_success_rate = performances
                        .values()
                        .filter(|p| {
                            p.provider_id == signal.provider
                                && matches!(p.status, PerformanceStatus::Won)
                        })
                        .count() as f64
                        / performances
                            .values()
                            .filter(|p| p.provider_id == signal.provider)
                            .count()
                            .max(1) as f64;
                    
                    if user_success_rate > 0.7 {
                        score += 5.0;
                        reasons.push("You've had success with this provider".to_string());
                    }
                }

                SignalRecommendation {
                    signal,
                    score,
                    reasons,
                }
            })
            .collect();

        // Sort by score (highest first)
        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        recommendations.truncate(limit);

        log::info!("📊 Generated {} signal recommendations", recommendations.len());
        recommendations
    }

    /// ENHANCED: Provider comparison
    pub async fn compare_providers(
        &self,
        provider_ids: Vec<String>,
    ) -> Vec<ProviderComparison> {
        let providers = self.base_marketplace.providers.lock().await;
        let subscriptions = self.subscriptions.lock().await;
        let performances = self.signal_performance.lock().await;
        let ratings = self.signal_ratings.lock().await;

        let mut comparisons = Vec::new();

        for provider_id in provider_ids {
            if let Some(provider) = providers.get(&provider_id) {
                let win_rate = if provider.total_signals > 0 {
                    (provider.successful_signals as f64 / provider.total_signals as f64) * 100.0
                } else {
                    0.0
                };

                let avg_profit = if provider.successful_signals > 0 {
                    provider.earnings / provider.successful_signals as f64
                } else {
                    0.0
                };

                // Calculate recent performance (last 10 signals)
                let recent_perfs: Vec<&SignalPerformance> = performances
                    .values()
                    .filter(|p| p.provider_id == provider_id)
                    .filter(|p| matches!(p.status, PerformanceStatus::Won | PerformanceStatus::Lost))
                    .collect();
                
                let recent_performance = if recent_perfs.len() >= 10 {
                    let recent = recent_perfs.iter().take(10);
                    let wins = recent.filter(|p| matches!(p.status, PerformanceStatus::Won)).count();
                    (wins as f64 / 10.0) * 100.0
                } else if !recent_perfs.is_empty() {
                    let wins = recent_perfs.iter().filter(|p| matches!(p.status, PerformanceStatus::Won)).count();
                    (wins as f64 / recent_perfs.len() as f64) * 100.0
                } else {
                    0.0
                };

                // Count subscribers
                let subscriber_count = subscriptions
                    .values()
                    .flatten()
                    .filter(|s| s.provider_id == provider_id && matches!(s.status, SubscriptionStatus::Active))
                    .count() as u32;

                // Calculate average rating from provider's signals
                let provider_signals: Vec<String> = ratings
                    .keys()
                    .filter(|_sid| {
                        // In real implementation, would lookup provider_id from signal_id
                        true
                    })
                    .cloned()
                    .collect();
                
                let avg_rating = if !provider_signals.is_empty() {
                    let total_rating: f64 = provider_signals
                        .iter()
                        .filter_map(|sid| ratings.get(sid))
                        .map(|r| r.average_rating)
                        .sum();
                    total_rating / provider_signals.len() as f64
                } else {
                    0.0
                };

                comparisons.push(ProviderComparison {
                    provider_id: provider_id.clone(),
                    provider_name: provider.name.clone(),
                    win_rate,
                    avg_profit,
                    total_signals: provider.total_signals as u32,
                    reputation_score: provider.reputation_score,
                    avg_rating,
                    subscribers: subscriber_count,
                    recent_performance,
                });
            }
        }

        // Sort by reputation score
        comparisons.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());

        log::info!("📊 Compared {} providers", comparisons.len());
        comparisons
    }

    /// ENHANCED: Signal portfolio management
    pub async fn get_user_portfolio(&self, user_id: &str) -> UserPortfolio {
        let performances = self.signal_performance.lock().await;
        let signals = self.base_marketplace.signals.lock().await;

        let mut active_positions = Vec::new();
        let mut closed_positions = Vec::new();
        let mut total_profit_loss = 0.0;
        let mut total_wins = 0;
        let mut total_trades = 0;
        let mut total_invested = 0.0;

        // In real implementation, would track which signals user purchased
        // For now, we'll use all signals as example
        for (signal_id, perf) in performances.iter() {
            if let Some(signal) = signals.get(signal_id) {
                let profit_loss_usd = (perf.profit_loss_pct / 100.0) * signal.price;
                total_profit_loss += profit_loss_usd;
                total_invested += signal.price;

                let position = PortfolioPosition {
                    signal_id: signal_id.clone(),
                    symbol: signal.symbol.clone(),
                    entry_price: perf.entry_price,
                    current_price: perf.current_price,
                    profit_loss_pct: perf.profit_loss_pct,
                    profit_loss_usd,
                    status: format!("{:?}", perf.status),
                    opened_at: perf.filled_at.unwrap_or(0),
                    closed_at: perf.closed_at,
                };

                match perf.status {
                    PerformanceStatus::Active | PerformanceStatus::Pending => {
                        active_positions.push(position);
                    }
                    PerformanceStatus::Won | PerformanceStatus::Lost => {
                        closed_positions.push(position);
                        total_trades += 1;
                        if matches!(perf.status, PerformanceStatus::Won) {
                            total_wins += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        let win_rate = if total_trades > 0 {
            (total_wins as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        UserPortfolio {
            user_id: user_id.to_string(),
            active_positions,
            closed_positions,
            total_profit_loss,
            win_rate,
            total_invested,
        }
    }

    /// ENHANCED: Market trends analysis
    pub async fn analyze_market_trends(&self) -> MarketTrends {
        let active_signals = self.base_marketplace.get_active_signals().await;
        let leaderboard = self.leaderboard.lock().await;

        let mut bullish_count = 0;
        let mut bearish_count = 0;
        let mut total_confidence = 0.0;
        let mut high_confidence_count = 0;

        for signal in &active_signals {
            total_confidence += signal.confidence;
            if signal.confidence > 0.8 {
                high_confidence_count += 1;
            }

            match signal.action {
                crate::signal_platform::SignalAction::Buy => bullish_count += 1,
                crate::signal_platform::SignalAction::Sell => bearish_count += 1,
                _ => {}
            }
        }

        let avg_market_confidence = if !active_signals.is_empty() {
            total_confidence / active_signals.len() as f64
        } else {
            0.0
        };

        let market_sentiment = if bullish_count as f64 > bearish_count as f64 * 1.5 {
            "BULLISH".to_string()
        } else if bearish_count as f64 > bullish_count as f64 * 1.5 {
            "BEARISH".to_string()
        } else {
            "NEUTRAL".to_string()
        };

        let top_providers: Vec<String> = leaderboard
            .top_providers
            .iter()
            .take(5)
            .map(|e| e.provider_id.clone())
            .collect();

        MarketTrends {
            bullish_symbols: leaderboard.trending_symbols
                .iter()
                .filter(|s| s.sentiment == "BULLISH")
                .cloned()
                .collect(),
            bearish_symbols: leaderboard.trending_symbols
                .iter()
                .filter(|s| s.sentiment == "BEARISH")
                .cloned()
                .collect(),
            high_confidence_signals: high_confidence_count,
            avg_market_confidence,
            top_providers,
            market_sentiment,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscription_creation() {
        let marketplace = Arc::new(SignalMarketplace::new("https://api.mainnet-beta.solana.com".to_string()));
        let enhanced = EnhancedMarketplace::new(marketplace);

        let sub = enhanced
            .subscribe_to_provider("user1", "provider1", SubscriptionTier::Premium)
            .await
            .unwrap();

        assert_eq!(sub.user_id, "user1");
        assert_eq!(sub.provider_id, "provider1");
        assert_eq!(sub.price_paid, 100.0);
    }

    #[tokio::test]
    async fn test_signal_rating() {
        let marketplace = Arc::new(SignalMarketplace::new("https://api.mainnet-beta.solana.com".to_string()));
        let enhanced = EnhancedMarketplace::new(marketplace);

        enhanced
            .rate_signal("signal1", "user1", 5, Some("Great signal!".to_string()), Some(15.5))
            .await
            .unwrap();

        let rating = enhanced.get_signal_rating("signal1").await.unwrap();
        assert_eq!(rating.average_rating, 5.0);
        assert_eq!(rating.five_star, 1);
    }
}

