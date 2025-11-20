# 🔮 Prediction Markets Trading System

A streamlined, focused trading system for on-chain prediction markets on Solana - similar to Polymarket but optimized for Solana blockchain.

## 🎯 Core Focus

This system is **exclusively focused** on prediction market trading with:

- **Expected Value (EV) Analysis** - Identifies positive EV opportunities
- **Kelly Criterion Position Sizing** - Optimal bet sizing based on edge
- **Automated Signal Generation** - Real-time trading signals for all markets
- **Risk-Adjusted Trading** - Conservative position sizing to manage risk

## ✨ Key Features

### 📊 Market Analysis
- Binary prediction markets (Yes/No outcomes)
- Real-time probability tracking
- Volume and liquidity analysis
- Bid/ask spread monitoring
- Market category classification (Crypto, Politics, Sports, etc.)

### 🎲 Expected Value (EV) Trading
- Calculates true probability vs market-implied probability
- Identifies mispriced markets with positive expected value
- Only generates signals when edge exceeds 5% threshold
- Confidence scoring based on EV magnitude

### 💰 Kelly Criterion Position Sizing
- Automatically calculates optimal position size
- Caps at 25% of bankroll for risk management
- Adjusts for win probability and payout odds
- Prevents over-betting and ruin risk

### 📈 Trading Signals
Each signal includes:
- **Action**: Buy/Sell Yes or No
- **Target Price**: Current market price
- **Confidence**: 0-100% based on EV edge
- **Expected Value**: Percentage edge over market
- **Kelly Fraction**: Recommended position size
- **Reasoning**: Detailed analysis explanation

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Node.js 18+ (for frontend)

### Running the Backend

```bash
# Navigate to backend
cd backend

# Run the prediction-markets binary
cargo run --bin prediction-markets

# Or build release version for production
cargo build --release --bin prediction-markets
./target/release/prediction-markets
```

### Running the Frontend

```bash
# Navigate to frontend
cd frontend

# Install dependencies (first time only)
npm install

# Start development server
npm run dev

# Access at http://localhost:5173
```

## 📡 API Endpoints

### Core Endpoints

**Health Check**
```bash
GET /health
```

**List All Markets**
```bash
GET /markets
```
Returns all active prediction markets with outcomes, prices, and volume.

**Get Market Details**
```bash
GET /markets/{market_id}
```
Returns detailed information for a specific market.

**Market Statistics**
```bash
GET /stats
```
Returns aggregate statistics: total markets, liquidity, volume.

**Get All Trading Signals**
```bash
GET /signals
```
Returns trading signals for all markets with positive EV.

**Get Signals for Specific Market**
```bash
GET /signals/{market_id}
```
Returns trading signals for one market only.

**Execute Trade**
```bash
POST /trade
Content-Type: application/json

{
  "market_id": "market_btc_100k",
  "outcome_id": "yes_btc_100k",
  "action": "buy_yes",
  "amount": "100"
}
```

### Example API Calls

```bash
# Check server health
curl http://localhost:8080/health

# List all markets
curl http://localhost:8080/markets | jq

# Get market statistics
curl http://localhost:8080/stats | jq

# Get all trading signals
curl http://localhost:8080/signals | jq

# Get signals for specific market
curl http://localhost:8080/signals/market_btc_100k | jq

# Execute a trade
curl -X POST http://localhost:8080/trade \
  -H "Content-Type: application/json" \
  -d '{
    "market_id": "market_btc_100k",
    "outcome_id": "yes_btc_100k", 
    "action": "buy_yes",
    "amount": "100"
  }'
```

## 📊 Example Markets

The system comes with 3 pre-configured crypto prediction markets:

### 1. Bitcoin to $100K
- **Question**: "Will Bitcoin reach $100,000 by end of 2025?"
- **Category**: Crypto
- **Liquidity**: $100,000
- **Current Odds**: Yes 65%, No 35%

### 2. Solana to $500
- **Question**: "Will Solana reach $500 in 2025?"
- **Category**: Crypto
- **Liquidity**: $60,000
- **Current Odds**: Yes 42%, No 58%

### 3. Ethereum to $10K
- **Question**: "Will Ethereum reach $10,000 by end of 2025?"
- **Category**: Crypto
- **Liquidity**: $80,000
- **Current Odds**: Yes 55%, No 45%

## 🎓 How It Works

### 1. Market Discovery
The system continuously monitors all active prediction markets for trading opportunities.

### 2. Probability Estimation
For each market outcome, the system:
- Analyzes market dynamics (price momentum, volume, liquidity)
- Estimates the "true" probability of the outcome
- Compares against market-implied probability (current price)

### 3. Expected Value Calculation
```
EV = (True Probability × Payout) - ((1 - True Probability) × Cost)
```
Positive EV = Good bet opportunity

### 4. Signal Generation
If EV exceeds threshold (5%):
- Generate trading signal with action (Buy/Sell)
- Calculate confidence based on EV magnitude
- Compute Kelly Criterion position size
- Provide reasoning for the recommendation

### 5. Risk Management
- Kelly fraction capped at 25% maximum
- Only trade when confidence > 60%
- Spread bets across multiple markets
- Account for fees (2% default)

## 📈 Trading Strategy

### Expected Value (EV) Approach
The system uses a mathematically rigorous approach:

1. **Identify Mispricing**: Find markets where implied probability differs from estimated true probability
2. **Calculate Edge**: Determine the percentage advantage
3. **Size Position**: Use Kelly Criterion for optimal bet size
4. **Execute**: Trade when edge is significant (>5%)

### Kelly Criterion
```
Kelly % = (Win Probability × Odds - (1 - Win Probability)) / Odds
```

Example:
- Market price: 65% (1.54 odds)
- True probability: 70%
- Kelly fraction: ~6% of bankroll

### Risk Controls
- **Max Position**: 25% of bankroll per market
- **Min Edge**: 5% expected value to trade
- **Min Confidence**: 60% to generate signal
- **Diversification**: Spread across multiple markets

## 🔧 Configuration

### Environment Variables

```bash
# Use real on-chain data (when available)
USE_REAL_PREDICTION_DATA=true

# Set custom port (default: 8080)
PORT=8080

# Log level
RUST_LOG=info
```

## 📊 Frontend Features

The web interface provides:

1. **Market Overview**
   - Grid of all active markets
   - Real-time price updates
   - Category filtering
   - Volume and liquidity tracking

2. **Market Details**
   - Full market question and context
   - Outcome prices with bid/ask
   - Historical price changes
   - Trade execution interface

3. **Trading Signals**
   - List of all opportunities
   - EV and Kelly sizing for each
   - Confidence scores
   - Detailed reasoning

4. **Statistics Dashboard**
   - Total markets tracked
   - Aggregate liquidity
   - 24-hour volume
   - Active opportunities

## 🧪 Testing

```bash
# Run all tests
cd backend
cargo test

# Run tests for prediction markets module
cargo test prediction_markets

# Run with output
cargo test -- --nocapture
```

## 📝 Market Structure

```rust
PredictionMarket {
    market_id: String,
    question: String,
    category: MarketCategory,
    outcomes: Vec<MarketOutcome>,
    liquidity: f64,
    volume_24h: f64,
    end_date: i64,
    status: MarketStatus,
    fee_bps: u16,
}

MarketOutcome {
    outcome_id: String,
    name: String,
    price: f64,        // Probability (0.0 to 1.0)
    shares: f64,
    volume: f64,
    bid: Option<f64>,
    ask: Option<f64>,
}
```

## 🎯 Use Cases

1. **Crypto Price Predictions** - Trade on BTC, ETH, SOL price targets
2. **Market Events** - Protocol launches, network upgrades, etc.
3. **Value Betting** - Identify mispriced markets for profit
4. **Portfolio Hedging** - Use predictions to hedge spot positions
5. **Research** - Backtest prediction market strategies

## ⚠️ Important Notes

### Development Mode
- Currently uses **simulated markets** for testing
- No real funds at risk
- Perfect for development and strategy testing

### Production Mode
- Set `USE_REAL_PREDICTION_DATA=true` for real markets
- Requires integration with on-chain prediction market protocol
- Always start with small positions

### Trading Disclaimer
- This is experimental trading software
- Prediction markets carry significant risk
- Never bet more than you can afford to lose
- Understand the math before trading
- Past performance doesn't guarantee future results

## 🚧 Roadmap

### Phase 1: Core Platform (Current)
- ✅ Expected Value analysis
- ✅ Kelly Criterion position sizing
- ✅ Trading signal generation
- ✅ REST API
- ✅ Web interface

### Phase 2: Advanced Features
- [ ] Historical market data
- [ ] Backtesting engine
- [ ] Multi-market arbitrage
- [ ] Automated trade execution
- [ ] Performance analytics

### Phase 3: On-Chain Integration
- [ ] Solana smart contract integration
- [ ] Real market data feeds
- [ ] On-chain trade execution
- [ ] Liquidity pool interactions
- [ ] Wallet integration

## 📄 License

MIT License

## 🤝 Contributing

Contributions welcome! Focus areas:
- Market analysis algorithms
- Signal quality improvements
- Risk management enhancements
- UI/UX improvements
- Documentation

---

Built with ❤️ for the Solana prediction markets ecosystem
