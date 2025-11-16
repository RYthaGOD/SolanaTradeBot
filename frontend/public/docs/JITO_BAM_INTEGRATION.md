# Jito BAM (Block Engine + Atomic Marketplace) Integration

## Overview

The SolanaTradeBot now integrates with **Jito Labs' Block Engine** for atomic bundle execution and MEV (Maximal Extractable Value) protection. This integration enables:

- **Atomic Bundle Execution**: Multiple transactions execute together or not at all
- **MEV Protection**: Protects trades from front-running and sandwich attacks
- **Priority Fee Optimization**: Automatic tip distribution to Jito validators
- **Bundle Status Tracking**: Real-time monitoring of bundle execution

## What is Jito?

Jito is a block engine that provides MEV infrastructure for Solana. It allows traders to submit bundles of transactions that are guaranteed to execute atomically (all or nothing), protecting against various MEV attacks.

**Key Benefits:**
- ✅ Atomic execution guarantees
- ✅ Protection from front-running
- ✅ Protection from sandwich attacks
- ✅ Optimized transaction ordering
- ✅ Priority fee management via tips

## Architecture

```
┌─────────────────────────────────────────────────────┐
│         SolanaTradeBot Trading Engine               │
│   (Signal Generation, Risk Management, ML)          │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────┐
│            Jito BAM Client Module                    │
│  - Bundle Builder                                    │
│  - Atomic Transaction Grouping                       │
│  - Priority Tip Management                           │
└──────────────────────┬───────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────┐
│           Jito Block Engine API                      │
│  - Bundle Submission (sendBundle)                    │
│  - Status Tracking (getBundleStatuses)               │
│  - Tip Account Selection                             │
└──────────────────────┬───────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────┐
│              Solana Blockchain                       │
│  - Atomic Bundle Execution                           │
│  - MEV-Protected Transactions                        │
└──────────────────────────────────────────────────────┘
```

## Configuration

### Environment Variables

```bash
# Set RPC URL to determine network (mainnet/devnet)
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Or use devnet for testing
SOLANA_RPC_URL=https://api.devnet.solana.com
```

### Bundle Configuration

The system uses sensible defaults but can be customized:

```rust
use crate::jito_bam::{JitoBamClient, BundleConfig};

// Default configuration
let jito = JitoBamClient::new(true); // true = mainnet

// Custom configuration
let config = BundleConfig {
    tip_amount_lamports: 20_000,  // 0.00002 SOL tip
    max_retries: 5,
    timeout_ms: 60_000,
};
let jito = JitoBamClient::with_config(true, config);
```

## API Endpoints

### 1. Get Jito Status

Get current Jito BAM configuration and status.

```bash
GET /jito/status
```

**Response:**
```json
{
  "success": true,
  "data": {
    "enabled": "true",
    "network": "mainnet",
    "block_engine": "https://mainnet.block-engine.jito.wtf",
    "features": "Atomic bundle execution, MEV protection, priority tips",
    "tip_accounts": "8"
  },
  "message": "Jito BAM status"
}
```

### 2. Check Bundle Status

Check the status of a submitted bundle.

```bash
POST /jito/bundle/status
Content-Type: application/json

{
  "bundle_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "bundle_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "Landed"
  },
  "message": "Bundle status retrieved"
}
```

**Possible statuses:**
- `Pending` - Bundle received, waiting for processing
- `Processing` - Bundle being processed by block engine
- `Landed` - Bundle successfully executed on-chain ✅
- `Failed` - Bundle execution failed ❌
- `Dropped` - Bundle dropped without execution ❌

### 3. Get Random Tip Account

Get a random Jito tip account for priority fees.

```bash
GET /jito/tip-account
```

**Response:**
```json
{
  "success": true,
  "data": {
    "tip_account": "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "total_accounts": "8"
  },
  "message": "Random tip account"
}
```

## Usage Examples

### Example 1: Submit Simple Bundle

```rust
use crate::jito_bam::{JitoBamClient, TradingBundleBuilder};
use solana_sdk::transaction::Transaction;

async fn submit_trade_bundle() -> Result<String, Box<dyn Error>> {
    // Initialize Jito client
    let jito = JitoBamClient::new(true); // mainnet
    
    // Create transactions
    let tx1 = create_swap_transaction()?;
    let tx2 = create_balance_check_transaction()?;
    
    // Build atomic bundle
    let bundle = TradingBundleBuilder::new("Swap with balance check")
        .add_transaction(tx1)
        .add_transaction(tx2)
        .build();
    
    // Submit bundle (all transactions execute or none)
    let bundle_id = jito.submit_bundle(bundle).await?;
    println!("Bundle submitted: {}", bundle_id);
    
    Ok(bundle_id)
}
```

### Example 2: Submit with Retry and Wait

```rust
async fn submit_and_wait() -> Result<(), Box<dyn Error>> {
    let jito = JitoBamClient::new(true);
    
    let bundle = create_trading_bundle();
    
    // Submit with automatic retry
    let bundle_id = jito.submit_bundle_with_retry(bundle).await?;
    println!("Bundle submitted: {}", bundle_id);
    
    // Wait for bundle to land
    let status = jito.wait_for_bundle(&bundle_id).await?;
    
    match status {
        BundleStatus::Landed => println!("✅ Bundle executed successfully!"),
        BundleStatus::Failed => println!("❌ Bundle failed to execute"),
        BundleStatus::Dropped => println!("❌ Bundle was dropped"),
        _ => println!("⏳ Unexpected status: {:?}", status),
    }
    
    Ok(())
}
```

### Example 3: Multi-Step Atomic Trade

```rust
async fn atomic_arbitrage() -> Result<(), Box<dyn Error>> {
    let jito = JitoBamClient::new(true);
    
    // Build multi-step arbitrage bundle
    let bundle = TradingBundleBuilder::new("DEX Arbitrage")
        .add_transaction(buy_on_dex_a())    // Step 1: Buy on DEX A
        .add_transaction(sell_on_dex_b())   // Step 2: Sell on DEX B
        .add_transaction(return_profit())   // Step 3: Return profit
        .build();
    
    // All three transactions execute atomically or none execute
    let bundle_id = jito.submit_bundle(bundle).await?;
    
    // Track execution
    let status = jito.wait_for_bundle(&bundle_id).await?;
    
    if matches!(status, BundleStatus::Landed) {
        println!("✅ Arbitrage completed atomically!");
    } else {
        println!("❌ Arbitrage bundle failed - no funds at risk");
    }
    
    Ok(())
}
```

## Priority Tips

Jito uses a tip-based system for priority fee management. The bot automatically:

1. **Selects random tip account** from 8 mainnet validators
2. **Includes tip transaction** in bundle
3. **Optimizes tip amount** based on network conditions

### Tip Amounts

Default tip: **0.00001 SOL (10,000 lamports)**

You can adjust based on urgency:
- Low priority: 5,000 lamports (0.000005 SOL)
- Normal priority: 10,000 lamports (0.00001 SOL)
- High priority: 50,000 lamports (0.00005 SOL)
- Urgent: 100,000 lamports (0.0001 SOL)

## Use Cases

### 1. MEV-Protected Swaps
Submit swap transactions as bundles to prevent front-running:
```
Bundle: [approve_token, swap_transaction]
Result: Atomic execution prevents MEV attacks
```

### 2. Atomic Arbitrage
Execute multi-step arbitrage atomically:
```
Bundle: [buy_token_a, swap_to_token_b, sell_token_b]
Result: All steps execute or none (no partial execution risk)
```

### 3. Complex DeFi Operations
Chain multiple DeFi operations:
```
Bundle: [deposit_collateral, borrow_asset, swap_asset, repay_loan]
Result: Complex operation executes atomically
```

### 4. Portfolio Rebalancing
Rebalance entire portfolio atomically:
```
Bundle: [sell_asset_1, sell_asset_2, buy_asset_3, buy_asset_4]
Result: Portfolio rebalanced in single atomic operation
```

## Integration with Trading Engine

The Jito BAM client integrates seamlessly with the existing trading engine:

```rust
// In trading_engine.rs
pub async fn execute_protected_trade(&self, signals: Vec<Signal>) {
    let jito = JitoBamClient::new(true);
    
    // Convert signals to transactions
    let txs = signals.iter()
        .map(|signal| self.signal_to_transaction(signal))
        .collect();
    
    // Submit as atomic bundle with MEV protection
    match jito.submit_bundle_with_retry(txs).await {
        Ok(bundle_id) => {
            log::info!("Protected trade submitted: {}", bundle_id);
            // Track bundle status...
        }
        Err(e) => log::error!("Bundle submission failed: {}", e),
    }
}
```

## Testing

### Unit Tests

```bash
cd backend
cargo test jito_bam
```

### Integration Test

```bash
# Set devnet RPC
export SOLANA_RPC_URL=https://api.devnet.solana.com

# Start backend
cargo run

# Test Jito status endpoint
curl http://localhost:8080/jito/status

# Test tip account endpoint
curl http://localhost:8080/jito/tip-account
```

## Monitoring

Monitor Jito bundle execution:

```bash
# Check bundle status
curl -X POST http://localhost:8080/jito/bundle/status \
  -H "Content-Type: application/json" \
  -d '{"bundle_id": "YOUR_BUNDLE_ID"}'
```

## Best Practices

1. **Use for High-Value Trades**: Jito is most beneficial for trades vulnerable to MEV
2. **Monitor Bundle Status**: Always check if bundle landed successfully
3. **Retry on Failure**: Use `submit_bundle_with_retry()` for important operations
4. **Optimize Tips**: Higher tips increase execution priority
5. **Test on Devnet**: Always test bundle logic on devnet first
6. **Bundle Size**: Keep bundles small (2-5 transactions) for better execution

## Limitations

- **Bundle Size**: Maximum ~5 transactions per bundle
- **Timeout**: Bundles expire if not executed within timeout period
- **Network**: Only available on mainnet-beta and devnet
- **Cost**: Requires tip payment (minimum 10,000 lamports)

## Resources

- **Jito Documentation**: https://jito-labs.gitbook.io/mev/
- **Block Engine API**: https://jito-labs.gitbook.io/mev/searcher-services/block-engine
- **Tip Accounts**: https://jito-labs.gitbook.io/mev/mev-payment-and-distribution/on-chain-addresses

## Support

For issues with Jito integration:
1. Check bundle status via API
2. Verify tip account selection
3. Review Jito Labs documentation
4. Check network connectivity to block engine

## Future Enhancements

Planned improvements:
- [ ] Automatic tip optimization based on network conditions
- [ ] Bundle simulation before submission
- [ ] Enhanced retry strategies
- [ ] Bundle analytics and reporting
- [ ] Integration with AI decision-making for optimal bundle composition
