# 🚀 Production Deployment Guide

Complete guide for deploying and running the Solana Trading Bot in production-ready paper trading mode.

## 📋 Pre-Deployment Checklist

### ✅ System Requirements
- **Rust**: 1.70+ (check with `rustc --version`)
- **Node.js**: 18+ (for frontend, optional)
- **OS**: Windows, Linux, or macOS
- **RAM**: Minimum 2GB, recommended 4GB+
- **Disk**: 500MB free space
- **Network**: Stable internet connection for API calls

### ✅ Environment Setup

1. **Clone and navigate to project**:
   ```bash
   cd backend
   ```

2. **Create `.env` file** (copy from `.env.example`):
   ```bash
   cp .env.example .env
   ```

3. **Configure `.env` file**:
   ```env
   # CRITICAL: Start with paper trading
   DRY_RUN_MODE=true
   ENABLE_TRADING=false
   
   # Network (use devnet for testing)
   SOLANA_RPC_URL=https://api.devnet.solana.com
   
   # Optional API keys (for enhanced features)
   # DEEPSEEK_API_KEY=your_key_here
   # MOBULA_API_KEY=your_key_here
   # MORALIS_API_KEY=your_key_here
   # JUPITER_API_KEY=your_key_here
   
   # Logging
   RUST_LOG=info
   ```

## 🏃 Quick Start (Paper Trading Mode)

### Step 1: Build the Project
```bash
cd backend
cargo build --release
```

### Step 2: Run the System
```bash
# On Windows
cargo run --release --bin agentburn-backend

# On Linux/macOS
./target/release/agentburn-backend
```

### Step 3: Verify System is Running

1. **Check logs** - You should see:
   ```
   🧪 DRY-RUN MODE ENABLED - NO REAL TRADES WILL BE EXECUTED
   🔒 Trading is DISABLED by default for safety
   ✅ All startup validation checks passed
   🌐 Starting Warp server on :8080
   🌐 Starting AI-Orchestrated API v2 on port 8081
   ```

2. **Test health endpoint**:
   ```bash
   curl http://localhost:8080/health
   ```

3. **Check dashboard** (if frontend is running):
   - Open `http://localhost:5000` (or your frontend port)
   - Verify "Paper Trading" indicator is visible
   - Check that balance shows 10 SOL

## 📊 System Architecture

### Core Components

1. **Trading Engine** (`trading_engine.rs`)
   - Executes trades (paper or real)
   - Manages portfolio and balance
   - Calculates PnL and ROI

2. **Risk Manager** (`risk_management.rs`)
   - Validates trades before execution
   - Tracks performance metrics
   - Enforces position limits

3. **Signal Marketplace** (`signal_platform.rs`)
   - X402 protocol implementation
   - Signal publishing and execution
   - Provider reputation tracking

4. **AI Orchestrator** (`ai_orchestrator.rs`)
   - Routes requests to appropriate functions
   - Manages 6 specialized provider agents
   - Coordinates reinforcement learning

5. **Live Data Feeds**
   - Switchboard Oracle (price feeds)
   - DEX Screener (token discovery)
   - PumpFun (meme coin tracking)

### API Endpoints

#### Legacy API (Port 8080)
- `GET /health` - System health check
- `GET /portfolio` - Portfolio data
- `GET /performance` - Performance metrics
- `GET /signals` - Trading signals
- `POST /trading-toggle` - Enable/disable trading
- `GET /safety/status` - Safety configuration

#### AI Orchestrated API v2 (Port 8081)
- `GET /health` - Health check
- `POST /orchestrate` - AI-routed function execution
- `POST /execute/{function}` - Direct function execution
- `GET /functions` - List available functions

## 🔧 Configuration

### Paper Trading Settings

**Starting Balance**: 10 SOL (hardcoded in `trading_engine.rs`)
- Automatically initialized on first trade
- Synced with RiskManager for accurate metrics

**Position Sizing**:
- Max 10% of balance per trade
- Kelly Criterion with historical win rate
- Portfolio heat limit: 30% total exposure

**Risk Limits**:
- Max drawdown: 10%
- Min confidence: 50% (75% for auto-execution)
- Trade validation before execution

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DRY_RUN_MODE` | No | `true` | Enable paper trading |
| `ENABLE_TRADING` | No | `false` | Master trading switch |
| `SOLANA_RPC_URL` | No | Devnet | Solana RPC endpoint |
| `DEEPSEEK_API_KEY` | No | - | AI analysis (optional) |
| `MOBULA_API_KEY` | No | - | Enhanced token data (optional) |
| `MORALIS_API_KEY` | No | - | PumpFun prices (optional) |
| `JUPITER_API_KEY` | No | - | Jupiter DEX (optional) |
| `RUST_LOG` | No | `info` | Logging level |

## 📈 Monitoring & Logs

### Log Levels
- `trace` - Very detailed (development only)
- `debug` - Debug information
- `info` - General information (recommended)
- `warn` - Warnings
- `error` - Errors only

### Key Log Messages

**System Startup**:
```
🧪 PAPER TRADING INITIALIZED
   Starting Balance: 10.00000000 SOL
✅ All startup validation checks passed
```

**Trade Execution**:
```
🧪 [PAPER TRADE] Bought 0.5 SOL/USD at $150.00 (cost: $75.00)
📊 Paper trade recorded: BUY 0.5 SOL/USD | Balance: $9.25 | Portfolio: {...}
```

**Signal Processing**:
```
🔍 Found 2 high-confidence signals ready for auto-execution
✅ Signal executed successfully: signal_123
```

### Health Check Monitoring

Monitor the health endpoint regularly:
```bash
# Check system status
curl http://localhost:8080/health | jq

# Expected response:
{
  "success": true,
  "data": {
    "status": "healthy",
    "trades_count": "5",
    "balance": "10.5",
    "dry_run_mode": "true",
    "trading_enabled": "false"
  }
}
```

## 🛡️ Safety Features

### Built-in Protections

1. **Dry-Run Mode** (Default: Enabled)
   - All trades simulated
   - No real funds used
   - Full paper trading with 10 SOL balance

2. **Trading Disabled by Default**
   - Must explicitly enable via API or env var
   - Prevents accidental trading

3. **Risk Management**
   - Position size limits
   - Drawdown protection
   - Trade validation

4. **Status Validation**
   - Startup validation checks
   - Configuration warnings
   - Network detection

### Enabling Real Trading (⚠️ Advanced)

**WARNING**: Only enable after extensive paper trading testing!

1. **Test in devnet first**:
   ```env
   DRY_RUN_MODE=false
   ENABLE_TRADING=true
   SOLANA_RPC_URL=https://api.devnet.solana.com
   ```

2. **Configure wallet**:
   - Set `WALLET_PRIVATE_KEY` in `.env`
   - Or use PDA treasury (recommended)

3. **Start with small amounts**:
   - Monitor first few trades closely
   - Verify all systems working correctly

4. **Mainnet deployment**:
   ```env
   DRY_RUN_MODE=false
   ENABLE_TRADING=true
   SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
   ```

## 🔄 Running 24/7

### Using systemd (Linux)

Create `/etc/systemd/system/solana-trader.service`:
```ini
[Unit]
Description=Solana Trading Bot
After=network.target

[Service]
Type=simple
User=your_user
WorkingDirectory=/path/to/SolanaTradeBot-copilot-merge-all-branch-code/backend
Environment="RUST_LOG=info"
EnvironmentFile=/path/to/.env
ExecStart=/path/to/target/release/agentburn-backend
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable solana-trader
sudo systemctl start solana-trader
sudo systemctl status solana-trader
```

### Using PM2 (Node.js process manager)

```bash
npm install -g pm2
cd backend
pm2 start "cargo run --release --bin agentburn-backend" --name solana-trader
pm2 save
pm2 startup
```

### Using Docker (Optional)

Create `Dockerfile`:
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/agentburn-backend /usr/local/bin/
CMD ["agentburn-backend"]
```

Build and run:
```bash
docker build -t solana-trader .
docker run -d --env-file .env --name trader solana-trader
```

## 🐛 Troubleshooting

### Common Issues

1. **"Port already in use"**
   - Kill existing process: `lsof -ti:8080 | xargs kill` (Linux/macOS)
   - Or change port in code

2. **"Failed to connect to RPC"**
   - Check `SOLANA_RPC_URL` is correct
   - Verify internet connection
   - Try different RPC endpoint

3. **"No trades executing"**
   - Check `ENABLE_TRADING=true` or enable via API
   - Verify signals are being generated
   - Check logs for errors

4. **"Balance not updating"**
   - Ensure paper trading is initialized (first trade)
   - Check RiskManager sync in logs
   - Verify trade execution succeeded

5. **"API errors"**
   - Check API keys are valid (if using)
   - Verify rate limits not exceeded
   - Check circuit breaker status

### Debug Mode

Enable detailed logging:
```env
RUST_LOG=debug
```

Or for specific modules:
```env
RUST_LOG=agentburn_backend::trading_engine=debug,agentburn_backend::signal_platform=debug
```

## 📊 Performance Monitoring

### Key Metrics to Monitor

1. **Trading Metrics**:
   - Total trades executed
   - Win rate
   - Total PnL
   - ROI percentage

2. **System Health**:
   - API response times
   - Error rates
   - Memory usage
   - CPU usage

3. **Signal Quality**:
   - Signals generated per hour
   - Average confidence
   - Execution success rate

### Dashboard Access

If frontend is running:
- Dashboard: `http://localhost:5000`
- Portfolio: Shows paper trading balance
- Performance: Real-time metrics
- Signals: Live signal feed

## 🔐 Security Best Practices

1. **Never commit `.env` file** to version control
2. **Use strong API keys** and rotate regularly
3. **Limit network access** (firewall rules)
4. **Monitor logs** for suspicious activity
5. **Keep dependencies updated**: `cargo update`
6. **Use PDA treasury** instead of direct wallet keys
7. **Enable 2FA** on all API accounts

## 📝 Maintenance

### Regular Tasks

1. **Daily**:
   - Check logs for errors
   - Monitor balance and PnL
   - Verify signals are generating

2. **Weekly**:
   - Review performance metrics
   - Check API key validity
   - Update dependencies if needed

3. **Monthly**:
   - Review and optimize strategies
   - Analyze trade history
   - Update configuration if needed

### Backup

Important data to backup:
- `.env` file (securely)
- Trade history database
- Configuration files
- Log files (optional)

## 🎯 Next Steps

After running in paper mode for a few days:

1. **Review Performance**:
   - Analyze win rate
   - Check PnL trends
   - Identify best strategies

2. **Optimize Configuration**:
   - Adjust position sizes
   - Tune risk parameters
   - Fine-tune signal thresholds

3. **Test Real Trading** (Devnet):
   - Enable real trading on devnet
   - Test with small amounts
   - Verify all systems work

4. **Production Deployment**:
   - Deploy to mainnet
   - Start with minimal capital
   - Gradually increase as confidence grows

## 📞 Support

For issues or questions:
- Check logs first: `RUST_LOG=debug`
- Review this guide
- Check GitHub issues
- Review code comments

---

**Remember**: Always start with paper trading (`DRY_RUN_MODE=true`) and test thoroughly before enabling real trading!

