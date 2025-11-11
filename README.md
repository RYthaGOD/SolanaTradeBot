# 🔥 AgentBurn Solana Trader

A sophisticated AI-powered trading system for Solana, built with Rust and React.

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

## 🏃‍♂️ Running the Application

The application automatically starts both backend and frontend servers:

```bash
./run.sh
```

Access the application:
- **Frontend Dashboard**: http://0.0.0.0:5000
- **Backend API**: http://localhost:8080

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
