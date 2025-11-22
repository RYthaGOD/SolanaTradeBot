# Backtesting Guide

## Overview

The backtesting framework allows you to test trading strategies on historical data before deploying with real money.

## Quick Start

### Run a Basic Backtest

```bash
cd backend
cargo run --bin backtest 10000 30
```

**Arguments:**
- `10000` - Initial balance in USD
- `30` - Number of days to backtest

### Example Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🧪 BACKTESTING ENGINE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Configuration:
   Initial Balance: $10000.00
   Period: 30 days
   Max Drawdown: 20.0%
   Commission: 0.100%
   Slippage: 0.050%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📈 Generating historical data...
   Generated 720 data points
🚀 Running backtest...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 BACKTEST RESULTS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💰 Performance:
   Initial Balance: $10000.00
   Final Balance: $10500.00
   Total Return: $500.00 (5.00%)
   Max Drawdown: 3.50%

📈 Trading Statistics:
   Total Trades: 45
   Winning Trades: 28 (62.2%)
   Losing Trades: 17
   Avg Win: $45.50
   Avg Loss: -$25.30
   Profit Factor: 2.85

📊 Risk Metrics:
   Sharpe Ratio: 1.25
   Sortino Ratio: 1.85
   Total Fees: $12.50
   Total Slippage: $6.25
```

## Understanding Results

### Key Metrics

**Performance:**
- **Total Return**: Absolute profit/loss in USD
- **Total Return %**: Percentage return on initial capital
- **Max Drawdown**: Largest peak-to-trough decline

**Trading Statistics:**
- **Win Rate**: Percentage of profitable trades
- **Profit Factor**: Ratio of gross profit to gross loss
- **Avg Win/Loss**: Average profit/loss per trade

**Risk Metrics:**
- **Sharpe Ratio**: Risk-adjusted return (higher is better, >1.0 is good)
- **Sortino Ratio**: Downside risk-adjusted return (higher is better)
- **Total Fees**: Cumulative trading fees
- **Total Slippage**: Cumulative slippage costs

### Interpreting Results

**Good Results:**
- Win rate > 55%
- Profit factor > 1.5
- Sharpe ratio > 1.0
- Max drawdown < 20%
- Positive total return

**Warning Signs:**
- Win rate < 50%
- Profit factor < 1.0
- Sharpe ratio < 0.5
- Max drawdown > 25%
- Negative total return

## Advanced Configuration

### Custom Backtest Configuration

Edit `backend/src/backtesting.rs` to customize:

```rust
let config = BacktestConfig {
    initial_balance: 10000.0,
    start_date: Utc::now() - chrono::Duration::days(30),
    end_date: Utc::now(),
    max_drawdown: 0.20,        // 20% max drawdown
    commission_rate: 0.001,    // 0.1% commission
    slippage: 0.0005,          // 0.05% slippage
    min_confidence: 0.6,       // 60% minimum confidence
    max_position_size_pct: 0.1, // 10% max position size
};
```

### Using Real Historical Data

To use real historical data instead of generated data:

1. **Export historical data** from your data source (CSV format):
   ```csv
   timestamp,symbol,price,volume,bid,ask,spread
   1698768000,SOL/USD,100.50,1500000,100.49,100.51,0.02
   ```

2. **Load in backtest:**
   ```rust
   let historical_data = load_from_csv("historical_data.csv")?;
   ```

3. **Run backtest:**
   ```rust
   let results = engine.run(historical_data).await;
   ```

## Best Practices

### 1. Test Multiple Time Periods

Test across different market conditions:
- Bull markets
- Bear markets
- Sideways markets
- High volatility periods

### 2. Test Different Configurations

Vary parameters to find optimal settings:
- Position sizes
- Confidence thresholds
- Stop loss/take profit levels
- Risk limits

### 3. Compare Strategies

Run multiple backtests to compare:
- Different signal generation methods
- Various risk management approaches
- Alternative position sizing algorithms

### 4. Out-of-Sample Testing

- Use 70% of data for training/optimization
- Use 30% for final validation
- Never optimize on test data

### 5. Walk-Forward Analysis

- Test on rolling windows
- Re-optimize periodically
- Validate robustness

## Limitations

### What Backtesting Can't Account For

1. **Market Impact**: Large orders may move prices
2. **Liquidity**: Real markets may have less liquidity
3. **Slippage**: Actual slippage may be higher
4. **Latency**: Real execution has delays
5. **Emotional Factors**: No fear/greed in backtests
6. **Market Regime Changes**: Past performance ≠ future results

### Improving Accuracy

1. **Use realistic slippage** (0.05-0.1% for crypto)
2. **Include all fees** (trading fees, network fees)
3. **Account for market hours** (if applicable)
4. **Consider partial fills** for large orders
5. **Model latency** in execution

## Exporting Results

Results are automatically saved to JSON:

```bash
backtest_results_1698768000.json
```

**Use results for:**
- Performance analysis
- Strategy optimization
- Risk assessment
- Reporting

## Next Steps

After successful backtesting:

1. ✅ Review results carefully
2. ✅ Test on paper trading (if available)
3. ✅ Start with minimal capital
4. ✅ Monitor closely in production
5. ✅ Scale gradually based on results

See `LIVE_TRADING_DEPLOYMENT.md` for production deployment guide.

