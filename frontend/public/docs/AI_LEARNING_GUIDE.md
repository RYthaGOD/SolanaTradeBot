# AI & Reinforcement Learning Guide

## Overview

The trading platform now includes **DeepSeek LLM integration** with **Reinforcement Learning** to enable agents that learn and improve over time. This guide explains how the AI learning system works and how to set it up.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Learning Coordinator                      │
│          • Manages all RL Agents                            │
│          • Coordinates learning updates                      │
│          • Tracks global performance                         │
└─────────────────────────────────────────────────────────────┘
                           ↓
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼─────┐      ┌─────▼────┐      ┌────▼─────┐
   │ RL Agent │      │ RL Agent │      │ RL Agent │
   │ Provider │      │ Provider │      │ Provider │
   │    1     │      │    2     │      │    N     │
   └────┬─────┘      └─────┬────┘      └────┬─────┘
        │                  │                  │
   ┌────▼──────────────────▼──────────────────▼─────┐
   │           DeepSeek LLM API                      │
   │   • Natural language understanding              │
   │   • Market analysis                            │
   │   • Trading decisions                          │
   │   • Learning from feedback                     │
   └────────────────────────────────────────────────┘
                           ↑
                    [API calls]
                           │
                      Your API Key
```

## Features

### 1. DeepSeek LLM Integration

**What is DeepSeek?**
- Advanced language model specialized in reasoning and analysis
- Free tier: 5 million tokens/month
- Suitable for trading analysis and decision-making

**Capabilities:**
- Analyzes market conditions in natural language
- Generates trading decisions with reasoning
- Learns from historical performance
- Adapts strategies based on success/failure patterns

### 2. Reinforcement Learning

**Q-Learning Implementation:**
- Agents learn optimal actions through trial and error
- Q-table stores value estimates for state-action pairs
- Epsilon-greedy exploration strategy
- Dynamic learning rate adjustment

**Experience Replay:**
- Stores past trading experiences
- Buffer size: 1,000 experiences per agent
- Used for pattern recognition
- Improves decision quality over time

### 3. Agent Performance Tracking

Each agent tracks:
- **Win Rate**: Percentage of profitable trades
- **Average Reward**: Mean return per trade
- **Sharpe Ratio**: Risk-adjusted performance
- **Learning Rate**: Dynamically adjusted based on performance
- **Total Trades**: Experience accumulation

## Setup Guide

### Step 1: Get DeepSeek API Key

1. Visit https://platform.deepseek.com/api_keys
2. Sign in or create account
3. Generate API key (starts with `sk-`)
4. Free tier includes 5M tokens/month

### Step 2: Secure Storage

**Option A: Interactive Setup (Recommended)**

```bash
cd backend
cargo run --bin setup_api_key
```

Follow the prompts to securely store your API key.

**Option B: Manual Environment Variable**

```bash
# Add to .env file
echo "DEEPSEEK_API_KEY=sk-your-actual-key-here" >> .env

# Set secure permissions
chmod 600 .env
```

**Option C: System Environment Variable**

```bash
export DEEPSEEK_API_KEY=sk-your-actual-key-here
```

### Step 3: Verify Configuration

```bash
# Start the system
cargo run

# Check API status
curl http://localhost:8080/ai/status
```

Expected response:
```json
{
  "success": true,
  "data": {
    "deepseek_enabled": "true",
    "model": "deepseek-chat",
    "features": "AI-powered trading decisions, risk assessment"
  },
  "message": "AI status"
}
```

## How It Works

### Learning Cycle

```
1. OBSERVE
   ↓
   Agent observes market state
   (price, volume, sentiment, volatility)
   
2. DECIDE
   ↓
   If exploring (20% chance):
     • Try random action
   Else if DeepSeek available:
     • Ask LLM with learning context
     • Get reasoned decision
   Else:
     • Use Q-learning table
   
3. ACT
   ↓
   Execute trading decision
   (Buy/Sell/Hold with confidence)
   
4. LEARN
   ↓
   Calculate reward from outcome
   Update Q-table
   Adjust learning rate
   Store experience
   
5. IMPROVE
   ↓
   Decay exploration rate
   Increase exploitation
   Refine strategy
```

### DeepSeek Decision Process

When making a decision, the agent:

1. **Gathers Context**:
   - Current market state
   - Recent successful trades
   - Recent failed trades
   - Current win rate
   - Average reward

2. **Constructs Prompt**:
   ```
   AGENT PERFORMANCE CONTEXT:
   - Provider Type: Memecoin Monitor
   - Current Win Rate: 65.5%
   - Average Reward: 0.0234
   - Learning Rate: 0.008
   - Total Experiences: 127
   
   RECENT SUCCESSFUL PATTERNS:
   • BUY on PEPE at $0.0001: Reward 0.0451 (Conf: 0.85)
   • BUY on BONK at $0.0002: Reward 0.0312 (Conf: 0.78)
   
   RECENT FAILED PATTERNS:
   • BUY on SHIB2 at $0.0003: Reward -0.0215 (Conf: 0.65)
   
   CURRENT MARKET STATE:
   - Symbol: DOGE2
   - Price: $0.00015
   - 1h Change: +8.5%
   - Sentiment: 82/100
   
   Based on your learning history, what action should you take?
   ```

3. **Gets LLM Response**:
   - Action (Buy/Sell/Hold)
   - Confidence (0-1)
   - Reasoning
   - Risk assessment
   - Suggested position size

4. **Executes Trade**

5. **Records Outcome**:
   - Stores experience
   - Updates Q-table
   - Adjusts learning parameters

### Reward Calculation

```rust
reward = price_change * confidence_bonus * 100

Where:
- price_change: (exit_price - entry_price) / entry_price
- confidence_bonus: agent's confidence in decision
- Positive for correct predictions
- Negative for incorrect predictions
- Scaled by confidence (confident correct = higher reward)
```

### Learning Rate Adjustment

```rust
if win_rate > 0.6:
    learning_rate *= 0.95  // Decrease (agent is doing well)
    learning_rate = max(0.001, learning_rate)
    
else if win_rate < 0.4:
    learning_rate *= 1.05  // Increase (agent needs to adapt)
    learning_rate = min(0.05, learning_rate)
```

## Agent Performance Monitoring

### View Individual Agent Performance

Each provider agent has associated RL metrics:

```bash
# Get provider statistics
curl http://localhost:8080/signals/marketplace/provider/memecoin_monitor

# Response includes RL performance
{
  "id": "memecoin_monitor",
  "name": "Memecoin Monitor",
  "reputation_score": 75.5,
  "total_signals": 120,
  "successful_signals": 85,
  "win_rate": 0.708,
  "learning_rate": 0.008,
  "avg_reward": 0.0234
}
```

### Global Performance Dashboard

```bash
# Get all agent performance
GET /api/learning/performance

# Response
{
  "memecoin_monitor": {
    "total_trades": 120,
    "successful_trades": 85,
    "win_rate": 0.708,
    "avg_reward": 0.0234,
    "sharpe_ratio": 1.45,
    "learning_rate": 0.008
  },
  "oracle_monitor": {
    "total_trades": 98,
    "successful_trades": 72,
    "win_rate": 0.735,
    "avg_reward": 0.0189,
    "sharpe_ratio": 1.62,
    "learning_rate": 0.007
  },
  ...
}
```

## Learning Parameters

### Default Configuration

```rust
epsilon: 0.2,           // 20% exploration rate
gamma: 0.95,            // Discount factor for future rewards
max_buffer_size: 1000,  // Experience replay buffer
initial_learning_rate: 0.01,
min_learning_rate: 0.001,
max_learning_rate: 0.05,
```

### Exploration vs Exploitation

- **Exploration (20%)**: Try new strategies, random actions
- **Exploitation (80%)**: Use learned knowledge, best-known actions
- Epsilon decays over time: `epsilon = epsilon * 0.995`
- Minimum exploration: 5% (always exploring)

## Advanced Features

### 1. Experience Replay

Agents store experiences for pattern recognition:

```rust
pub struct Experience {
    state: MarketState,        // Market conditions
    action: Action,            // What agent did
    reward: f64,               // Outcome
    next_state: Option<MarketState>,  // Result
    timestamp: i64,
    provider_id: String,
}
```

### 2. Pattern Recognition

DeepSeek analyzes:
- **Successful Patterns**: What worked well
- **Failed Patterns**: What to avoid
- **Market Conditions**: When strategies work
- **Confidence Correlation**: Link between confidence and success

### 3. Adaptive Learning

- Win rate > 60%: Decrease learning rate (fine-tuning)
- Win rate < 40%: Increase learning rate (major adjustments)
- Dynamic exploration: More when uncertain
- Sharpe ratio tracking: Risk-adjusted performance

### 4. Meta-Learning

The Master Analyzer (Provider 6) learns from all other agents:
- Identifies successful strategies across providers
- Recognizes failing patterns
- Generates consensus signals
- Cross-provider insights

## Security Best Practices

### API Key Security

1. **Never commit API keys to version control**
   - `.env` is in `.gitignore`
   - Use secure storage

2. **Encryption**:
   ```rust
   // Keys are encrypted before storage
   let encryption_key = SecureConfig::generate_encryption_key();
   config.with_encryption_key(encryption_key);
   config.save_deepseek_key(api_key)?;
   ```

3. **File Permissions**:
   - `.env` set to 600 (owner read/write only)
   - Config files in `.secure/` directory

4. **Key Rotation**:
   - Rotate keys periodically
   - Update immediately if compromised

### API Key Validation

```rust
// Automatic validation
SecureConfig::validate_deepseek_key(api_key)?;

// Checks:
// - Starts with 'sk-'
// - Minimum 32 characters
// - Proper format
```

## Troubleshooting

### Issue: "DeepSeek API key not found"

**Solution:**
```bash
# Check if key is set
echo $DEEPSEEK_API_KEY

# Run setup tool
cargo run --bin setup_api_key

# Verify in .env
cat .env | grep DEEPSEEK
```

### Issue: "AI features disabled"

**Causes:**
1. No API key configured
2. Invalid API key format
3. API key expired/revoked

**Solution:**
```bash
# Re-run setup
cargo run --bin setup_api_key

# Check API status
curl http://localhost:8080/ai/status
```

### Issue: "Agent not learning"

**Check:**
1. Sufficient experiences (need 10+ trades)
2. Reward calculation working
3. Q-table updates enabled
4. Learning rate not too low

**Debug:**
```bash
# View agent performance
curl http://localhost:8080/signals/marketplace/provider/memecoin_monitor

# Check logs for learning updates
grep "learned from experience" logs/app.log
```

## Performance Metrics

### Expected Improvement Timeline

| Phase | Trades | Expected Win Rate | Learning Status |
|-------|--------|-------------------|-----------------|
| **Week 1** | 0-50 | 40-50% | Initial exploration |
| **Week 2** | 50-150 | 50-60% | Pattern recognition |
| **Month 1** | 150-500 | 60-70% | Strategy refinement |
| **Month 3** | 500-1500 | 70-75% | Mature strategies |
| **Month 6+** | 1500+ | 75-80% | Optimized performance |

### Success Indicators

✅ **Good Learning**:
- Win rate steadily increasing
- Sharpe ratio > 1.0
- Learning rate decreasing (convergence)
- Avg reward trending positive
- Exploration rate decaying

❌ **Poor Learning**:
- Win rate stagnant or decreasing
- High volatility in rewards
- Learning rate stuck at max
- Repeated failed patterns
- No exploration decay

## API Reference

### Learning Endpoints

```bash
# Get agent performance
GET /api/learning/performance

# Get agent experiences
GET /api/learning/experiences/{agent_id}?limit=50

# Force learning update
POST /api/learning/update
Body: {
  "signal_id": "abc123",
  "outcome": "success",
  "reward": 0.0345
}

# Reset agent learning
POST /api/learning/reset/{agent_id}
```

## Future Enhancements

### Planned Features

1. **Multi-Agent Collaboration**
   - Agents share successful strategies
   - Collective learning
   - Strategy marketplace

2. **Advanced RL Algorithms**
   - Deep Q-Networks (DQN)
   - Proximal Policy Optimization (PPO)
   - Actor-Critic methods

3. **Transfer Learning**
   - Learn from similar markets
   - Cross-asset knowledge transfer
   - Pre-trained models

4. **Explainable AI**
   - Detailed reasoning for decisions
   - Strategy visualization
   - Performance attribution

5. **Automated Hyperparameter Tuning**
   - Optimize learning rate
   - Adjust exploration
   - Auto-tune parameters

## Conclusion

The AI learning system enables agents to:
- Make intelligent trading decisions
- Learn from successes and failures
- Improve performance over time
- Adapt to changing market conditions
- Leverage DeepSeek LLM for reasoning

With proper setup and monitoring, agents will continuously improve their trading strategies, leading to better performance and more profitable signals.

---

**Next Steps:**
1. Set up your DeepSeek API key: `cargo run --bin setup_api_key`
2. Start the system: `cargo run`
3. Monitor agent performance: Check provider stats
4. Watch agents learn and improve over time!
