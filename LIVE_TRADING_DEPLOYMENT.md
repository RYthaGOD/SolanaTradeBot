# Live Trading Deployment Guide

## ⚠️ CRITICAL WARNINGS

**BEFORE DEPLOYING WITH REAL MONEY:**

1. **Start Small**: Begin with minimal capital (e.g., $100-500) to test the system
2. **Backtest First**: Run comprehensive backtests to validate strategies
3. **Monitor Closely**: Watch the system 24/7 for the first week
4. **Set Limits**: Use production safeguards to limit exposure
5. **Emergency Stop**: Know how to immediately halt trading
6. **Test Environment**: Test all components in devnet/testnet first

## Pre-Deployment Checklist

### 1. Backtesting ✅

Run comprehensive backtests before going live:

```bash
cd backend
cargo run --bin backtest 10000 30
# Args: initial_balance days
```

**Review:**
- Win rate should be > 55%
- Profit factor should be > 1.5
- Max drawdown should be < 20%
- Sharpe ratio should be > 1.0

### 2. Production Safeguards Configuration

Edit `.env` or set environment variables:

```bash
# Production Safety Limits
MAX_POSITION_SIZE_PCT=0.05          # 5% max per position
MAX_TOTAL_EXPOSURE_PCT=0.30         # 30% max total exposure
MAX_DAILY_LOSS_PCT=0.05             # 5% max daily loss
MAX_DRAWDOWN_PCT=0.20               # 20% max drawdown
MIN_CONFIDENCE=0.75                 # 75% minimum confidence
MAX_TRADES_PER_DAY=50               # Max 50 trades/day
MAX_TRADES_PER_HOUR=10              # Max 10 trades/hour
EMERGENCY_STOP_ENABLED=true          # Enable emergency stop
REQUIRE_CONFIRMATION_ABOVE_PCT=0.10 # Require confirmation for >10% positions
```

### 3. Wallet Configuration

**CRITICAL**: Use a dedicated trading wallet with limited funds:

```bash
# Generate or load wallet
# Store private key securely (encrypted)
WALLET_KEY_PATH=./wallet.json
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
```

**Security Best Practices:**
- Use a hardware wallet for main funds
- Use a hot wallet with only trading capital
- Enable wallet encryption
- Store backup securely offline
- Never commit wallet files to git

### 4. PDA Setup

Ensure your Program Derived Address (PDA) is properly configured:

```bash
# PDA should be derived from your program
# Balance will be synced automatically
# Verify PDA has sufficient funds for trading
```

### 5. API Keys

Configure all required API keys:

```bash
# DeepSeek AI (optional but recommended)
DEEPSEEK_API_KEY=sk-...

# Twitter Sentiment (optional)
TWITTER_SENTIMENT_SERVICE_URL=http://localhost:8000

# Solana RPC (required)
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
# OR use a premium RPC provider for better reliability
```

### 6. Risk Management

Verify risk manager settings:

```bash
# Initial capital (will be synced from PDA)
INITIAL_CAPITAL=10000.0

# Max drawdown threshold
MAX_DRAWDOWN=0.20  # 20%
```

## Deployment Steps

### Step 1: Test on Devnet

```bash
# Switch to devnet
export SOLANA_RPC_URL=https://api.devnet.solana.com

# Run with minimal capital
export INITIAL_CAPITAL=100.0
export MAX_POSITION_SIZE_PCT=0.01  # 1% for testing

# Start system
cargo run --bin agentburn-backend
```

**Verify:**
- ✅ Trades execute successfully
- ✅ Balance syncs correctly
- ✅ Risk limits are enforced
- ✅ Emergency stop works
- ✅ All logs are clear

### Step 2: Test on Mainnet with Minimal Capital

```bash
# Switch to mainnet
export SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Start with VERY small capital
export INITIAL_CAPITAL=100.0  # $100 test
export MAX_POSITION_SIZE_PCT=0.01  # 1% positions only

# Enable trading
export TRADING_ENABLED=true

# Start system
cargo run --bin agentburn-backend
```

**Monitor for 24-48 hours:**
- Watch all trades execute
- Verify P&L calculations
- Check risk limits are working
- Ensure no unexpected behavior

### Step 3: Gradual Scale-Up

Once confident, gradually increase capital:

**Week 1:** $100-500
**Week 2:** $500-1,000 (if profitable)
**Week 3:** $1,000-5,000 (if still profitable)
**Week 4+:** Scale based on performance

**NEVER** increase capital if:
- Drawdown exceeds 15%
- Win rate drops below 50%
- Any unexpected errors occur

## Production Monitoring

### Real-Time Monitoring

The system provides several monitoring endpoints:

```bash
# Health check
curl http://localhost:3030/health

# Trading status
curl http://localhost:3030/api/trading/status

# Portfolio
curl http://localhost:3030/api/portfolio

# Performance metrics
curl http://localhost:3030/api/performance
```

### Log Monitoring

Watch logs in real-time:

```bash
# Follow logs
tail -f logs/agentburn-backend.log

# Filter for trades
tail -f logs/agentburn-backend.log | grep "TRADE"

# Filter for errors
tail -f logs/agentburn-backend.log | grep "ERROR"
```

### Key Metrics to Monitor

1. **Daily P&L**: Should be positive over time
2. **Win Rate**: Should stay above 55%
3. **Drawdown**: Should never exceed 20%
4. **Trade Frequency**: Should respect limits
5. **Balance**: Should sync correctly with PDA
6. **Error Rate**: Should be minimal

## Emergency Procedures

### Emergency Stop

If something goes wrong, immediately:

1. **Stop the system:**
   ```bash
   # Send SIGTERM
   pkill -TERM agentburn-backend
   ```

2. **Or use API:**
   ```bash
   curl -X POST http://localhost:3030/api/trading/emergency-stop
   ```

3. **Or set environment variable:**
   ```bash
   export TRADING_ENABLED=false
   ```

### Manual Position Closure

If you need to close positions manually:

```bash
# Use Solana CLI or web wallet
# Close all open positions
# Withdraw remaining funds to secure wallet
```

### Recovery Procedures

After emergency stop:

1. **Review logs** to identify issue
2. **Fix the problem** in code
3. **Test thoroughly** before restarting
4. **Restart with reduced capital** if needed

## Performance Optimization

### RPC Provider Selection

Use a premium RPC provider for better reliability:

```bash
# Recommended providers:
# - Helius (https://helius.dev)
# - QuickNode (https://quicknode.com)
# - Alchemy (https://alchemy.com)

export SOLANA_RPC_URL=https://your-premium-rpc-url.com
```

### Fee Optimization

The system includes automatic fee optimization:
- Monitors network congestion
- Adjusts fees based on confirmation time
- Optimizes for cost vs speed

### Database Optimization

For high-frequency trading:

```bash
# Use PostgreSQL instead of SQLite
export DATABASE_URL=postgresql://user:pass@localhost/agentburn
```

## Scaling Strategy

### Phase 1: Manual Monitoring (Weeks 1-4)
- Monitor 24/7
- Manual intervention if needed
- Small capital ($100-1,000)

### Phase 2: Semi-Automated (Months 2-3)
- Daily monitoring
- Automated alerts
- Medium capital ($1,000-10,000)

### Phase 3: Fully Automated (Month 4+)
- Weekly monitoring
- Full automation
- Larger capital (based on performance)

## Risk Management Best Practices

1. **Never risk more than you can afford to lose**
2. **Diversify across multiple strategies**
3. **Set strict stop-losses**
4. **Review performance weekly**
5. **Adjust limits based on market conditions**
6. **Keep emergency fund separate**

## Troubleshooting

### Common Issues

**Issue: Trades not executing**
- Check RPC connection
- Verify wallet has sufficient balance
- Check trading is enabled
- Review risk manager logs

**Issue: Balance not syncing**
- Verify PDA is correct
- Check RPC connection
- Review sync logs

**Issue: High drawdown**
- Review recent trades
- Check market conditions
- Consider reducing position sizes
- Enable emergency stop if needed

**Issue: Too many trades**
- Reduce max_trades_per_day
- Increase min_confidence
- Review signal generation logic

## Support and Resources

- **Logs**: Check `logs/` directory
- **Metrics**: Use API endpoints
- **Backtesting**: Run `cargo run --bin backtest`
- **Documentation**: See other `.md` files

## Legal and Compliance

**IMPORTANT:**
- Ensure compliance with local regulations
- Report trading activity as required
- Pay taxes on profits
- Consult with financial advisor if needed

## Final Checklist Before Going Live

- [ ] Backtests show positive results
- [ ] Production safeguards configured
- [ ] Wallet secured and backed up
- [ ] Minimal capital allocated ($100-500)
- [ ] Monitoring set up
- [ ] Emergency stop tested
- [ ] All API keys configured
- [ ] RPC provider selected
- [ ] Logs directory created
- [ ] Monitoring dashboard ready
- [ ] Support contacts ready
- [ ] Legal compliance verified

## Post-Deployment

### Week 1
- Monitor 24/7
- Review every trade
- Document any issues
- Adjust limits if needed

### Week 2-4
- Daily monitoring
- Weekly performance review
- Adjust strategy if needed
- Scale capital if profitable

### Month 2+
- Weekly monitoring
- Monthly performance review
- Continuous optimization
- Scale based on results

---

**Remember**: Trading involves risk. Never invest more than you can afford to lose. Start small, test thoroughly, and scale gradually.

