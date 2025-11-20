# 🚀 Quick Start: Solana Smart Contract Integration

This guide provides the fastest path to integrate the Solana prediction market smart contract with our trading system.

## ✅ Completed Steps

1. ✅ **Smart Contract Forked** - Cloned roswelly's production-ready contract
2. ✅ **Documentation Created** - Comprehensive setup guide available
3. ✅ **Project Structure** - Smart contract in `./smart-contract/` directory

## 🎯 Current Status

```
Project Structure:
├── backend/                    # Rust trading backend
│   ├── src/prediction_markets.rs    # EV-based signal generation
│   ├── src/api_prediction_only.rs   # Current API (simulated)
│   └── src/main_prediction_only.rs  # Entry point
├── smart-contract/             # 🆕 Anchor smart contract
│   ├── programs/prediction-market/  # Solana program
│   ├── tests/                      # Test suite
│   └── Anchor.toml                 # Configuration
├── frontend/                   # React UI
└── docs/                      # Documentation
```

## 🏃 Quick Integration (3 Steps)

### Step 1: Install Prerequisites (5 minutes)

**Option A: Full Setup** (if you want to build/deploy)
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest
```

**Option B: Connect to Existing** (if contract already deployed)
```bash
# Just install Rust dependencies for client
cd backend
cargo build --features anchor-client
```

### Step 2: Build Smart Contract (2 minutes)

```bash
cd smart-contract

# Install dependencies
yarn install

# Build the program
anchor build

# This generates:
# - target/deploy/prediction_market.so
# - target/idl/prediction_market.json (for integration)
```

### Step 3: Deploy to Devnet (1 minute)

```bash
# Setup devnet wallet
solana config set --url devnet
solana-keygen new --outfile ~/.config/solana/id.json
solana airdrop 2

# Deploy
anchor deploy --provider.cluster devnet

# Program ID: 3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg
```

## 🔗 Backend Integration Options

### Option 1: Quick Test (Use Deployed Contract)

The contract is already deployed on devnet. You can connect directly:

```rust
// backend/src/solana_prediction_client.rs
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

pub struct SolanaPredictionClient {
    rpc_client: RpcClient,
    program_id: Pubkey,
}

impl SolanaPredictionClient {
    pub fn new_devnet() -> Self {
        let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());
        let program_id = "3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg"
            .parse()
            .unwrap();
        
        Self { rpc_client, program_id }
    }
    
    pub async fn get_market(&self, market_pubkey: &Pubkey) -> Result<Market, Error> {
        // Fetch market account from blockchain
        let account = self.rpc_client.get_account(market_pubkey)?;
        // Deserialize Market struct from account.data
        Ok(Market::try_from_slice(&account.data)?)
    }
}
```

### Option 2: Full Integration (Production Ready)

Use anchor-client for complete functionality:

```rust
// Add to backend/Cargo.toml
[dependencies]
anchor-client = "0.31"
anchor-lang = "0.31"

// backend/src/solana_prediction_client.rs
use anchor_client::{Client, Cluster, Program};
use solana_sdk::signature::Keypair;

pub struct SolanaPredictionClient {
    program: Program,
}

impl SolanaPredictionClient {
    pub fn new(keypair: Keypair) -> Self {
        let client = Client::new_with_options(
            Cluster::Devnet,
            Rc::new(keypair),
            CommitmentConfig::confirmed(),
        );
        
        let program_id = "3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg"
            .parse()
            .unwrap();
        
        let program = client.program(program_id);
        
        Self { program }
    }
    
    pub async fn create_market(&self, question: String, end_time: i64) -> Result<Pubkey> {
        // Full transaction handling
        let question_hash = hash(question.as_bytes());
        let (market_pda, _) = Pubkey::find_program_address(
            &[b"market", self.payer.pubkey().as_ref(), &question_hash.to_bytes()],
            &self.program.id(),
        );
        
        self.program
            .request()
            .accounts(prediction_market::accounts::InitializeMarket {
                market: market_pda,
                creator: self.payer.pubkey(),
                system_program: system_program::ID,
            })
            .args(prediction_market::instruction::InitializeMarket {
                question,
                end_time,
                question_hash: question_hash.to_bytes(),
            })
            .send()?;
        
        Ok(market_pda)
    }
}
```

## 🧪 Testing the Integration

### Test 1: Verify Contract Deployment
```bash
# Check if program exists on devnet
solana program show 3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg --url devnet

# View on Solana Explorer
# https://explorer.solana.com/address/3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg?cluster=devnet
```

### Test 2: Create Test Market
```bash
cd smart-contract

# Run test suite
anchor test

# Or run devnet integration test
yarn test:devnet
```

### Test 3: Backend Connection
```bash
cd backend

# Add test for on-chain connection
cargo test test_solana_connection -- --nocapture
```

## 📊 Integration Checklist

### Phase 1: Basic Connection ✅
- [x] Fork smart contract
- [x] Add to project
- [x] Document setup
- [ ] Build contract
- [ ] Deploy to devnet
- [ ] Verify deployment

### Phase 2: Client Integration
- [ ] Add anchor-client dependency
- [ ] Create `solana_prediction_client.rs`
- [ ] Implement market fetching
- [ ] Implement bet placement
- [ ] Test with devnet

### Phase 3: API Integration
- [ ] Update `api_prediction_only.rs`
- [ ] Replace simulated markets with on-chain
- [ ] Update EV calculations with real data
- [ ] Add transaction handling
- [ ] Error handling and retries

### Phase 4: Frontend Updates
- [ ] Add wallet connection (Phantom, Solflare)
- [ ] Update UI for on-chain status
- [ ] Show transaction confirmations
- [ ] Display Solana Explorer links

### Phase 5: Production Ready
- [ ] Security audit
- [ ] Load testing
- [ ] Mainnet deployment
- [ ] Monitoring and alerts

## 🎓 Example: Complete Flow

### 1. Create Market (On-Chain)
```rust
let client = SolanaPredictionClient::new_devnet();
let market_pubkey = client.create_market(
    "Will Bitcoin reach $100K by EOY 2025?".to_string(),
    1735689600, // Unix timestamp
).await?;
```

### 2. Analyze Market (EV Calculation)
```rust
// Fetch on-chain data
let market = client.get_market(&market_pubkey).await?;

// Calculate implied probability from bets
let total_pool = market.total_yes_bets + market.total_no_bets;
let yes_price = market.total_yes_bets as f64 / total_pool as f64;

// Estimate true probability (our existing algorithm)
let true_prob = estimate_true_probability(&market);

// Calculate EV
let ev = (true_prob * (1.0 / yes_price)) - ((1.0 - true_prob) * yes_price);

// Generate signal if positive EV
if ev > 0.05 {
    let kelly_fraction = calculate_kelly(true_prob, yes_price);
    signals.push(PredictionSignal { ev, kelly_fraction, ... });
}
```

### 3. Execute Trade (On-Chain)
```rust
// Place bet based on signal
let amount_lamports = (kelly_fraction * 1_000_000_000.0) as u64; // SOL to lamports
let signature = client.place_bet(
    &market_pubkey,
    amount_lamports,
    true, // Betting Yes
).await?;

println!("Bet placed! Signature: {}", signature);
```

### 4. Monitor & Resolve
```rust
// After end time, resolve market
if market.end_time < current_time && !market.resolved {
    // Check oracle data or manual resolution
    let outcome = check_oracle_outcome(&market).await?;
    
    client.resolve_market(&market_pubkey, outcome).await?;
}

// Winners claim automatically
if market.resolved && bet.outcome == market.outcome && !bet.claimed {
    client.claim_winnings(&market_pubkey).await?;
}
```

## 💡 Pro Tips

### Development
- **Use devnet** for all testing (free, fast)
- **Anchor test** runs local validator automatically
- **Solana Explorer** for viewing transactions
- **Solscan** for detailed account inspection

### Integration
- **Start simple** - Read-only queries first
- **Then write** - Add transaction capabilities
- **Cache data** - Don't query chain every API call
- **WebSocket** - Subscribe to account changes for real-time

### Performance
- **Batch queries** - Fetch multiple accounts at once
- **Use indexes** - Helius, Triton for fast queries
- **Rate limits** - Public RPC has limits, use private
- **Retries** - Transactions can fail, implement retry logic

### Security
- **Never expose** private keys
- **Validate inputs** before sending transactions
- **Check balances** before executing trades
- **Monitor activity** for suspicious patterns

## 📚 Resources

### Documentation
- **Smart Contract Setup**: `./SMART_CONTRACT_SETUP.md` (detailed guide)
- **Integration Plan**: `./SOLANA_INTEGRATION_PLAN.md` (full roadmap)
- **Contract Docs**: `./smart-contract/README.md` (Anchor program)

### Live Examples
- **Devnet Transactions**: See smart-contract/README.md for links
- **Program Explorer**: [View on Solscan](https://solscan.io/account/3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg?cluster=devnet)

### Support
- **Anchor Discord**: [discord.gg/anchor](https://discord.gg/anchor)
- **Solana Discord**: [discord.gg/solana](https://discord.gg/solana)

## 🎯 Next Actions

**Immediate** (You can do now):
```bash
# 1. Build the contract
cd smart-contract
yarn install
anchor build

# 2. Run tests
anchor test

# 3. Deploy to devnet
solana config set --url devnet
solana-keygen new
solana airdrop 2
anchor deploy
```

**Next Steps** (Integration):
1. Create `backend/src/solana_prediction_client.rs`
2. Add anchor-client to Cargo.toml
3. Update API to fetch from chain
4. Test end-to-end flow
5. Deploy backend with on-chain connection

---

**Status**: ✅ Smart contract forked and ready  
**Next**: Build and deploy to devnet for testing  
**Timeline**: 1-2 hours for basic integration, 1-2 days for full features
