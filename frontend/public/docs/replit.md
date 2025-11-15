# AgentBurn Solana Trader - Project Documentation

## Overview
AgentBurn is an AI-powered cryptocurrency trading system designed for Solana. The project features a Rust backend with sophisticated trading logic and a React frontend for real-time monitoring. Currently configured with simulated market data for safe development and testing.

## Project State (Created: November 11, 2025)
- Complete MVP implementation with all core features
- Rust backend with trading engine, risk management, and API server
- React frontend with Dashboard, Trading Signals, Portfolio, and Performance views
- Simulated market data for SOL/USDC, BTC/USDC, and ETH/USDC pairs
- Ready for testing and development

## Architecture

### Backend (Rust)
- **main.rs**: Application entry point, spawns async tasks for market simulation and signal generation
- **trading_engine.rs**: Core trading logic with SMA crossover strategy, portfolio management
- **solana_integration.rs**: Market data simulation, trade execution (mocked for development)
- **risk_management.rs**: Position sizing, Kelly criterion, drawdown tracking
- **ml_models.rs**: Confidence scoring and feature engineering for trading signals
- **api.rs**: REST API server using Warp framework

### Frontend (React + TypeScript)
- **Dashboard.tsx**: Overview with portfolio value, market data, and quick stats
- **TradingView.tsx**: Real-time trading signals display
- **Portfolio.tsx**: Holdings visualization with pie chart
- **Performance.tsx**: Performance metrics with charts and analytics

## Key Features
1. Real-time market data simulation (2-second intervals)
2. AI trading signals using SMA-10/SMA-20 crossover strategy
3. Dynamic position sizing with 10% max per trade
4. Risk management with drawdown limits
5. Live portfolio tracking and P&L calculation
6. Performance analytics (Sharpe ratio, win rate, total returns)

## Configuration
- Backend runs on port 8080
- Frontend runs on port 5000 (bound to 0.0.0.0 for Replit compatibility)
- Initial capital: $10,000
- Max drawdown: 10%
- Refresh intervals: Market data (2s), Signals (3s), Dashboard (5s)

## Workflow
The project uses a single workflow that runs `./run.sh`, which:
1. Installs frontend dependencies (npm install)
2. Builds Rust backend (cargo build --release)
3. Starts backend server on port 8080
4. Starts frontend dev server on port 5000
5. Handles graceful shutdown on Ctrl+C

## Future Integration Points
- Real Solana RPC connection for live blockchain data
- Wallet integration for actual trade execution
- DEX integration (Jupiter, Raydium)
- Advanced ML models with trained neural networks
- WebSocket streaming for real-time updates
- Historical data backtesting engine

## User Preferences
- Clean, modern UI with gradient backgrounds
- Real-time updates without manual refresh
- Clear risk warnings about simulated environment
- Educational focus on understanding trading strategies

## Development Notes
- Uses Rust 2021 edition
- Frontend built with Vite for fast development
- CORS enabled on backend for local development
- Logging enabled via pretty_env_logger
- Type-safe TypeScript throughout frontend
