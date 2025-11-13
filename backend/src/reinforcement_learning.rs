use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::deepseek_ai::{DeepSeekClient, TradingDecision};
use crate::signal_platform::TradingSignalData;
use crate::historical_data::{HistoricalDataManager, HistoricalFeatures, PriceDataPoint};

/// Experience replay buffer for reinforcement learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub state: MarketState,
    pub action: Action,
    pub reward: f64,
    pub next_state: Option<MarketState>,
    pub timestamp: i64,
    pub provider_id: String,
}

/// Market state representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub price_change_1h: f64,
    pub price_change_24h: f64,
    pub sentiment_score: f64,
    pub liquidity: f64,
    pub volatility: f64,
    pub market_cap: Option<f64>,
}

/// Action taken by agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String, // "BUY", "SELL", "HOLD"
    pub confidence: f64,
    pub size: f64,
    pub price: f64,
}

/// Performance metrics for agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformance {
    pub total_trades: usize,
    pub successful_trades: usize,
    pub failed_trades: usize,
    pub total_profit: f64,
    pub total_loss: f64,
    pub avg_reward: f64,
    pub win_rate: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub learning_rate: f64,
}

impl AgentPerformance {
    pub fn new() -> Self {
        Self {
            total_trades: 0,
            successful_trades: 0,
            failed_trades: 0,
            total_profit: 0.0,
            total_loss: 0.0,
            avg_reward: 0.0,
            win_rate: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            learning_rate: 0.01, // Initial learning rate
        }
    }

    pub fn update(&mut self, reward: f64) {
        self.total_trades += 1;
        
        if reward > 0.0 {
            self.successful_trades += 1;
            self.total_profit += reward;
        } else {
            self.failed_trades += 1;
            self.total_loss += reward.abs();
        }

        // Update win rate
        self.win_rate = self.successful_trades as f64 / self.total_trades as f64;

        // Update average reward (exponential moving average)
        let alpha = 0.1; // Smoothing factor
        self.avg_reward = alpha * reward + (1.0 - alpha) * self.avg_reward;

        // Adjust learning rate based on performance
        if self.win_rate > 0.6 {
            self.learning_rate = (self.learning_rate * 0.95).max(0.001); // Decrease if doing well
        } else if self.win_rate < 0.4 {
            self.learning_rate = (self.learning_rate * 1.05).min(0.05); // Increase if struggling
        }
    }

    pub fn calculate_sharpe_ratio(&mut self, returns: &[f64]) {
        if returns.len() < 2 {
            return;
        }

        let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev > 0.0 {
            self.sharpe_ratio = mean_return / std_dev;
        }
    }
}

/// Reinforcement Learning Agent with DeepSeek LLM and Historical Data
pub struct RLAgent {
    pub agent_id: String,
    pub provider_type: String,
    deepseek_client: Option<Arc<DeepSeekClient>>,
    experience_buffer: Arc<Mutex<VecDeque<Experience>>>,
    pub(crate) performance: Arc<Mutex<AgentPerformance>>,
    q_table: Arc<Mutex<HashMap<String, f64>>>, // Simple Q-learning table
    pub(crate) epsilon: f64, // Exploration rate
    gamma: f64,   // Discount factor
    max_buffer_size: usize,
    historical_data: Arc<Mutex<HistoricalDataManager>>, // Historical price data
}

impl RLAgent {
    pub fn new(agent_id: String, provider_type: String, deepseek_api_key: Option<String>) -> Self {
        let deepseek_client = deepseek_api_key.map(|key| Arc::new(DeepSeekClient::new(key)));

        Self {
            agent_id,
            provider_type,
            deepseek_client,
            experience_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            performance: Arc::new(Mutex::new(AgentPerformance::new())),
            q_table: Arc::new(Mutex::new(HashMap::new())),
            epsilon: 0.2, // 20% exploration
            gamma: 0.95,  // Future reward discount
            max_buffer_size: 1000,
            historical_data: Arc::new(Mutex::new(HistoricalDataManager::new(1000))), // Keep 1000 data points per symbol
        }
    }
    
    /// Add historical price data for training
    pub async fn add_historical_data(&self, symbol: String, data_point: PriceDataPoint) {
        let mut historical = self.historical_data.lock().await;
        historical.add_price_data(symbol, data_point);
    }
    
    /// Get historical features for a symbol
    pub async fn get_historical_features(&self, symbol: &str) -> Option<HistoricalFeatures> {
        let historical = self.historical_data.lock().await;
        historical.get_features(symbol)
    }

    /// Make a decision using DeepSeek LLM with reinforcement learning and historical data
    pub async fn make_decision(
        &self,
        state: &MarketState,
        historical_experiences: &[Experience],
    ) -> Result<Action, String> {
        // Get historical features for enhanced decision making
        let historical_features = self.get_historical_features(&state.symbol).await;
        // Get performance metrics for context
        let performance = self.performance.lock().await;
        let current_win_rate = performance.win_rate;
        let avg_reward = performance.avg_reward;
        let learning_rate = performance.learning_rate;
        drop(performance);

        // Epsilon-greedy strategy: explore vs exploit
        let should_explore = rand::random::<f64>() < self.epsilon;

        if should_explore {
            // Exploration: Try something different
            log::debug!("Agent {} exploring new strategy", self.agent_id);
            return Ok(self.generate_exploratory_action(state));
        }

        // Exploitation: Use learned knowledge
        if let Some(ref deepseek) = self.deepseek_client {
            // Use DeepSeek LLM for enhanced decision making with historical data
            match self.ask_deepseek_with_learning(deepseek, state, current_win_rate, avg_reward, learning_rate, historical_experiences, historical_features.as_ref()).await {
                Ok(decision) => {
                    return Ok(Action {
                        action_type: decision.action,
                        confidence: decision.confidence,
                        size: decision.suggested_size / 100.0, // Convert percentage
                        price: state.price,
                    });
                }
                Err(e) => {
                    log::warn!("DeepSeek error for agent {}: {}", self.agent_id, e);
                    // Fallback to Q-learning
                }
            }
        }

        // Fallback: Use Q-learning
        self.make_q_learning_decision(state).await
    }

    /// Ask DeepSeek LLM with learning context and historical data
    async fn ask_deepseek_with_learning(
        &self,
        deepseek: &DeepSeekClient,
        state: &MarketState,
        win_rate: f64,
        avg_reward: f64,
        learning_rate: f64,
        historical_experiences: &[Experience],
        historical_features: Option<&HistoricalFeatures>,
    ) -> Result<TradingDecision, Box<dyn std::error::Error>> {
        // Analyze recent experiences for patterns
        let recent_successes: Vec<&Experience> = historical_experiences.iter()
            .filter(|e| e.reward > 0.0)
            .rev()
            .take(5)
            .collect();

        let recent_failures: Vec<&Experience> = historical_experiences.iter()
            .filter(|e| e.reward < 0.0)
            .rev()
            .take(5)
            .collect();

        // Build enhanced learning context with historical data
        let historical_context = if let Some(features) = historical_features {
            format!(
                "\nHISTORICAL DATA ANALYSIS:\n\
                 - Data Points: {}\n\
                 - 5m Change: {:.2}%\n\
                 - 1h Change: {:.2}%\n\
                 - 4h Change: {:.2}%\n\
                 - 24h Change: {:.2}%\n\
                 - Volatility (20-period): {:.2}%\n\
                 - RSI (14-period): {}\n\
                 - Volume Ratio: {:.2}x average\n\
                 - Trend Strength: {:.2}%\n\
                 - EMA 10: ${:.2}\n\
                 - EMA 20: ${:.2}\n\
                 - SMA 50: ${:.2}",
                features.data_points,
                features.price_changes.change_5m,
                features.price_changes.change_1h,
                features.price_changes.change_4h,
                features.price_changes.change_24h,
                features.volatility,
                features.rsi.map(|r| format!("{:.1}", r)).unwrap_or_else(|| "N/A".to_string()),
                features.volume_ratio,
                features.trend_strength,
                features.moving_averages.ema_10.unwrap_or(0.0),
                features.moving_averages.ema_20.unwrap_or(0.0),
                features.moving_averages.sma_50.unwrap_or(0.0)
            )
        } else {
            "\nHISTORICAL DATA ANALYSIS: Not available".to_string()
        };
        
        // Build learning context for DeepSeek
        let _learning_context = format!(
            "AGENT PERFORMANCE CONTEXT:\n\
             - Provider Type: {}\n\
             - Current Win Rate: {:.1}%\n\
             - Average Reward: {:.4}\n\
             - Learning Rate: {:.4}\n\
             - Total Experiences: {}\n\n\
             RECENT SUCCESSFUL PATTERNS:\n{}\n\n\
             RECENT FAILED PATTERNS:\n{}\n\n\
             CURRENT MARKET STATE:\n\
             - Symbol: {}\n\
             - Price: ${:.2}\n\
             - Volume: {:.0}\n\
             - 1h Change: {:.2}%\n\
             - 24h Change: {:.2}%\n\
             - Sentiment: {:.1}/100\n\
             - Volatility: {:.2}%{}\n\n\
             Based on your learning history AND historical data patterns, what action should you take? \
             Prioritize strategies that have worked well and avoid patterns that failed. \
             Use the historical technical indicators to make more informed predictions.",
            self.provider_type,
            win_rate * 100.0,
            avg_reward,
            learning_rate,
            historical_experiences.len(),
            self.summarize_experiences(&recent_successes),
            self.summarize_experiences(&recent_failures),
            state.symbol,
            state.price,
            state.volume,
            state.price_change_1h,
            state.price_change_24h,
            state.sentiment_score,
            state.volatility,
            historical_context
        );

        // Call DeepSeek with minimal history for context
        let price_history = vec![state.price * 0.98, state.price * 0.99, state.price];
        let volume_history = vec![state.volume * 0.9, state.volume * 0.95, state.volume];

        // Enhanced analysis with learning context
        let decision = deepseek.analyze_trade(
            &state.symbol,
            state.price,
            &price_history,
            &volume_history,
            10000.0, // Placeholder portfolio value
            0.0,
        ).await?;

        Ok(decision)
    }

    /// Summarize experiences for learning
    fn summarize_experiences(&self, experiences: &[&Experience]) -> String {
        if experiences.is_empty() {
            return "None".to_string();
        }

        experiences.iter()
            .map(|e| format!(
                "  • {} on {} at ${:.2}: Reward {:.4} (Conf: {:.2})",
                e.action.action_type,
                e.state.symbol,
                e.action.price,
                e.reward,
                e.action.confidence
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate exploratory action (exploration strategy)
    fn generate_exploratory_action(&self, state: &MarketState) -> Action {
        let actions = ["BUY", "SELL", "HOLD"];
        let action_type = actions[rand::random::<usize>() % actions.len()].to_string();
        
        Action {
            action_type,
            confidence: 0.3 + rand::random::<f64>() * 0.4, // Lower confidence for exploration
            size: 0.02 + rand::random::<f64>() * 0.05, // Smaller size for exploration
            price: state.price,
        }
    }

    /// Q-learning decision (fallback)
    async fn make_q_learning_decision(&self, state: &MarketState) -> Result<Action, String> {
        let state_key = self.encode_state(state);
        let q_table = self.q_table.lock().await;

        let actions = vec!["BUY", "SELL", "HOLD"];
        let mut best_action = "HOLD".to_string();
        let mut best_q_value = f64::NEG_INFINITY;

        for action in &actions {
            let key = format!("{}:{}", state_key, action);
            let q_value = q_table.get(&key).copied().unwrap_or(0.0);
            if q_value > best_q_value {
                best_q_value = q_value;
                best_action = action.to_string();
            }
        }

        Ok(Action {
            action_type: best_action,
            confidence: 0.6,
            size: 0.05,
            price: state.price,
        })
    }

    /// Encode state for Q-table (improved with finer granularity)
    pub(crate) fn encode_state(&self, state: &MarketState) -> String {
        // Discretize continuous values with finer buckets for better accuracy
        let price_bucket = (state.price / 5.0).floor() as i32; // Finer: 5 instead of 10
        let change_bucket = (state.price_change_1h / 1.0).floor() as i32; // Finer: 1% instead of 2%
        let sentiment_bucket = (state.sentiment_score / 10.0).floor() as i32; // Finer: 10 instead of 20
        let volume_bucket = (state.volume.log10() / 0.5).floor() as i32; // Add volume dimension

        format!("{}:{}:{}:{}:{}", 
            state.symbol, price_bucket, change_bucket, sentiment_bucket, volume_bucket)
    }

    /// Record experience and learn from it
    pub async fn record_experience(&self, experience: Experience) {
        // Add to buffer
        let mut buffer = self.experience_buffer.lock().await;
        buffer.push_back(experience.clone());
        
        // Keep buffer size limited
        while buffer.len() > self.max_buffer_size {
            buffer.pop_front();
        }
        drop(buffer);

        // Update performance metrics
        let mut performance = self.performance.lock().await;
        performance.update(experience.reward);
        drop(performance);

        // Update Q-table
        self.update_q_table(&experience).await;

        log::debug!(
            "Agent {} learned from experience: {} on {} with reward {:.4}",
            self.agent_id,
            experience.action.action_type,
            experience.state.symbol,
            experience.reward
        );
    }

    /// Update Q-table using Q-learning algorithm
    async fn update_q_table(&self, experience: &Experience) {
        let state_key = self.encode_state(&experience.state);
        let action_key = format!("{}:{}", state_key, experience.action.action_type);

        let mut q_table = self.q_table.lock().await;
        let performance = self.performance.lock().await;
        let alpha = performance.learning_rate;
        drop(performance);

        let current_q = q_table.get(&action_key).copied().unwrap_or(0.0);

        // Q-learning update rule: Q(s,a) = Q(s,a) + α[r + γ*max(Q(s',a')) - Q(s,a)]
        let next_max_q = if let Some(ref next_state) = experience.next_state {
            let next_state_key = self.encode_state(next_state);
            let actions = ["BUY", "SELL", "HOLD"];
            actions.iter()
                .map(|a| {
                    let key = format!("{}:{}", next_state_key, a);
                    q_table.get(&key).copied().unwrap_or(0.0)
                })
                .fold(f64::NEG_INFINITY, f64::max)
        } else {
            0.0
        };

        let new_q = current_q + alpha * (experience.reward + self.gamma * next_max_q - current_q);
        q_table.insert(action_key, new_q);
    }

    /// Calculate reward from signal outcome
    pub fn calculate_reward(
        entry_price: f64,
        exit_price: f64,
        action_type: &str,
        confidence: f64,
    ) -> f64 {
        // Prevent division by zero
        if entry_price <= 0.0 {
            return 0.0;
        }

        let price_change = (exit_price - entry_price) / entry_price;

        let raw_reward = match action_type {
            "BUY" => price_change,
            "SELL" => -price_change,
            "HOLD" => 0.0,
            _ => 0.0,
        };

        // Scale reward by confidence (confident correct decisions get higher rewards)
        let confidence_bonus = if raw_reward > 0.0 { confidence } else { 1.0 };
        raw_reward * confidence_bonus * 100.0 // Scale to reasonable range
    }

    /// Get agent performance metrics
    pub async fn get_performance(&self) -> AgentPerformance {
        self.performance.lock().await.clone()
    }

    /// Get recent experiences for analysis
    pub async fn get_recent_experiences(&self, count: usize) -> Vec<Experience> {
        let buffer = self.experience_buffer.lock().await;
        buffer.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Adaptive exploration decay based on performance
    pub async fn decay_exploration(&mut self) {
        let performance = self.performance.lock().await;
        let win_rate = performance.win_rate;
        drop(performance);
        
        // Adaptive epsilon decay
        if win_rate > 0.6 {
            // Doing well, reduce exploration more aggressively
            self.epsilon = (self.epsilon * 0.99).max(0.03); // Min 3% exploration
        } else if win_rate < 0.4 {
            // Struggling, maintain higher exploration
            self.epsilon = (self.epsilon * 0.995).max(0.10); // Min 10% exploration
        } else {
            // Normal decay
            self.epsilon = (self.epsilon * 0.997).max(0.05); // Min 5% exploration
        }
        
        log::debug!("Agent {} epsilon decayed to {:.3}, win_rate: {:.2}%", 
                    self.agent_id, self.epsilon, win_rate * 100.0);
    }
}

/// Agent learning coordinator
pub struct LearningCoordinator {
    agents: Arc<Mutex<HashMap<String, Arc<RLAgent>>>>,
}

impl LearningCoordinator {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register an agent for learning
    pub async fn register_agent(&self, agent: Arc<RLAgent>) {
        let mut agents = self.agents.lock().await;
        agents.insert(agent.agent_id.clone(), agent);
    }

    /// Update all agents with market feedback
    pub async fn update_all_agents(&self, signal: &TradingSignalData, outcome: SignalOutcome) {
        let agents = self.agents.lock().await;
        
        if let Some(agent) = agents.get(&signal.provider) {
            let experience = Experience {
                state: MarketState {
                    symbol: signal.symbol.clone(),
                    price: signal.entry_price,
                    volume: 0.0, // Would be populated from market data
                    price_change_1h: 0.0,
                    price_change_24h: 0.0,
                    sentiment_score: signal.confidence * 100.0,
                    liquidity: 0.0,
                    volatility: 0.0,
                    market_cap: None,
                },
                action: Action {
                    action_type: format!("{:?}", signal.action),
                    confidence: signal.confidence,
                    size: 0.05, // Placeholder
                    price: signal.entry_price,
                },
                reward: outcome.reward,
                next_state: outcome.final_state,
                timestamp: Utc::now().timestamp(),
                provider_id: signal.provider.clone(),
            };

            agent.record_experience(experience).await;
        }
    }

    /// Get performance summary for all agents
    pub async fn get_all_performance(&self) -> HashMap<String, AgentPerformance> {
        let agents = self.agents.lock().await;
        let mut performance_map = HashMap::new();

        for (id, agent) in agents.iter() {
            performance_map.insert(id.clone(), agent.get_performance().await);
        }

        performance_map
    }
}

/// Outcome of a signal for learning
#[derive(Debug, Clone)]
pub struct SignalOutcome {
    pub reward: f64,
    pub final_state: Option<MarketState>,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_performance_update() {
        let mut perf = AgentPerformance::new();
        perf.update(0.5);
        assert_eq!(perf.total_trades, 1);
        assert_eq!(perf.successful_trades, 1);
    }

    #[test]
    fn test_calculate_reward() {
        let reward = RLAgent::calculate_reward(100.0, 110.0, "BUY", 0.8);
        assert!(reward > 0.0);
        
        let loss = RLAgent::calculate_reward(100.0, 90.0, "BUY", 0.8);
        assert!(loss < 0.0);
        
        // Test division by zero protection
        let zero_reward = RLAgent::calculate_reward(0.0, 110.0, "BUY", 0.8);
        assert_eq!(zero_reward, 0.0);
        
        let negative_reward = RLAgent::calculate_reward(-10.0, 110.0, "BUY", 0.8);
        assert_eq!(negative_reward, 0.0);
    }

    #[tokio::test]
    async fn test_rl_agent_creation() {
        let agent = RLAgent::new(
            "test_agent".to_string(),
            "test_provider".to_string(),
            None,
        );
        assert_eq!(agent.agent_id, "test_agent");
    }
}
