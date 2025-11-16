# 🔥 AgentBurn Solana Trader

A sophisticated AI-powered trading system for Solana, built with Rust and React.

## ⚠️ IMPORTANT WARNINGS

**🚨 THIS SYSTEM IS CURRENTLY IN PAPER TRADING MODE 🚨**

- **NOT READY FOR MAINNET**: This system is for development and testing only
- **NO REAL TRADING**: Currently operates in simulation mode with fake market data
- **READ MAINNET_READINESS.md**: See comprehensive checklist before considering mainnet deployment
- **PAPER TRADING ENABLED**: All trades are simulated - no real funds at risk

For a detailed mainnet readiness assessment, see [MAINNET_READINESS.md](./MAINNET_READINESS.md)

## 🚀 Features

- **AI-Powered Trading**: Machine learning models for market prediction using moving average strategies
- **Risk Management**: Dynamic position sizing based on Kelly criterion and drawdown control
- **Real-time Dashboard**: Live market data simulation and portfolio tracking
- **Solana Integration**: Ready for DEX trading and wallet integration (currently simulated)
- **Performance Analytics**: Comprehensive trading metrics including Sharpe ratio, win rate, and P&L tracking

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

## 🏃‍♂️ Quick Start

### Prerequisites
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Node.js 18+ (`https://nodejs.org/`)

### Setup

1. **Configure Environment**:
```bash
cd backend
cp .env.example .env
# Edit .env with your settings (defaults are safe for testing)
```

2. **Start the Application**:
```bash
# Option 1: Use the run script (starts both backend and frontend)
./run.sh

# Option 2: Run backend and frontend separately
# Terminal 1 - Backend
cd backend && cargo run

# Terminal 2 - Frontend
cd frontend && npm install && npm run dev
```

### Access the Application
- **Frontend Dashboard**: http://0.0.0.0:5000
- **Backend API**: http://localhost:8080
- **Health Check**: http://localhost:8080/health

### Configuration
The system is configured via environment variables. Key settings:
- `ENABLE_PAPER_TRADING=true` - Simulated trading (default)
- `ENABLE_TRADING=false` - Real trading disabled by default
- `SOLANA_NETWORK=devnet` - Network selection
- See `.env.example` for all options

## 📊 API Endpoints

- `GET /health` - Health check
- `GET /portfolio` - Portfolio data with positions and P&L
- `GET /performance` - Trading performance metrics
- `GET /market-data` - Live market data for SOL/USDC, BTC/USDC, ETH/USDC
- `GET /signals` - Recent AI-generated trading signals

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

## 🏗️ Infrastructure (Phase 1 Complete)

### Solana Integration
- ✅ Solana SDK integrated (`solana-client`, `solana-sdk` v1.18)
- ✅ RPC client with automatic fallback support
- ✅ Health monitoring and connection management
- ✅ Paper trading mode for safe development

### Security & Key Management
- ✅ AES-256-GCM encryption for keypairs
- ✅ Argon2id key derivation from passwords
- ✅ Secure wallet generation and storage
- ✅ Hardware wallet ready architecture

### Configuration System
- ✅ Environment variable based configuration
- ✅ Comprehensive `.env` support with validation
- ✅ Multiple RPC endpoint fallbacks
- ✅ Feature flags for gradual rollout

### Monitoring & Alerting
- ✅ Prometheus metrics integration
- ✅ Real-time performance tracking
- ✅ Webhook alerts (Slack, Discord, etc.)
- ✅ System health monitoring
- ✅ Trading metrics (P&L, drawdown, win rate)

### Risk Management
- ✅ Position size validation
- ✅ Maximum drawdown protection
- ✅ Trade validation before execution
- ✅ Kelly criterion position sizing
- ✅ Configurable risk parameters

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

## 🔮 Future Enhancements

- Real Solana blockchain integration with DEX connectivity
- Advanced ML models for price prediction
- WebSocket support for real-time streaming data
- Backtesting engine with historical data
- Multi-strategy support and strategy optimization

## ⚠️ Risk Warning

This is experimental software for educational purposes. Always understand the code and test thoroughly before considering any real trading applications.

## 📄 License

MIT License
