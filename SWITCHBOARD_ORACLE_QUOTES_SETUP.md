# Switchboard Oracle Quotes Setup Guide

## Overview

This system now uses **Switchboard Oracle Quotes** (Ed25519) - the new standard for oracle data on Solana. Oracle Quotes provide:

- ✅ **90% lower cost** (~0.00015 SOL vs ~0.002 SOL)
- ✅ **<1 second latency** (vs 2-10 seconds)
- ✅ **No account setup required** (no feed accounts to create/fund)
- ✅ **No write locks** (unlimited parallel access)
- ✅ **~485 compute units** (vs 50,000+ for traditional feeds)

**Documentation**: https://docs.switchboard.xyz/oracle-quotes-the-new-standard/oracle-quotes

## Getting Feed Hashes

Oracle Quotes use **feed hashes** instead of feed addresses. To get feed hashes:

1. Visit **Switchboard Explorer**: https://explorer.switchboardlabs.xyz/
2. Search for your desired feed (e.g., "SOL/USD", "BTC/USD")
3. Copy the **feed hash** (format: `0x...` hex string)
4. Update the feed hashes in `backend/src/switchboard_oracle.rs`

## Configuration

### Current Implementation

The system is configured with placeholder feed hashes. To enable Oracle Quotes:

1. **Get actual feed hashes** from https://explorer.switchboardlabs.xyz/
2. **Update `feed_hashes` in `SwitchboardClient::new()`**:

```rust
feed_hashes.insert(
    "SOL/USD".to_string(),
    "0xYOUR_ACTUAL_FEED_HASH_HERE".to_string(), // Replace with actual hash
);
```

### Example Feed Hashes

Replace these placeholders with actual feed hashes from the explorer:

- `SOL/USD`: Get hash from explorer
- `BTC/USD`: Get hash from explorer  
- `ETH/USD`: Get hash from explorer
- `USDC/USD`: Get hash from explorer

## How It Works

### Priority Order

The system tries data sources in this order:

1. **Switchboard Oracle Quotes** (new standard - fastest, cheapest)
2. **Jupiter Quote API** (free, reliable fallback)
3. **Mobula API** (free tier available)
4. **Switchboard Legacy On-Chain** (old method - not recommended)

### API Endpoints

Oracle Quotes are accessed via the **Switchboard Gateway API**:

```
GET https://api.switchboard.xyz/v1/gateway/quotes/{feed_hash}
```

Response format:
```json
{
  "value": 150.25,
  "timestamp": 1234567890,
  "confidence": 0.5
}
```

## Benefits Over Traditional Feeds

| Feature | Oracle Quotes | Traditional Feeds |
|---------|---------------|------------------|
| **Transaction Cost** | ~0.00015 SOL | ~0.002 SOL |
| **Update Latency** | <1 second | 2-10 seconds |
| **Write Locks** | None | Required |
| **Setup Time** | Instant | 5-10 minutes |
| **Parallel Access** | Unlimited | Limited |
| **Compute Units** | ~485 CU | 50,000+ CU |

## Rate Limiting

The system includes rate limiting for Oracle Quotes:
- **Max requests**: 1000 per minute
- **Automatic backoff**: Exponential retry on failures
- **Caching**: 10-second TTL to reduce API calls

## Troubleshooting

### "Oracle Quotes feed hash not configured"

This means the feed hash is still a placeholder. Get the actual feed hash from:
https://explorer.switchboardlabs.xyz/

### "Switchboard Oracle Quotes Gateway API returned error"

- Check network connectivity
- Verify the feed hash is correct
- Check Switchboard API status
- The system will automatically fall back to Jupiter/Mobula APIs

### All Sources Fail

If all data sources fail:
1. Check network connectivity
2. Verify API keys (for Mobula)
3. Check Switchboard API status
4. The system will retry on the next request

## Next Steps

1. ✅ System is configured to use Oracle Quotes (priority #1)
2. ⏳ **Get actual feed hashes** from https://explorer.switchboardlabs.xyz/
3. ⏳ **Update `feed_hashes` in `switchboard_oracle.rs`**
4. ✅ System will automatically use Oracle Quotes once configured

## References

- **Oracle Quotes Docs**: https://docs.switchboard.xyz/oracle-quotes-the-new-standard/oracle-quotes
- **Switchboard Explorer**: https://explorer.switchboardlabs.xyz/
- **Switchboard Gateway API**: https://api.switchboard.xyz/v1/gateway/quotes/{feed_hash}

