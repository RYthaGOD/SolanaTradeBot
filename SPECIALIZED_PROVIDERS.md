# Specialized Signal Providers

## Overview

The trading platform now includes **5 specialized provider agents** that each focus on specific market analysis and signal generation strategies. These providers work autonomously and interact through the X402 signal marketplace.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Signal Marketplace (X402)                   │
│         • Provider Registration                          │
│         • Signal Publishing                              │
│         • Signal Trading                                 │
└─────────────────────────────────────────────────────────┘
         ↑           ↑           ↑           ↑           ↑
    [publish]   [publish]   [publish]   [publish]   [buy/sell]
         │           │           │           │           │
    ┌────┴────┐ ┌───┴────┐ ┌────┴────┐ ┌───┴────┐ ┌────┴────┐
    │Provider1│ │Provider2│ │Provider3│ │Provider4│ │Provider5│
    │Memecoin │ │ Oracle  │ │  Perps  │ │ Opport. │ │ Signal  │
    │ Monitor │ │ Monitor │ │ Monitor │ │Analyzer │ │ Trader  │
    └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘
         ↓           ↓           ↓           ↓           ↓
    PumpFun+    Switchboard   Oracle+      DEX       Marketplace
     Oracle       Oracle      Jupiter    Screener      Signals
```

## The 5 Specialized Providers

### 1. Memecoin Monitor (`memecoin_monitor`)

**Purpose**: Monitors and analyzes meme coin launches using oracle price validation

**Data Sources**:
- PumpFun API for meme coin launches
- Switchboard Oracle for SOL/USD price

**Strategy**:
- Fetches recent meme coin launches (top 10)
- Analyzes sentiment using engagement metrics, market cap, and launch timing
- Only generates signals for memecoins with sentiment score > 60/100
- Validates entry prices against SOL oracle price
- 30-minute signal expiry (fast-moving market)

**Signal Characteristics**:
- **Timeframe**: 30 minutes
- **Target**: +20% profit
- **Stop Loss**: -15%
- **Signal Price**: 25 tokens (premium for meme signals)
- **Confidence**: Based on sentiment score (max 95%)

**Example Signal**:
```json
{
  "provider": "memecoin_monitor",
  "symbol": "PEPE",
  "action": "Buy",
  "confidence": 0.85,
  "analysis": "Memecoin PEPE - Sentiment: 85/100, Hype: High, Market Cap: $50000, SOL Price: $105.50",
  "price": 25.0
}
```

---

### 2. Oracle Monitor (`oracle_monitor`)

**Purpose**: Pure oracle data analysis and price movement detection

**Data Sources**:
- Switchboard Oracle only (SOL/USD, BTC/USD, ETH/USD)

**Strategy**:
- Monitors price movements from oracle feeds
- Generates signals on significant price changes (>1.5%)
- Uses oracle confidence intervals for signal quality
- Buy signals on upward momentum, sell signals on downward
- 1-hour signal expiry

**Signal Characteristics**:
- **Timeframe**: 1 hour
- **Target**: +3% profit (conservative)
- **Stop Loss**: -2%
- **Signal Price**: 10 tokens (base price)
- **Confidence**: Based on price change magnitude (max 90%)

**Example Signal**:
```json
{
  "provider": "oracle_monitor",
  "symbol": "SOL/USD",
  "action": "Buy",
  "confidence": 0.75,
  "analysis": "Oracle-based signal: SOL/USD movement of 2.5%, Price: $105.50, Confidence: 98.5%",
  "price": 10.0
}
```

---

### 3. Perps Monitor (`perps_monitor`)

**Purpose**: Monitors perpetual futures markets on Jupiter using oracle data

**Data Sources**:
- Switchboard Oracle for price feeds
- Jupiter Perps market (simulated)

**Strategy**:
- Analyzes volatility from oracle price data
- High volatility (>5%) triggers perps signals
- Suggests 2x leverage for positions
- Direction based on oracle confidence (low confidence = breakout potential)
- 2-hour signal expiry

**Signal Characteristics**:
- **Timeframe**: 2 hours
- **Target**: +5% * 2x leverage = +10%
- **Stop Loss**: -3%
- **Signal Price**: 15 tokens
- **Confidence**: Based on volatility (max 85%)
- **Additional**: Leverage recommendation included

**Example Signal**:
```json
{
  "provider": "perps_monitor",
  "symbol": "SOL-PERP",
  "action": "Buy",
  "confidence": 0.80,
  "analysis": "Perps signal: SOL/USD - Volatility: 7.5%, Suggested Leverage: 2x, Oracle Confidence: 95%",
  "price": 15.0
}
```

---

### 4. Opportunity Analyzer (`opportunity_analyzer`)

**Purpose**: Analyzes all DEX trading opportunities across Solana

**Data Sources**:
- DEX Screener API for token discovery
- Multi-DEX liquidity and volume analysis

**Strategy**:
- Scans top 10 opportunities from DEX Screener
- Only signals on very high quality opportunities (score >75/100)
- Considers momentum, volume, liquidity, and price action
- 4-hour signal expiry (medium-term trades)

**Signal Characteristics**:
- **Timeframe**: 4 hours
- **Target**: +12% profit
- **Stop Loss**: -8%
- **Signal Price**: 20 tokens
- **Confidence**: Based on opportunity score (max 92%)

**Example Signal**:
```json
{
  "provider": "opportunity_analyzer",
  "symbol": "RAY",
  "action": "Buy",
  "confidence": 0.88,
  "analysis": "High opportunity: Raydium - Score: 88, Vol 24h: $5000000, Liquidity: $150000, Signals: Strong momentum, Volume spike",
  "price": 20.0
}
```

---

### 5. Signal Trader (`signal_trader`)

**Purpose**: Meta-agent that buys/sells signals from other providers and generates consensus signals

**Data Sources**:
- X402 Signal Marketplace
- Signals from all other providers

**Strategy (Dual Function)**:

#### A. Signal Purchasing
- Continuously monitors marketplace for signals from other providers
- Evaluates signals based on:
  - Confidence > 70%
  - Price < 30 tokens
  - Time remaining > 30 minutes
  - Risk/reward ratio > 1.5
- Manages $10,000 starting capital
- Never buys own signals

#### B. Meta-Signal Generation
- Identifies consensus signals (3+ providers agreeing on same symbol)
- Generates premium meta-signals with:
  - Averaged entry prices from multiple providers
  - Boosted confidence (avg * 1.1, max 95%)
  - 6-hour expiry (longer-term consensus plays)

**Signal Characteristics**:
- **Timeframe**: 6 hours
- **Target**: +10% profit
- **Stop Loss**: -6%
- **Signal Price**: 30 tokens (premium for consensus)
- **Confidence**: Consensus-based (max 95%)

**Example Signal**:
```json
{
  "provider": "signal_trader",
  "symbol": "SOL/USD",
  "action": "Buy",
  "confidence": 0.91,
  "analysis": "Consensus signal: SOL/USD - 4 providers agree, Avg confidence: 85%",
  "price": 30.0,
  "data_sources": ["Meta-Analysis from 4 providers", "Signal Consensus"]
}
```

---

## Provider Lifecycle

### Initialization

All 5 providers are initialized on system startup:

```rust
// In main.rs
let marketplace = Arc::new(SignalMarketplace::new(rpc_url.clone()));
let providers = initialize_all_providers(marketplace, rpc_url).await;

for provider in providers {
    tokio::spawn(async move {
        provider.run().await;  // Each runs independently
    });
}
```

### Runtime Behavior

Each provider runs in its own async task with a 60-second check interval:

1. **Generate Signals**: Provider-specific logic analyzes data sources
2. **Publish to Marketplace**: Valid signals published via X402 protocol
3. **Sleep**: Wait 60 seconds before next cycle
4. **Repeat**: Continuous operation

### Provider Registration

Each provider automatically registers with the marketplace on startup:

```rust
marketplace.register_provider(
    "memecoin_monitor".to_string(),
    "Memecoin Monitor".to_string()
).await
```

---

## Signal Quality Metrics

### By Provider Type

| Provider | Avg Signals/Hour | Confidence Range | Price Range | Risk Level |
|----------|------------------|------------------|-------------|------------|
| Memecoin Monitor | 3-5 | 60-95% | 25 tokens | High |
| Oracle Monitor | 2-4 | 50-90% | 10 tokens | Low |
| Perps Monitor | 1-3 | 60-85% | 15 tokens | Medium |
| Opportunity Analyzer | 1-2 | 75-92% | 20 tokens | Medium |
| Signal Trader | 0-2 | 85-95% | 30 tokens | Low |

### Signal Expiry Times

- **Fast Markets** (Memecoins): 30 minutes
- **Standard** (Oracle): 1 hour
- **Medium-term** (Perps): 2 hours
- **Position Trades** (Opportunities): 4 hours
- **Consensus** (Meta-signals): 6 hours

---

## API Integration

### View Provider Statistics

```bash
# Get specific provider stats
GET /signals/marketplace/provider/{provider_id}

# Response
{
  "id": "memecoin_monitor",
  "name": "Memecoin Monitor",
  "reputation_score": 75.5,
  "total_signals": 120,
  "successful_signals": 85,
  "earnings": 2125.0
}
```

### View Active Signals by Provider

```bash
# Get all active signals
GET /signals/marketplace/active

# Filter by provider in client
signals.filter(s => s.provider === "memecoin_monitor")
```

### Generate Signals Manually

```bash
# Trigger signal generation for specific provider
POST /signals/marketplace/generate/{provider_id}

# Example: Generate memecoin signals
POST /signals/marketplace/generate/memecoin_monitor
```

---

## Provider Interaction Example

### Scenario: Signal Trader Purchases and Republishes

1. **Oracle Monitor** detects SOL price movement
   - Generates signal: SOL/USD Buy @ $105, confidence 0.80

2. **Memecoin Monitor** finds high-sentiment meme
   - Generates signal: PEPE Buy @ $0.0001, confidence 0.85

3. **Opportunity Analyzer** finds DEX opportunity
   - Generates signal: RAY Buy @ $2.50, confidence 0.88

4. **Signal Trader** evaluates marketplace
   - Purchases all 3 signals (total cost: $55 tokens)
   - Capital: $10,000 → $9,945

5. If 3+ providers agree on SOL/USD:
   - **Signal Trader** generates meta-signal
   - SOL/USD Buy @ $105.20, confidence 0.91
   - Sells for 30 tokens premium

---

## Configuration

### Environment Variables

```bash
# Provider-specific settings (future enhancement)
PROVIDER_CHECK_INTERVAL=60  # Seconds between checks
PROVIDER_INITIAL_CAPITAL=10000  # For signal trader
MIN_CONSENSUS_PROVIDERS=3  # For meta-signals
```

### Per-Provider Tuning

Each provider can be independently configured:

```rust
// In specialized_providers.rs
pub fn new(...) -> Self {
    Self {
        check_interval_secs: 60,  // Can be customized per provider
        capital: Arc::new(Mutex::new(10000.0)),  // Only for signal trader
        // ...
    }
}
```

---

## Monitoring & Debugging

### Logs

Each provider logs its activities:

```
🤖 Starting Memecoin Monitor provider: memecoin_monitor
✅ Memecoin Monitor published 3 signals
📊 Signal Trader purchased signal abc123 from oracle_monitor for $10.00
✅ Signal Trader published 1 signals (meta-signal)
```

### Marketplace Statistics

```bash
GET /signals/marketplace/stats

{
  "total_signals": 45,
  "active_signals": 23,
  "total_providers": 5,
  "total_subscriptions": 12,
  "protocol_version": "X402-1.0"
}
```

---

## Future Enhancements

### Planned Features

1. **Provider Performance Tracking**
   - Win rate per provider
   - Average profit per signal
   - Sharpe ratio calculation

2. **Dynamic Pricing**
   - Signal prices adjust based on historical performance
   - Premium for high-performing providers

3. **Provider Competition**
   - Leaderboards
   - Reputation-based bonuses
   - Top provider rewards

4. **Advanced Signal Trader**
   - Machine learning for signal evaluation
   - Portfolio optimization across purchased signals
   - Risk-adjusted position sizing

5. **Provider Specialization**
   - Custom intervals per provider
   - Provider-specific risk parameters
   - Market condition awareness

---

## Testing

### Unit Tests

Each provider type has dedicated tests:

```rust
#[test]
fn test_provider_types() {
    let types = vec![
        ProviderType::MemecoinMonitor,
        ProviderType::OracleMonitor,
        ProviderType::PerpsMonitor,
        ProviderType::OpportunityAnalyzer,
        ProviderType::SignalTrader,
    ];
    assert_eq!(types.len(), 5);
}
```

### Integration Testing

```bash
# Start system
cargo run

# Verify all providers registered
curl http://localhost:8080/signals/marketplace/stats

# Check active signals from all providers
curl http://localhost:8080/signals/marketplace/active
```

---

## Summary

The 5 specialized providers create a **complete signal marketplace ecosystem**:

1. **Memecoin Monitor**: Fast-moving meme opportunities
2. **Oracle Monitor**: Reliable, low-risk oracle signals
3. **Perps Monitor**: Leveraged perps opportunities
4. **Opportunity Analyzer**: High-quality DEX opportunities
5. **Signal Trader**: Meta-agent that trades and creates consensus signals

Together, they provide **diverse signal sources** and enable a **self-sustaining marketplace** where providers compete and the signal trader acts as an aggregator and quality filter.
