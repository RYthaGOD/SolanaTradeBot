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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalAction {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalStatus {
    Active,
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

    pub fn update_reputation(&mut self, signal_success: bool) {
        self.total_signals += 1;
        if signal_success {
            self.successful_signals += 1;
            self.reputation_score = (self.reputation_score * 0.95 + 5.0).min(100.0);
        } else {
            self.reputation_score = (self.reputation_score * 0.95 - 2.0).max(0.0);
        }
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
    pub async fn publish_signal(&self, signal: TradingSignalData) -> Result<String, String> {
        let mut signals = self.signals.lock().await;
        
        if signals.contains_key(&signal.id) {
            return Err("Signal ID already exists".to_string());
        }

        let signal_id = signal.id.clone();
        signals.insert(signal_id.clone(), signal);
        
        log::info!("📡 Published signal: {}", signal_id);
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
                feed.price * 0.98, // Simulate previous price
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

    /// Clean up expired signals
    pub async fn cleanup_expired_signals(&self) {
        let mut signals = self.signals.lock().await;
        let now = Utc::now().timestamp();
        
        let expired: Vec<String> = signals.iter()
            .filter(|(_, s)| s.expiry < now && matches!(s.status, SignalStatus::Active))
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired {
            if let Some(signal) = signals.get_mut(&id) {
                signal.status = SignalStatus::Expired;
            }
        }
        
        log::debug!("🧹 Cleaned up expired signals");
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
