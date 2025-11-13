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

## 🚨 Important Notes

- This is a **simulated trading environment** - no real funds are at risk
- Market data is generated algorithmically for demonstration purposes
- Trading signals are based on simple moving average strategies
- Real Solana integration requires additional configuration (wallet, RPC endpoint)

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
