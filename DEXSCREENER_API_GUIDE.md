# DEX Screener API Integration Guide

## Overview

This system integrates with **DEX Screener API** for real-time token discovery, price tracking, and trading opportunity analysis across multiple decentralized exchanges.

## Official Documentation

- **API Docs**: https://docs.dexscreener.com/api/reference
- **Website**: https://dexscreener.com
- **Rate Limit**: 300 requests per minute
- **Base URL**: `https://api.dexscreener.com/latest`

## Implementation References

This integration is based on:
- **hedgey-finance/dexscreener-api**: https://github.com/hedgey-finance/dexscreener-api
- **vincentkoc/dexscraper**: https://github.com/vincentkoc/dexscraper

## Supported Chains

- Solana (chain_id: "solana")
- Ethereum, BSC, Polygon, Avalanche, Arbitrum, etc.
- 50+ blockchain networks supported

## API Endpoints Implemented

### 1. Search Tokens
```
GET /dex/search/?q={query}
```

**Usage:**
```rust
let client = DexScreenerClient::new();
let pairs = client.search_tokens("BONK").await?;

for pair in pairs {
    println!("{}: ${}", pair.base_token.symbol, pair.price_usd.unwrap_or_default());
}
```

### 2. Get Token Pairs
```
GET /dex/tokens/{tokenAddresses}
```

**Supports Multiple Addresses (comma-separated):**
```rust
// Single token
let pairs = client.get_token_pairs("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").await?;

// Multiple tokens
let addresses = vec![
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
    "So11111111111111111111111111111111111111112".to_string(),
];
let all_pairs = client.get_multiple_token_pairs(&addresses).await?;
```

### 3. Get Pair by Address
```
GET /dex/pairs/{chainId}/{pairAddresses}
```

**Usage:**
```rust
// Single pair
let pair = client.get_pair(
    "solana",
    "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6"
).await?;

// Solana-specific convenience method
let pairs = client.get_solana_pairs(&[
    "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".to_string()
]).await?;
```

### 4. Get Latest Boosted Tokens
```
GET /token-profiles/latest/v1
```

**Returns trending/boosted tokens:**
```rust
let boosted = client.get_latest_boosted_tokens().await?;
println!("Found {} boosted tokens", boosted.len());
```

### 5. Get Token Orders
```
GET /orders/{tokenAddress}
```

**Order book data:**
```rust
let orders = client.get_token_orders("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").await?;
```

## Data Structures

### TokenPair
```rust
pub struct TokenPair {
    pub chain_id: String,              // "solana"
    pub dex_id: String,                // "raydium", "orca", etc.
    pub url: String,                   // DexScreener URL
    pub pair_address: String,          // Pair contract address
    pub base_token: Token,             // Token being traded
    pub quote_token: Token,            // Quote currency (usually SOL/USDC)
    pub price_native: String,          // Price in quote token
    pub price_usd: Option<String>,     // Price in USD
    pub txns: Transactions,            // Buys/sells data
    pub volume: Volume,                // 24h, 6h, 1h, 5m volume
    pub liquidity: Liquidity,          // Liquidity in USD/base/quote
    pub fdv: Option<f64>,              // Fully diluted valuation
    pub price_change: PriceChange,     // % changes across timeframes
    pub pair_created_at: Option<i64>,  // Unix timestamp
}
```

### Volume & Price Change
```rust
pub struct Volume {
    pub m5: f64,   // 5-minute volume
    pub h1: f64,   // 1-hour volume
    pub h6: f64,   // 6-hour volume
    pub h24: f64,  // 24-hour volume
}

pub struct PriceChange {
    pub m5: f64,   // 5-minute % change
    pub h1: f64,   // 1-hour % change
    pub h6: f64,   // 6-hour % change
    pub h24: f64,  // 24-hour % change
}
```

### Transactions
```rust
pub struct Transactions {
    pub m5: TransactionCount,   // Last 5 minutes
    pub h1: TransactionCount,   // Last hour
    pub h6: TransactionCount,   // Last 6 hours
    pub h24: TransactionCount,  // Last 24 hours
}

pub struct TransactionCount {
    pub buys: i32,    // Number of buy transactions
    pub sells: i32,   // Number of sell transactions
}
```

## Rate Limiting

**Built-in Rate Limiter:**
- Automatic tracking of requests per minute
- Enforces 300 req/min limit
- Auto-waits when limit reached
- Thread-safe implementation

```rust
// Rate limiting is automatic
let client = DexScreenerClient::new();

// These calls are rate-limited automatically
for i in 0..500 {
    let pairs = client.search_tokens(&format!("token{}", i)).await?;
    // Will automatically pause when hitting 300 req/min
}
```

## Finding Trading Opportunities

### Method 1: Search-Based Discovery
```rust
let client = DexScreenerClient::new();

// Search for specific token
let pairs = client.search_tokens("BONK").await?;

// Filter Solana pairs only
let solana_pairs: Vec<_> = pairs.into_iter()
    .filter(|p| p.chain_id == "solana")
    .collect();
```

### Method 2: Trending Tokens
```rust
// Find trending Solana tokens with minimum liquidity
let trending = client.find_trending_solana_tokens(50000.0).await?;

for pair in trending {
    println!("Trending: {} - ${} - Volume: ${}", 
        pair.base_token.symbol,
        pair.price_usd.unwrap_or_default(),
        pair.volume.h24
    );
}
```

### Method 3: Opportunity Analysis
```rust
// Get pairs
let pairs = client.search_tokens("SOL").await?;

// Analyze for opportunities
let opportunities = client.analyze_opportunities(pairs).await;

// Sort by score
let mut sorted = opportunities;
sorted.sort_by(|a, b| b.opportunity_score.partial_cmp(&a.opportunity_score).unwrap());

// Top 10 opportunities
for opp in sorted.iter().take(10) {
    println!("Score: {:.1} - {} - Signals: {:?}",
        opp.opportunity_score,
        opp.token_symbol,
        opp.signals
    );
}
```

## Opportunity Scoring Algorithm

**Weighted Multi-Factor Analysis (0-100 scale):**

1. **Momentum (30%)** - Multi-timeframe price changes
   - 5m, 1h, 6h momentum
   - Consistency bonus (same direction)
   
2. **Volume (25%)** - Trading activity
   - 24h volume relative to historical
   - Volume acceleration (5m vs 1h)
   
3. **Liquidity (25%)** - Market depth
   - Log-scale scoring for better differentiation
   - Higher liquidity = higher score
   
4. **Sentiment (20%)** - Buy/sell pressure
   - Buy/sell ratio from transaction data
   - Consistent buying = positive signal

**Example Calculation:**
```rust
let score = DexScreenerClient::calculate_opportunity_score(&pair);
// Returns: 75.5 (out of 100)
```

## Signal Generation

**Automatic signal detection:**
```rust
let opportunities = client.analyze_opportunities(pairs).await;

for opp in opportunities {
    if opp.opportunity_score > 60.0 {
        println!("Strong opportunity: {}", opp.token_symbol);
        println!("Signals: {:?}", opp.signals);
        // Signals might include:
        // - "Strong upward momentum"
        // - "High volume spike"
        // - "Deep liquidity"
        // - "Bullish sentiment"
    }
}
```

## Advanced Filtering

### Filter by Liquidity
```rust
let pairs = client.search_tokens("meme").await?;
let liquid_pairs: Vec<_> = pairs.into_iter()
    .filter(|p| p.liquidity.usd.unwrap_or(0.0) > 100000.0)
    .collect();
```

### Filter by Volume
```rust
let high_volume: Vec<_> = pairs.into_iter()
    .filter(|p| p.volume.h24 > 1000000.0)
    .collect();
```

### Filter by Price Change
```rust
let pumping: Vec<_> = pairs.into_iter()
    .filter(|p| p.price_change.h24 > 50.0) // 50%+ gain
    .collect();
```

### Filter by Buy/Sell Ratio
```rust
let bullish: Vec<_> = pairs.into_iter()
    .filter(|p| {
        let buys = p.txns.h24.buys as f64;
        let sells = p.txns.h24.sells as f64;
        buys > sells * 1.5 // 1.5x more buys than sells
    })
    .collect();
```

## Error Handling

```rust
match client.search_tokens("BONK").await {
    Ok(pairs) => {
        if pairs.is_empty() {
            println!("No pairs found");
        } else {
            println!("Found {} pairs", pairs.len());
        }
    }
    Err(e) => {
        log::error!("DEX Screener error: {}", e);
        // Handle specific errors:
        // - Rate limit exceeded
        // - Network timeout
        // - Invalid response
        // - Token not found
    }
}
```

## Performance Optimization

### 1. Batch Requests
```rust
// Instead of multiple single requests:
// BAD:
for address in addresses {
    let pairs = client.get_token_pairs(&address).await?;
}

// GOOD: Use batch request
let pairs = client.get_multiple_token_pairs(&addresses).await?;
```

### 2. Cache Results
```rust
// Cache frequently accessed data
let mut cache: HashMap<String, Vec<TokenPair>> = HashMap::new();

let pairs = if let Some(cached) = cache.get("BONK") {
    cached.clone()
} else {
    let pairs = client.search_tokens("BONK").await?;
    cache.insert("BONK".to_string(), pairs.clone());
    pairs
};
```

### 3. Parallel Requests
```rust
use futures::future::join_all;

let queries = vec!["BONK", "WIF", "MYRO"];
let futures = queries.iter().map(|q| client.search_tokens(q));
let results = join_all(futures).await;
```

## Integration with Trading System

### 1. Provider Integration
```rust
// In specialized_providers.rs
let dex_client = DexScreenerClient::new();
let opportunities = dex_client.find_trending_solana_tokens(50000.0).await?;

for opp in opportunities {
    // Generate trading signal
    let signal = TradingSignal {
        symbol: opp.base_token.symbol,
        action: determine_action(&opp),
        confidence: calculate_confidence(&opp),
        // ...
    };
}
```

### 2. Historical Data Collection
```rust
// Fetch and store for analysis
loop {
    let pairs = client.search_tokens("SOL").await?;
    
    for pair in pairs {
        historical_data.add_price_point(
            &pair.base_token.symbol,
            pair.price_usd.parse().unwrap_or(0.0)
        ).await;
    }
    
    tokio::time::sleep(Duration::from_secs(300)).await; // 5 minutes
}
```

### 3. Real-Time Monitoring
```rust
// Monitor specific tokens
let watch_list = vec!["BONK", "WIF", "MYRO"];

loop {
    for token in &watch_list {
        let pairs = client.search_tokens(token).await?;
        
        if let Some(pair) = pairs.first() {
            // Check for significant changes
            if pair.price_change.m5.abs() > 5.0 {
                log::warn!("Alert: {} moved {:.2}% in 5m", 
                    token, pair.price_change.m5);
            }
        }
    }
    
    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

## Troubleshooting

### Rate Limit Errors
```
Error: DEX Screener API error 429: Too Many Requests
```
**Solution**: Built-in rate limiter handles this automatically. If you still see this, reduce concurrent requests.

### Empty Results
```
Found 0 pairs for query: XYZ
```
**Solution**: 
- Check token symbol spelling
- Token might not be listed on any DEX
- Try searching by contract address instead

### Timeout Errors
```
Error: Request failed: operation timed out
```
**Solution**:
- Check network connectivity
- DEX Screener might be under high load
- Implement retry logic with exponential backoff

### Invalid Response
```
Error: Failed to parse response: missing field 'pairs'
```
**Solution**:
- API response format changed
- Check for API updates
- Enable debug logging to see raw response

## Best Practices

1. **Always check liquidity** before trading
   ```rust
   if pair.liquidity.usd.unwrap_or(0.0) < 10000.0 {
       log::warn!("Low liquidity, skip");
       continue;
   }
   ```

2. **Verify price changes** across multiple timeframes
   ```rust
   let consistent = pair.price_change.m5 > 0.0 
       && pair.price_change.h1 > 0.0 
       && pair.price_change.h24 > 0.0;
   ```

3. **Monitor transaction counts**
   ```rust
   let healthy_activity = pair.txns.h24.buys + pair.txns.h24.sells > 100;
   ```

4. **Use opportunity scoring**
   ```rust
   let score = DexScreenerClient::calculate_opportunity_score(&pair);
   if score > 70.0 {
       // High-quality opportunity
   }
   ```

## Additional Resources

- **API Status**: https://status.dexscreener.com
- **Discord**: https://discord.gg/dexscreener
- **Twitter**: https://twitter.com/dexscreener
- **Telegram**: https://t.me/dexscreener

---

**Status**: ✅ Full DEX Screener API integration complete with rate limiting, error handling, and opportunity analysis.
