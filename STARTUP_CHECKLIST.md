# 🚀 Production Startup Checklist

Quick reference for starting the system in paper trading mode.

## ✅ Pre-Start Checklist

- [ ] Rust 1.70+ installed (`rustc --version`)
- [ ] `.env` file created from `.env.example`
- [ ] `DRY_RUN_MODE=true` in `.env`
- [ ] `ENABLE_TRADING=false` in `.env`
- [ ] Project built: `cargo build --release`

## 🏃 Quick Start

```bash
cd backend
cargo run --release --bin agentburn-backend
```

## ✅ Expected Startup Output

You should see:
```
🧪 DRY-RUN MODE ENABLED - NO REAL TRADES WILL BE EXECUTED
🔒 Trading is DISABLED by default for safety
✅ All startup validation checks passed
🌐 Starting Warp server on :8080
🌐 Starting AI-Orchestrated API v2 on port 8081
```

## 🔍 Verification Steps

1. **Health Check**:
   ```bash
   curl http://localhost:8080/health
   ```

2. **Check Paper Trading Status**:
   ```bash
   curl http://localhost:8080/safety/status
   ```

3. **Verify Portfolio** (should show 10 SOL after first trade):
   ```bash
   curl http://localhost:8080/portfolio
   ```

## 📊 Key Endpoints

- `GET /health` - System health
- `GET /portfolio` - Portfolio data
- `GET /performance` - Performance metrics
- `GET /safety/status` - Safety configuration
- `POST /trading-toggle` - Enable/disable trading

## ⚠️ Important Notes

- Paper trading starts with **10 SOL** balance
- First trade initializes the paper trading system
- All trades are simulated - no real funds used
- Dashboard shows paper trading indicator when enabled

## 🐛 Troubleshooting

- **Port in use**: Kill process on port 8080/8081
- **No trades**: Check `ENABLE_TRADING` or enable via API
- **Balance not showing**: Wait for first trade to initialize

See `PRODUCTION_GUIDE.md` for detailed information.

