# 🔥 AgentBurn Solana Trader

A sophisticated AI-powered trading system for Solana, built with Rust and React.

## 🚀 Features

- **AI-Powered Trading**: Machine learning models for market prediction using moving average strategies
- **Risk Management**: Dynamic position sizing based on Kelly criterion and drawdown control
- **Real-time Dashboard**: Live market data simulation and portfolio tracking
- **Solana Integration**: Ready for DEX trading and wallet integration (currently simulated)
- **Performance Analytics**: Comprehensive trading metrics including Sharpe ratio, win rate, and P&L tracking

### 🆕 New: Live Data & Autonomous Trading
- **Switchboard Oracle**: Live price feeds for SOL, BTC, ETH, and USDC with confidence intervals
- **DEX Screener Integration**: Real-time token discovery, trending analysis, and opportunity scoring
- **PumpFun Meme Tracking**: Monitor and analyze meme coin launches with sentiment analysis
- **Autonomous Agent**: Multi-source decision-making agent that trades 24/7
- **X402 Signal Platform**: Trade signals as assets using the X402 protocol
- **Wallet Integration**: Solana wallet management with keypair generation and secure storage
- **Treasury PDA**: Program Derived Address for agent trading treasury
- **RPC Integration**: Direct connection to Solana blockchain via RPC endpoints
- **Budget Management**: User-configurable trading budget with deposit/withdraw functionality
- **Advanced Quant Analysis**: 15+ technical indicators with signal quality scoring

## 🛠️ Tech Stack

**Backend:**
- Rust with Tokio async runtime
- Warp web framework for REST API
- Real-time market data simulation
- ML-based confidence scoring

**Frontend:**
- React 18 with TypeScript
- Vite for fast development
- Recharts for data visualization
- Axios for API communication

## 🏃‍♂️ Running the Application

The application automatically starts both backend and frontend servers:

```bash
./run.sh
```

Access the application:
- **Frontend Dashboard**: http://0.0.0.0:5000
- **Backend API**: http://localhost:8080

## 📊 API Endpoints

### Core Trading
- `GET /health` - Health check
- `GET /portfolio` - Portfolio data with positions and P&L
- `GET /performance` - Trading performance metrics
- `GET /market-data` - Live market data for SOL/USDC, BTC/USDC, ETH/USDC
- `GET /signals` - Recent AI-generated trading signals
- `WS /ws` - WebSocket real-time updates

### Live Data Feeds
- `GET /oracle/price/{symbol}` - Switchboard Oracle live price for symbol
- `GET /oracle/feeds` - All Switchboard Oracle feeds
- `GET /dex/search/{query}` - Search tokens on DEX Screener
- `GET /dex/opportunities` - Top trading opportunities from DEX Screener
- `GET /pumpfun/launches` - Recent meme coin launches from PumpFun
- `GET /pumpfun/signals` - Meme coin trading signals

### X402 Signal Trading Platform
- `GET /signals/marketplace/stats` - Marketplace statistics
- `GET /signals/marketplace/active` - All active tradeable signals
- `GET /signals/marketplace/symbol/{symbol}` - Signals for specific token
- `POST /signals/marketplace/generate/{provider_id}` - Generate and publish signals
- `POST /signals/marketplace/provider/register` - Register as signal provider
- `GET /signals/marketplace/provider/{id}` - Provider statistics
- `POST /signals/marketplace/purchase` - Purchase a signal using X402 protocol

### Wallet & Treasury
- `GET /wallet/status` - Get wallet information (address, balance, treasury, budget)
- `GET /treasury/status` - Get treasury PDA information

### Budget Management
- `GET /budget/status` - Check current trading budget and balance
- `POST /budget/set` - Set trading budget (requires JSON: `{"budget": 10000.0}`)
- `POST /budget/deposit` - Deposit funds to budget (requires JSON: `{"amount": 5000.0}`)
- `POST /budget/withdraw` - Withdraw funds from budget (requires JSON: `{"amount": 2000.0}`)

### Quantitative Analysis
- `GET /quant/analyze/{symbol}` - Get detailed technical analysis for a symbol
- `GET /quant/overview` - Get quick analysis overview for all symbols

### DEX Integration
- `GET /jupiter/quote/{input_mint}/{output_mint}/{amount}` - Get Jupiter swap quote
- `GET /ai/status` - DeepSeek AI configuration status

## 📁 Project Structure

```
agentburn-solana-trader/
├── backend/              # Rust trading engine
│   ├── src/
│   │   ├── main.rs      # Application entry point
│   │   ├── trading_engine.rs    # Core trading logic with SMA strategy
│   │   ├── solana_integration.rs # Market data simulation
│   │   ├── risk_management.rs   # Position sizing and risk control
│   │   ├── ml_models.rs        # ML prediction models
│   │   └── api.rs              # REST API server
│   └── Cargo.toml       # Rust dependencies
├── frontend/            # React dashboard
│   ├── src/
│   │   ├── components/  # Dashboard, Trading, Portfolio, Performance
│   │   ├── App.tsx      # Main application
│   │   └── main.tsx     # React entry point
│   └── package.json     # Node dependencies
└── run.sh              # Startup script
```

## 🔧 How It Works

### Trading Strategy
1. **Market Data Simulation**: Generates realistic price movements for crypto pairs
2. **Moving Average Analysis**: Uses SMA-10 and SMA-20 crossover detection
3. **Signal Generation**: Creates buy/sell signals with confidence scores
4. **Risk Management**: Validates trades against drawdown limits and position sizing rules
5. **Portfolio Management**: Tracks positions, P&L, and performance metrics

### Risk Controls
- Maximum 10% of capital per trade
- Kelly criterion-based position sizing
- Maximum drawdown limit of 10%
- Confidence threshold of 50% for trade execution

### Wallet & Blockchain Integration

**Wallet Management:**
- Automatic keypair generation or load from environment variable (`WALLET_PRIVATE_KEY`)
- Support for Solana CLI JSON format wallet files
- Secure storage with proper file permissions (0600 on Unix)
- Base58 private key encoding/decoding

**Treasury PDA (Program Derived Address):**
- Deterministic address derivation for agent trading treasury
- Separate treasury account for isolating agent funds
- Authority-based access control
- Seed-based PDA derivation for multiple agent treasuries

**RPC Integration:**
- Direct connection to Solana blockchain (devnet/mainnet-beta)
- Real-time balance queries
- Transaction submission and confirmation
- Account state queries
- Block and slot information

**API Endpoints:**
```bash
# Check wallet status
curl http://localhost:8080/wallet/status

# Check treasury PDA
curl http://localhost:8080/treasury/status
```

**Configuration:**
```bash
# Set RPC endpoint (defaults to devnet)
SOLANA_RPC_URL=https://api.devnet.solana.com

# Optional: Provide existing wallet (otherwise generates new)
WALLET_PRIVATE_KEY=your_base58_private_key_here

# Set trading budget (defaults to 10000.0)
TRADING_BUDGET=8000.0
```

### Budget Management

The system now supports configurable trading budgets:

**Set Budget:**
```bash
# Via environment variable
TRADING_BUDGET=10000.0

# Or via API
curl -X POST http://localhost:8080/budget/set \
  -H "Content-Type: application/json" \
  -d '{"budget": 15000.0}'
```

**Manage Funds:**
```bash
# Deposit funds
curl -X POST http://localhost:8080/budget/deposit \
  -H "Content-Type: application/json" \
  -d '{"amount": 5000.0}'

# Withdraw funds
curl -X POST http://localhost:8080/budget/withdraw \
  -H "Content-Type: application/json" \
  -d '{"amount": 2000.0}'

# Check budget status
curl http://localhost:8080/budget/status
```

### Advanced Quantitative Analysis

The system includes comprehensive technical analysis with 15+ indicators:

**Technical Indicators:**
- **Trend**: SMA-10, SMA-20, SMA-50, EMA-12, EMA-26
- **Momentum**: RSI-14, MACD (with signal and histogram)
- **Volatility**: Bollinger Bands, ATR-14, Standard Deviation
- **Volume**: On-Balance Volume (OBV)
- **Price Action**: Momentum percentage

**Signal Quality Scoring:**
- Score: 0-100 (higher = stronger signal)
- Trend: Bullish/Neutral/Bearish
- Strength: Strong/Moderate/Weak
- Risk Level: Low/Medium/High
- Recommendation: Strong Buy/Buy/Hold/Sell/Strong Sell

**Usage:**
```bash
# Get detailed analysis for a symbol
curl http://localhost:8080/quant/analyze/SOL/USDC | jq

# Get quick overview for all symbols
curl http://localhost:8080/quant/overview | jq
```

**Example Response:**
```json
{
  "signal_quality": {
    "score": 72.5,
    "trend": "Bullish",
    "strength": "Strong",
    "confidence": 0.78,
    "risk_level": "Medium",
    "recommendation": "Buy"
  },
  "indicators": {
    "sma_10": 105.2,
    "sma_20": 102.8,
    "rsi_14": 58.3,
    "macd": 2.15,
    "bollinger_upper": 108.5,
    "bollinger_lower": 98.2
  }
}
```

## 🚨 Important Notes

- Trading can be done in **simulation mode** (default) or with **real Solana integration**
- In simulation mode, no real funds are at risk
- For real trading, configure wallet and RPC endpoint in `.env`
- Always test on devnet before using mainnet
- Market data can be simulated or fetched from live sources (Switchboard, DEX Screener)

## 🎯 X402 Signal Trading Protocol

The platform implements the **X402 protocol** for automated signal trading between agents:

### What is X402?
X402 is a protocol for decentralized trading signal exchange that enables:
- **Signal Marketplace**: Buy and sell trading signals as tradeable assets
- **Provider Reputation**: Track provider success rates and earnings
- **Automated Trading**: Agents can purchase and execute signals autonomously  
- **Multi-Source Analysis**: Signals generated from Oracle, DEX, and PumpFun data

### Using the Signal Platform
```bash
# Register as a signal provider
curl -X POST http://localhost:8080/signals/marketplace/provider/register \
  -H "Content-Type: application/json" \
  -d '{"id": "provider1", "name": "My Trading Signals"}'

# Generate signals from all data sources
curl -X POST http://localhost:8080/signals/marketplace/generate/provider1

# View active signals
curl http://localhost:8080/signals/marketplace/active

# Purchase a signal
curl -X POST http://localhost:8080/signals/marketplace/purchase \
  -H "Content-Type: application/json" \
  -d '{"user_id": "trader1", "signal_id": "abc123", "payment": 10.0}'
```

## 🔮 Future Enhancements

- Real Solana blockchain integration with DEX connectivity
- Advanced ML models for price prediction
- Backtesting engine with historical data
- Multi-strategy support and strategy optimization
- X402 on-chain signal marketplace with smart contracts

## ⚠️ Risk Warning

This is experimental software for educational purposes. Always understand the code and test thoroughly before considering any real trading applications.

## 📄 License

MIT License
