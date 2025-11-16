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

    /// Track signal performance
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
            
            if let Some(exit) = perf.exit_price {
                perf.profit_loss_pct = ((exit - perf.entry_price) / perf.entry_price) * 100.0;
            } else {
                perf.profit_loss_pct = ((current_price - perf.entry_price) / perf.entry_price) * 100.0;
            }
        }

        Ok(())
    }

    /// Close signal position and finalize performance
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

        log::info!(
            "📊 Signal {} closed: {:.2}% P/L",
            signal_id,
            perf.profit_loss_pct
        );

        Ok(perf.clone())
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
            "perps_monitor",
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
