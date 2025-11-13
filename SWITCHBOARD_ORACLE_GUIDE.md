# Switchboard Oracle Integration Guide

## Overview

This system integrates with **Switchboard Oracle** for real-time, decentralized price feeds on Solana. Switchboard provides accurate, tamper-resistant price data directly from the Solana blockchain.

## Documentation

- **Official Docs**: https://docs.switchboard.xyz/
- **Solana Feeds**: https://docs.switchboard.xyz/docs/solana/feeds
- **Explorer**: https://app.switchboard.xyz/solana/mainnet

## Feed Addresses (Mainnet-Beta)

The system uses official Switchboard V2 aggregator addresses:

| Symbol | Feed Address | Description |
|--------|-------------|-------------|
| SOL/USD | `GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR` | Solana price feed |
| BTC/USD | `8SXvChNYFhRq4EZuZvnhjrB3jJRQCv4k3P4W6hesH3Ee` | Bitcoin price feed |
| ETH/USD | `JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB` | Ethereum price feed |
| USDC/USD | `En8hkHLkRe9d9DraYmBTrus518BvmVH448YcvmrFM6Ce` | USDC price feed |

## Configuration

### Development Mode (Simulated Data)

By default, the system uses simulated oracle data for development:

```rust
let client = SwitchboardClient::new_simulated();
```

This mode:
- ✅ Works without RPC configuration
- ✅ Fast for development and testing
- ✅ Realistic price ranges and movements
- ❌ Not suitable for production trading

### Production Mode (Real Switchboard Data)

To use real on-chain Switchboard data:

**1. Set Solana RPC URL in `.env`:**

```bash
# Use a high-performance RPC provider for production
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
# Or use premium providers:
# SOLANA_RPC_URL=https://solana-api.projectserum.com
# SOLANA_RPC_URL=https://rpc.ankr.com/solana
```

**2. Enable real oracle mode:**

```rust
let rpc_url = std::env::var("SOLANA_RPC_URL")
    .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
let client = SwitchboardClient::new_production(rpc_url);
```

## Usage Examples

### Fetch Single Price

```rust
use crate::switchboard_oracle::SwitchboardClient;

let client = SwitchboardClient::new_simulated();
let feed = client.fetch_price("SOL/USD").await?;

println!("Price: ${}", feed.price);
println!("Confidence: ±${}", feed.confidence);
println!("Range: ${} - ${}", feed.min_price, feed.max_price);
println!("24h Change: {:+.2}%", feed.price_change_24h.unwrap_or(0.0));
```

### Fetch Multiple Feeds

```rust
let symbols = vec!["SOL/USD".to_string(), "BTC/USD".to_string(), "ETH/USD".to_string()];
let feeds = client.fetch_multiple_feeds(&symbols).await?;

for feed in feeds {
    println!("{}: ${:.2}", feed.symbol, feed.price);
}
```

### Check Data Freshness

```rust
let feed = client.fetch_price("SOL/USD").await?;

// Check if data is less than 60 seconds old
if SwitchboardClient::is_data_fresh(&feed, 60) {
    println!("Oracle data is fresh!");
} else {
    println!("Warning: Oracle data may be stale");
}
```

### Calculate Price Changes

```rust
let old_feed = client.fetch_price("SOL/USD").await?;
tokio::time::sleep(Duration::from_secs(300)).await; // Wait 5 minutes
let new_feed = client.fetch_price("SOL/USD").await?;

let change = SwitchboardClient::calculate_price_change(old_feed.price, new_feed.price);
println!("Price changed by {:.2}%", change);
```

### Use Oracle Aggregator

```rust
use crate::switchboard_oracle::OracleAggregator;

let aggregator = OracleAggregator::new(rpc_url);

// Get simple price
let price = aggregator.get_aggregated_price("SOL/USD").await?;

// Get price with confidence interval
let (price, confidence) = aggregator.get_price_with_confidence("SOL/USD").await?;
println!("Price: ${} ± ${}", price, confidence);
```

## Adding Custom Feeds

You can add custom Switchboard feeds:

```rust
let mut client = SwitchboardClient::new_simulated();

// Add custom feed address from Switchboard explorer
client.add_feed(
    "USDT/USD".to_string(),
    "ETAaeeuQBwsh9mWwxc9Pvt3J6E4SzTkBMlYZxXfYTv9n".to_string()
);

let feed = client.fetch_price("USDT/USD").await?;
```

## Data Structure

### OracleFeed

```rust
pub struct OracleFeed {
    pub feed_address: String,    // Switchboard feed public key
    pub symbol: String,           // Trading pair (e.g., "SOL/USD")
    pub price: f64,              // Current price
    pub confidence: f64,         // Confidence interval (±)
    pub min_price: f64,          // price - confidence
    pub max_price: f64,          // price + confidence
    pub timestamp: i64,          // Unix timestamp
    pub slot: u64,               // Solana slot number
    pub price_change_24h: Option<f64>,  // 24h % change
}
```

## Production Deployment

### Recommended Setup

1. **Use Premium RPC Provider**
   - High rate limits (e.g., 100+ req/sec)
   - Low latency (<100ms)
   - High availability (99.9%+ uptime)
   - Examples: Helius, QuickNode, Triton

2. **Implement Caching**
   ```rust
   // Cache oracle data for 10-30 seconds
   // Reduces RPC calls and improves performance
   ```

3. **Monitor Data Quality**
   - Check `confidence` values (lower is better)
   - Verify `timestamp` freshness (<60 seconds)
   - Log any failed fetches for debugging

4. **Handle Errors Gracefully**
   ```rust
   match client.fetch_price("SOL/USD").await {
       Ok(feed) => {
           if SwitchboardClient::is_data_fresh(&feed, 60) {
               // Use price
           } else {
               // Use fallback or cached data
           }
       }
       Err(e) => {
           log::error!("Oracle fetch failed: {}", e);
           // Use fallback price source
       }
   }
   ```

### Performance Optimization

1. **Batch Requests**
   ```rust
   // Fetch multiple feeds in parallel
   let symbols = vec!["SOL/USD", "BTC/USD", "ETH/USD"];
   let feeds = client.fetch_multiple_feeds(&symbols).await?;
   ```

2. **Rate Limiting**
   - Public RPC: Max ~10 req/sec
   - Premium RPC: Max 100+ req/sec
   - Implement exponential backoff on errors

3. **Connection Pooling**
   - Reuse RpcClient instances
   - Configure connection timeouts
   - Use keepalive connections

## Migration from Simulated to Real Data

**Step 1: Test with Devnet**
```bash
SOLANA_RPC_URL=https://api.devnet.solana.com
```

**Step 2: Test with Mainnet (Read-Only)**
```bash
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
```

**Step 3: Switch to Premium RPC**
```bash
SOLANA_RPC_URL=https://rpc.helius.xyz/?api-key=YOUR_KEY
```

**Step 4: Enable Production Mode**
```rust
let use_real_oracle = std::env::var("USE_REAL_ORACLE")
    .unwrap_or_else(|_| "false".to_string()) == "true";

let client = if use_real_oracle {
    SwitchboardClient::new_production(rpc_url)
} else {
    SwitchboardClient::new_simulated()
};
```

## Troubleshooting

### "Failed to fetch account" Error
- **Cause**: RPC connection issue or invalid feed address
- **Solution**: 
  - Verify RPC URL is accessible
  - Check feed address on Switchboard explorer
  - Ensure network connectivity

### "Invalid Switchboard account data" Error
- **Cause**: Account data parsing issue
- **Solution**: 
  - Verify feed address is a Switchboard V2 aggregator
  - Update to latest Switchboard SDK version
  - Check account data size (should be >200 bytes)

### Stale Data Warning
- **Cause**: Oracle hasn't updated recently
- **Solution**:
  - Check Switchboard explorer for feed status
  - Verify oracle heartbeat settings
  - Consider using backup price source

### High Latency
- **Cause**: Slow RPC provider or network issues
- **Solution**:
  - Switch to premium RPC provider
  - Enable connection pooling
  - Implement local caching

## Additional Resources

- **Switchboard Discord**: https://discord.com/invite/switchboardxyz
- **GitHub**: https://github.com/switchboard-xyz
- **Rust SDK**: https://crates.io/crates/switchboard-solana
- **Price Explorer**: https://app.switchboard.xyz/solana/mainnet

## Security Considerations

1. **Validate Confidence Intervals**
   - Only use prices with acceptable confidence (<1% for stable assets)
   - Implement confidence-based trade filtering

2. **Check Data Freshness**
   - Reject prices older than 60 seconds for trading
   - Use staleness detection in production

3. **Multiple Oracle Sources**
   - Consider aggregating from multiple oracles
   - Implement outlier detection
   - Use median pricing for critical decisions

4. **RPC Security**
   - Use HTTPS connections only
   - Store RPC URLs in environment variables
   - Rotate API keys regularly

---

**Status**: ✅ Integration complete with both simulated and real oracle modes supported.
