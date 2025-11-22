//! X402 Signal Platform - Protocol for trading signal marketplace
//! Integrated into AI orchestrator for signal sharing and monetization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::switchboard_oracle::SwitchboardClient;
use crate::dex_screener::DexScreenerClient;
use crate::pumpfun::PumpFunClient;

/// Trading signal that can be shared/traded on the platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignalData {
    pub id: String,
    pub provider: String,
    pub symbol: String,
    pub action: SignalAction,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub confidence: f64,
    pub timeframe: String,
    pub data_sources: Vec<String>,
    pub analysis: String,
    pub timestamp: i64,
    pub expiry: i64,
    pub price: f64, // Price to buy/sell this signal (in tokens)
    pub status: SignalStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignalAction {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalStatus {
    Active,
    Executing,  // FIX #1: Added to prevent duplicate execution
    Filled,
    Expired,
    Cancelled,
}

/// X402 Protocol message for signal trading
/// X402 is a protocol for automated signal exchange between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Message {
    pub protocol_version: String,
    pub message_type: X402MessageType,
    pub sender_id: String,
    pub receiver_id: Option<String>,
    pub timestamp: i64,
    pub payload: X402Payload,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum X402MessageType {
    SignalOffer,
    SignalRequest,
    SignalPurchase,
    SignalConfirmation,
    SignalUpdate,
    SignalExpiry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum X402Payload {
    Signal(TradingSignalData),
    Request { symbols: Vec<String>, max_price: f64 },
    Purchase { signal_id: String, payment: f64 },
    Confirmation { signal_id: String, status: String },
    Update { signal_id: String, new_data: HashMap<String, String> },
}

/// Signal provider that generates and sells signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalProvider {
    pub id: String,
    pub name: String,
    pub reputation_score: f64,
    pub total_signals: u64,
    pub successful_signals: u64,
    pub earnings: f64,
}

impl SignalProvider {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            reputation_score: 50.0,
            total_signals: 0,
            successful_signals: 0,
            earnings: 0.0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_signals == 0 {
            0.0
        } else {
            (self.successful_signals as f64 / self.total_signals as f64) * 100.0
        }
    }

    /// Enhanced reputation update with multiple factors
    /// Updates reputation based on signal outcome with profit percentage, confidence accuracy, and timing
    pub fn update_reputation_enhanced(
        &mut self,
        signal_success: bool,
        profit_loss_pct: f64,
        predicted_confidence: f64,
        target_pct: f64,
        duration_seconds: Option<i64>,
    ) {
        self.total_signals += 1;
        
        if signal_success {
            self.successful_signals += 1;
        }
        
        // Calculate reputation change based on multiple factors
        let mut reputation_change = 0.0;
        
        // 1. Base success/failure impact
        if signal_success {
            reputation_change += 5.0; // Base success bonus
        } else {
            reputation_change -= 2.0; // Base failure penalty
        }
        
        // 2. Profit percentage impact (scaled)
        // Higher profits = more reputation gain, larger losses = more reputation loss
        if signal_success {
            // Success: reward based on profit percentage
            // 5% profit = +2.5, 10% profit = +5.0, 20% profit = +10.0 (capped)
            let profit_bonus = (profit_loss_pct / 2.0).min(10.0).max(0.0);
            reputation_change += profit_bonus;
        } else {
            // Failure: penalty based on loss percentage
            // -5% loss = -2.5, -10% loss = -5.0, -20% loss = -10.0 (capped)
            let loss_penalty = (profit_loss_pct.abs() / 2.0).min(10.0).max(0.0);
            reputation_change -= loss_penalty;
        }
        
        // 3. Confidence accuracy bonus/penalty
        // If high confidence and success = bonus, if high confidence and failure = penalty
        let confidence_accuracy = if signal_success {
            // Success with high confidence = bonus
            predicted_confidence * 2.0
        } else {
            // Failure with high confidence = penalty (overconfidence)
            -predicted_confidence * 3.0
        };
        reputation_change += confidence_accuracy;
        
        // 4. Target achievement bonus
        // If profit exceeds target significantly, bonus
        if signal_success && profit_loss_pct > target_pct {
            let target_exceed_bonus = ((profit_loss_pct - target_pct) / target_pct * 5.0).min(5.0);
            reputation_change += target_exceed_bonus;
        }
        
        // 5. Timing bonus (faster target achievement = better)
        if signal_success {
            if let Some(duration) = duration_seconds {
                // Faster execution = bonus (target reached quickly)
                // 10 minutes = +2.0, 20 minutes = +1.0, 30+ minutes = +0.5
                let timing_bonus = if duration < 600 {
                    2.0 // Under 10 minutes
                } else if duration < 1200 {
                    1.0 // Under 20 minutes
                } else if duration < 1800 {
                    0.5 // Under 30 minutes
                } else {
                    0.0 // Over 30 minutes
                };
                reputation_change += timing_bonus;
            }
        }
        
        // 6. Consistency bonus (recent performance vs historical)
        // If success rate is improving, bonus
        let recent_success_rate = if self.total_signals > 10 {
            let recent_signals = self.total_signals.min(20);
            let recent_successes = self.successful_signals.min(recent_signals);
            recent_successes as f64 / recent_signals as f64
        } else {
            0.5 // Default for new providers
        };
        
        let historical_success_rate = if self.total_signals > 0 {
            self.successful_signals as f64 / self.total_signals as f64
        } else {
            0.5
        };
        
        // If recent performance is better than historical, bonus
        if recent_success_rate > historical_success_rate + 0.1 {
            reputation_change += 1.0; // Consistency bonus
        }
        
        // Apply reputation change with exponential moving average
        // Use 0.95 decay factor (recent performance weighted more)
        self.reputation_score = (self.reputation_score * 0.95 + reputation_change).min(100.0).max(0.0);
        }
    
    /// Legacy simple reputation update (for backward compatibility)
    pub fn update_reputation(&mut self, signal_success: bool) {
        self.update_reputation_enhanced(
            signal_success,
            if signal_success { 5.0 } else { -5.0 }, // Default profit/loss
            0.5, // Default confidence
            5.0, // Default target
            None, // No duration
        );
    }

    pub fn add_earnings(&mut self, amount: f64) {
        self.earnings += amount;
    }
}

/// Signal marketplace for trading signals using X402 protocol
pub struct SignalMarketplace {
    pub signals: Arc<Mutex<HashMap<String, TradingSignalData>>>,
    pub providers: Arc<Mutex<HashMap<String, SignalProvider>>>,
    pub subscriptions: Arc<Mutex<HashMap<String, Vec<String>>>>, // user_id -> signal_ids
    oracle_client: Arc<SwitchboardClient>,
    dex_client: Arc<DexScreenerClient>,
    pumpfun_client: Arc<PumpFunClient>,
}

impl SignalMarketplace {
    pub fn new(rpc_url: String) -> Self {
        Self {
            signals: Arc::new(Mutex::new(HashMap::new())),
            providers: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            oracle_client: Arc::new(SwitchboardClient::new(rpc_url.clone(), std::env::var("SOLANA_RPC_URL").is_ok())),
            dex_client: Arc::new(DexScreenerClient::new()),
            pumpfun_client: Arc::new(PumpFunClient::new()),
        }
    }

    /// Register a signal provider
    pub async fn register_provider(&self, id: String, name: String) -> Result<(), String> {
        let mut providers = self.providers.lock().await;
        if providers.contains_key(&id) {
            return Err("Provider already registered".to_string());
        }
        providers.insert(id.clone(), SignalProvider::new(id, name));
        log::info!("✅ Registered new signal provider");
        Ok(())
    }

    /// Publish a signal to the marketplace
    /// Signals published here are:
    /// 1. Available for autonomous execution (if confidence ≥75%)
    /// 2. Available for purchase by other agents
    /// 3. Tracked for performance metrics
    pub async fn publish_signal(&self, signal: TradingSignalData) -> Result<String, String> {
        let mut signals = self.signals.lock().await;
        
        if signals.contains_key(&signal.id) {
            return Err("Signal ID already exists".to_string());
        }

        let signal_id = signal.id.clone();
        let signal_clone = signal.clone();
        let is_executable = signal_clone.confidence >= 0.75 && 
                           signal_clone.expiry > chrono::Utc::now().timestamp();
        
        signals.insert(signal_id.clone(), signal);
        
        log::info!("📡 Published signal to marketplace: {} | Symbol: {} | Confidence: {:.1}% | Price: {} tokens", 
                  signal_id, signal_clone.symbol, signal_clone.confidence * 100.0, signal_clone.price);
        
        if is_executable {
            log::info!("   ✅ Signal qualifies for autonomous execution (confidence ≥75%)");
        } else {
            log::debug!("   ⏸️  Signal below auto-execution threshold (confidence <75%)");
        }
        
        Ok(signal_id)
    }

    /// Generate signals from all data sources
    pub async fn generate_signals(&self, provider_id: &str) -> Result<Vec<TradingSignalData>, String> {
        let mut signals = Vec::new();

        // Generate signals from Switchboard Oracle
        match self.generate_oracle_signals(provider_id).await {
            Ok(oracle_signals) => signals.extend(oracle_signals),
            Err(e) => log::warn!("Oracle signal generation failed: {}", e),
        }

        // Generate signals from DEX Screener
        match self.generate_dex_signals(provider_id).await {
            Ok(dex_signals) => signals.extend(dex_signals),
            Err(e) => log::warn!("DEX signal generation failed: {}", e),
        }

        // Generate signals from PumpFun
        match self.generate_meme_signals(provider_id).await {
            Ok(meme_signals) => signals.extend(meme_signals),
            Err(e) => log::warn!("Meme signal generation failed: {}", e),
        }

        log::info!("🎯 Generated {} signals from all sources", signals.len());
        Ok(signals)
    }

    /// Generate signals from Switchboard Oracle data
    async fn generate_oracle_signals(&self, provider_id: &str) -> Result<Vec<TradingSignalData>, String> {
        let symbols = vec!["SOL/USD".to_string(), "BTC/USD".to_string(), "ETH/USD".to_string()];
        let feeds = self.oracle_client.fetch_multiple_feeds(&symbols).await
            .map_err(|e| format!("Oracle fetch error: {}", e))?;
        
        let mut signals = Vec::new();
        
        for feed in feeds {
            let change = SwitchboardClient::calculate_price_change(
                feed.min_price, // Use real min price from oracle confidence interval
                feed.price
            );
            
            if change.abs() > 2.0 { // More than 2% change
                let action = if change > 0.0 {
                    SignalAction::Buy
                } else {
                    SignalAction::Sell
                };
                
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: provider_id.to_string(),
                    symbol: feed.symbol.clone(),
                    action,
                    entry_price: feed.price,
                    target_price: if change > 0.0 { feed.price * 1.05 } else { feed.price * 0.95 },
                    stop_loss: if change > 0.0 { feed.price * 0.97 } else { feed.price * 1.03 },
                    confidence: (1.0 - feed.confidence).min(0.95),
                    timeframe: "1h".to_string(),
                    data_sources: vec!["Switchboard Oracle".to_string()],
                    analysis: format!("Price movement of {:.2}% detected via oracle", change),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 3600, // 1 hour expiry
                    price: 10.0, // 10 tokens to buy this signal
                    status: SignalStatus::Active,
                };
                
                signals.push(signal);
            }
        }
        
        Ok(signals)
    }

    /// Generate signals from DEX Screener opportunities
    async fn generate_dex_signals(&self, provider_id: &str) -> Result<Vec<TradingSignalData>, String> {
        let opportunities = self.dex_client.get_top_opportunities(5).await
            .map_err(|e| format!("DEX fetch error: {}", e))?;
        
        let mut signals = Vec::new();
        
        for opp in opportunities {
            if opp.opportunity_score > 60.0 {
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: provider_id.to_string(),
                    symbol: opp.token_symbol.clone(),
                    action: SignalAction::Buy,
                    entry_price: opp.price_usd,
                    target_price: opp.price_usd * 1.15,
                    stop_loss: opp.price_usd * 0.90,
                    confidence: opp.opportunity_score / 100.0,
                    timeframe: "4h".to_string(),
                    data_sources: vec!["DEX Screener".to_string()],
                    analysis: format!("Opportunity score: {:.1}, Signals: {}", 
                        opp.opportunity_score, opp.signals.join(", ")),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 14400, // 4 hours expiry
                    price: 15.0,
                    status: SignalStatus::Active,
                };
                
                signals.push(signal);
            }
        }
        
        Ok(signals)
    }

    /// Generate signals from PumpFun meme coins
    async fn generate_meme_signals(&self, provider_id: &str) -> Result<Vec<TradingSignalData>, String> {
        let meme_signals = self.pumpfun_client.get_top_opportunities(5).await
            .map_err(|e| format!("PumpFun fetch error: {}", e))?;
        
        let mut signals = Vec::new();
        
        for meme in meme_signals {
            if meme.confidence > 0.6 {
                let action = match meme.action.as_str() {
                    "BUY" => SignalAction::Buy,
                    "SELL" => SignalAction::Sell,
                    _ => SignalAction::Hold,
                };
                
                let signal = TradingSignalData {
                    id: uuid::Uuid::new_v4().to_string(),
                    provider: provider_id.to_string(),
                    symbol: meme.symbol.clone(),
                    action,
                    entry_price: meme.entry_price,
                    target_price: meme.target_price,
                    stop_loss: meme.stop_loss,
                    confidence: meme.confidence,
                    timeframe: "15m".to_string(),
                    data_sources: vec!["PumpFun".to_string()],
                    analysis: format!("Meme coin: {}, Reasons: {}", 
                        meme.name, meme.reasons.join(", ")),
                    timestamp: Utc::now().timestamp(),
                    expiry: Utc::now().timestamp() + 900, // 15 min expiry
                    price: 20.0, // Higher price for meme signals
                    status: SignalStatus::Active,
                };
                
                signals.push(signal);
            }
        }
        
        Ok(signals)
    }

    /// Get all active signals
    pub async fn get_active_signals(&self) -> Vec<TradingSignalData> {
        let signals = self.signals.lock().await;
        let now = Utc::now().timestamp();
        
        signals.values()
            .filter(|s| matches!(s.status, SignalStatus::Active) && s.expiry > now)
            .cloned()
            .collect()
    }

    /// Get signals by symbol
    pub async fn get_signals_by_symbol(&self, symbol: &str) -> Vec<TradingSignalData> {
        let signals = self.signals.lock().await;
        let now = Utc::now().timestamp();
        
        signals.values()
            .filter(|s| s.symbol == symbol && matches!(s.status, SignalStatus::Active) && s.expiry > now)
            .cloned()
            .collect()
    }

    /// Purchase a signal using X402 protocol
    pub async fn purchase_signal(&self, user_id: &str, signal_id: &str, payment: f64) -> Result<X402Message, String> {
        let mut signals = self.signals.lock().await;
        let mut providers = self.providers.lock().await;
        let mut subscriptions = self.subscriptions.lock().await;
        
        let signal = signals.get_mut(signal_id)
            .ok_or("Signal not found")?;
        
        if !matches!(signal.status, SignalStatus::Active) {
            return Err("Signal is not active".to_string());
        }
        
        if payment < signal.price {
            return Err("Insufficient payment".to_string());
        }
        
        // Update provider earnings
        if let Some(provider) = providers.get_mut(&signal.provider) {
            provider.add_earnings(payment);
        }
        
        // Add to user subscriptions
        subscriptions.entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(signal_id.to_string());
        
        // Create X402 confirmation message
        let message = X402Message {
            protocol_version: "1.0".to_string(),
            message_type: X402MessageType::SignalConfirmation,
            sender_id: "marketplace".to_string(),
            receiver_id: Some(user_id.to_string()),
            timestamp: Utc::now().timestamp(),
            payload: X402Payload::Confirmation {
                signal_id: signal_id.to_string(),
                status: "purchased".to_string(),
            },
            signature: None,
        };
        
        log::info!("💰 Signal {} purchased by user {}", signal_id, user_id);
        
        Ok(message)
    }

    /// Create X402 message for signal offer
    pub fn create_signal_offer(&self, signal: TradingSignalData, provider_id: &str) -> X402Message {
        X402Message {
            protocol_version: "1.0".to_string(),
            message_type: X402MessageType::SignalOffer,
            sender_id: provider_id.to_string(),
            receiver_id: None, // Broadcast to all
            timestamp: Utc::now().timestamp(),
            payload: X402Payload::Signal(signal),
            signature: None,
        }
    }

    /// Process X402 message
    pub async fn process_x402_message(&self, message: X402Message) -> Result<Option<X402Message>, String> {
        match message.message_type {
            X402MessageType::SignalOffer => {
                if let X402Payload::Signal(signal) = message.payload {
                    self.publish_signal(signal).await?;
                    Ok(None)
                } else {
                    Err("Invalid payload for SignalOffer".to_string())
                }
            }
            X402MessageType::SignalRequest => {
                if let X402Payload::Request { symbols, max_price } = message.payload {
                    let mut matching_signals = Vec::new();
                    let signals = self.signals.lock().await;
                    
                    for signal in signals.values() {
                        if symbols.contains(&signal.symbol) && signal.price <= max_price {
                            matching_signals.push(signal.clone());
                        }
                    }
                    
                    log::info!("🔍 Found {} matching signals for request", matching_signals.len());
                    Ok(None)
                } else {
                    Err("Invalid payload for SignalRequest".to_string())
                }
            }
            X402MessageType::SignalPurchase => {
                if let X402Payload::Purchase { signal_id, payment } = message.payload {
                    let confirmation = self.purchase_signal(
                        &message.sender_id,
                        &signal_id,
                        payment
                    ).await?;
                    Ok(Some(confirmation))
                } else {
                    Err("Invalid payload for SignalPurchase".to_string())
                }
            }
            _ => Ok(None),
        }
    }

    /// Get provider statistics
    pub async fn get_provider_stats(&self, provider_id: &str) -> Option<SignalProvider> {
        let providers = self.providers.lock().await;
        providers.get(provider_id).cloned()
    }
    
    /// Get all registered providers
    pub async fn get_all_providers(&self) -> Vec<SignalProvider> {
        let providers = self.providers.lock().await;
        providers.values().cloned().collect()
    }

    /// Get marketplace statistics
    pub async fn get_marketplace_stats(&self) -> HashMap<String, String> {
        let signals = self.signals.lock().await;
        let providers = self.providers.lock().await;
        let subscriptions = self.subscriptions.lock().await;
        
        let now = Utc::now().timestamp();
        let active_signals = signals.values()
            .filter(|s| matches!(s.status, SignalStatus::Active) && s.expiry > now)
            .count();
        
        let mut stats = HashMap::new();
        stats.insert("total_signals".to_string(), signals.len().to_string());
        stats.insert("active_signals".to_string(), active_signals.to_string());
        stats.insert("total_providers".to_string(), providers.len().to_string());
        stats.insert("total_subscriptions".to_string(), subscriptions.len().to_string());
        stats.insert("protocol_version".to_string(), "X402-1.0".to_string());
        
        stats
    }

    /// Clean up expired signals and remove old ones to prevent memory leaks
    pub async fn cleanup_expired_signals(&self) {
        let mut signals = self.signals.lock().await;
        let now = Utc::now().timestamp();
        const CLEANUP_DELAY_SECS: i64 = 86400; // Keep signals for 24 hours after expiry
        
        // First pass: Mark expired signals and collect IDs of old signals to remove
        let mut to_remove = Vec::new();
        let mut expired_count = 0u32;
        
        for (_id, signal) in signals.iter_mut() {
            // Mark active signals that have expired
            if signal.expiry < now && matches!(signal.status, SignalStatus::Active) {
                signal.status = SignalStatus::Expired;
                expired_count += 1;
            }
        }
        
        // Second pass: Collect IDs of signals to remove (after marking is done)
        for (id, signal) in signals.iter() {
            // Remove signals that expired more than CLEANUP_DELAY_SECS ago
            // Keep Filled signals for historical tracking (they're valuable for RL learning)
            // Only remove Expired signals after delay
            if matches!(signal.status, SignalStatus::Expired) && 
               signal.expiry < now - CLEANUP_DELAY_SECS {
                to_remove.push(id.clone());
            }
        }
        
        // Remove old expired signals
        let removed_count = to_remove.len();
        for id in to_remove {
            signals.remove(&id);
        }
        
        if removed_count > 0 {
            log::info!("🧹 Cleaned up {} expired signals (older than 24h)", removed_count);
        } else if expired_count > 0 {
            log::debug!("🧹 Marked {} signals as expired (will be removed after 24h)", expired_count);
        } else {
            log::debug!("🧹 No expired signals to clean up");
        }
    }
    
    /// Get high-confidence signals ready for auto-execution
    /// Returns signals with confidence >= threshold that are still active
    /// FIX #1: Only returns Active signals (not Executing, Filled, etc.)
    pub async fn get_executable_signals(&self, min_confidence: f64) -> Vec<TradingSignalData> {
        let signals = self.signals.lock().await;
        let now = Utc::now().timestamp();
        
        signals.values()
            .filter(|s| {
                matches!(s.status, SignalStatus::Active)
                    && s.expiry > now
                    && s.confidence >= min_confidence
            })
            .cloned()
            .collect()
    }
    
    /// FIX #1: Atomically mark signal as Executing (returns false if already Executing/Filled)
    /// This prevents duplicate execution by multiple tasks
    pub async fn try_mark_executing(&self, signal_id: &str) -> Result<bool, String> {
        let mut signals = self.signals.lock().await;
        if let Some(signal) = signals.get_mut(signal_id) {
            // Only allow transition from Active to Executing
            if matches!(signal.status, SignalStatus::Active) {
                signal.status = SignalStatus::Executing;
                log::info!("🔒 Signal {} marked as Executing (atomic lock)", signal_id);
                Ok(true)
            } else {
                log::warn!("⚠️ Signal {} cannot be marked Executing - current status: {:?}", signal_id, signal.status);
                Ok(false)
            }
        } else {
            Err(format!("Signal {} not found", signal_id))
        }
    }
    
    /// Update signal status with validation for valid transitions
    pub async fn update_signal_status(&self, signal_id: &str, new_status: SignalStatus) -> Result<(), String> {
        let mut signals = self.signals.lock().await;
        if let Some(signal) = signals.get_mut(signal_id) {
            let old_status = signal.status.clone();
            
            // Validate status transition
            let is_valid_transition = match (&old_status, &new_status) {
                // Valid transitions
                (SignalStatus::Active, SignalStatus::Executing) => true,
                (SignalStatus::Active, SignalStatus::Expired) => true,
                (SignalStatus::Active, SignalStatus::Cancelled) => true,
                (SignalStatus::Executing, SignalStatus::Filled) => true,
                (SignalStatus::Executing, SignalStatus::Active) => true, // On failure, revert
                // Invalid transitions
                (SignalStatus::Filled, _) => false, // Can't change filled signals
                (SignalStatus::Expired, _) => false, // Can't change expired signals
                (SignalStatus::Cancelled, _) => false, // Can't change cancelled signals
                (_, SignalStatus::Executing) => false, // Can only go to Executing from Active
                _ => false,
            };
            
            if !is_valid_transition {
                return Err(format!(
                    "Invalid status transition: {:?} -> {:?} for signal {}",
                    old_status, new_status, signal_id
                ));
            }
            
            signal.status = new_status.clone();
            log::info!("📊 Signal {} status updated: {:?} -> {:?}", signal_id, old_status, new_status);
            Ok(())
        } else {
            Err(format!("Signal {} not found", signal_id))
        }
    }
    
    /// Get signal by ID
    pub async fn get_signal(&self, signal_id: &str) -> Option<TradingSignalData> {
        let signals = self.signals.lock().await;
        signals.get(signal_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_marketplace_creation() {
        let marketplace = SignalMarketplace::new("https://api.mainnet-beta.solana.com".to_string());
        let stats = marketplace.get_marketplace_stats().await;
        assert!(stats.contains_key("total_signals"));
    }

    #[tokio::test]
    async fn test_provider_registration() {
        let marketplace = SignalMarketplace::new("https://api.mainnet-beta.solana.com".to_string());
        let result = marketplace.register_provider("provider1".to_string(), "Test Provider".to_string()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_reputation() {
        let mut provider = SignalProvider::new("test".to_string(), "Test".to_string());
        assert_eq!(provider.success_rate(), 0.0);
        
        provider.update_reputation(true);
        assert_eq!(provider.total_signals, 1);
        assert_eq!(provider.successful_signals, 1);
        assert_eq!(provider.success_rate(), 100.0);
    }

    #[test]
    fn test_x402_message_creation() {
        let signal = TradingSignalData {
            id: "test123".to_string(),
            provider: "provider1".to_string(),
            symbol: "SOL/USD".to_string(),
            action: SignalAction::Buy,
            entry_price: 100.0,
            target_price: 110.0,
            stop_loss: 95.0,
            confidence: 0.8,
            timeframe: "1h".to_string(),
            data_sources: vec!["Oracle".to_string()],
            analysis: "Test signal".to_string(),
            timestamp: Utc::now().timestamp(),
            expiry: Utc::now().timestamp() + 3600,
            price: 10.0,
            status: SignalStatus::Active,
        };
        
        let marketplace = SignalMarketplace::new("https://api.mainnet-beta.solana.com".to_string());
        let message = marketplace.create_signal_offer(signal, "provider1");
        
        assert_eq!(message.protocol_version, "1.0");
        assert!(matches!(message.message_type, X402MessageType::SignalOffer));
    }
}
