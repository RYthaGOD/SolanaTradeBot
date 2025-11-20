# Smart Contract Setup and Integration Guide

## 🎯 Overview

This guide provides step-by-step instructions for deploying and integrating the Solana prediction market smart contract with our backend trading system.

## 📦 Smart Contract Source

**Repository**: roswelly/solana-prediction-market-smart-contract  
**Location**: `./smart-contract/` directory  
**Framework**: Anchor v0.31+  
**Program ID**: `3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg`

## 🛠️ Prerequisites

Before deploying the smart contract, ensure you have the following installed:

### 1. Rust and Cargo
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Solana CLI Tools (v1.18+)
```bash
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Verify installation
solana --version
```

### 3. Anchor Framework (v0.31+)
```bash
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest

# Verify installation
anchor --version
```

### 4. Node.js and Yarn
```bash
# Install Node.js v18+ (using nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Install Yarn
npm install -g yarn

# Verify installation
node --version
yarn --version
```

## 🔧 Configuration

### 1. Setup Solana CLI

#### For Devnet (Recommended for Testing)
```bash
# Configure for devnet
solana config set --url devnet

# Create a new wallet (or use existing)
solana-keygen new --outfile ~/.config/solana/id.json

# Get devnet airdrop (for testing)
solana airdrop 2

# Check balance
solana balance
```

#### For Mainnet (Production)
```bash
# Configure for mainnet
solana config set --url mainnet-beta

# Use your production wallet
solana-keygen new --outfile ~/.config/solana/mainnet-wallet.json
solana config set --keypair ~/.config/solana/mainnet-wallet.json
```

### 2. Install Smart Contract Dependencies
```bash
cd smart-contract
yarn install
```

## 🏗️ Building the Smart Contract

### Build the Program
```bash
cd smart-contract

# Build the program
anchor build

# This creates:
# - target/deploy/prediction_market.so (program binary)
# - target/idl/prediction_market.json (IDL for client integration)
# - target/types/prediction_market.ts (TypeScript types)
```

### Verify Build
```bash
# Check program hash
solana program show target/deploy/prediction_market.so

# Check program size
ls -lh target/deploy/prediction_market.so
```

## 🚀 Deployment

### Deploy to Devnet

```bash
cd smart-contract

# Deploy the program
anchor deploy --provider.cluster devnet

# Expected output:
# Program Id: 3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg
```

### Verify Deployment
```bash
# Check program account
solana program show 3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg --url devnet

# View on Solana Explorer
# https://explorer.solana.com/address/3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg?cluster=devnet
```

### Deploy to Mainnet (Production)

⚠️ **Important**: Mainnet deployment requires:
- Sufficient SOL for deployment (~5-10 SOL)
- Security audit completion
- Thorough testing on devnet

```bash
cd smart-contract

# Switch to mainnet
solana config set --url mainnet-beta

# Deploy (costs SOL)
anchor deploy --provider.cluster mainnet-beta

# Upgrade program (if already deployed)
solana program deploy target/deploy/prediction_market.so --program-id 3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg
```

## 🧪 Testing

### Run Local Tests
```bash
cd smart-contract

# Start local validator (in separate terminal)
solana-test-validator

# Run tests
anchor test
```

### Run Devnet Tests
```bash
cd smart-contract

# Run devnet integration tests
yarn test:devnet
# OR
ts-node test-devnet.ts
```

### Test Coverage

The test suite includes:
- ✅ Market initialization
- ✅ Placing bets (Yes/No)
- ✅ Market resolution
- ✅ Claiming winnings
- ✅ Error conditions (invalid operations)

## 📊 Smart Contract Architecture

### Account Structures

#### Market Account (~305 bytes)
```rust
pub struct Market {
    pub creator: Pubkey,              // 32 bytes
    pub resolution_authority: Pubkey, // 32 bytes
    pub question: String,             // Variable (max 200 chars)
    pub end_time: i64,               // 8 bytes
    pub resolved: bool,              // 1 byte
    pub outcome: Option<bool>,       // 2 bytes
    pub total_yes_bets: u64,         // 8 bytes
    pub total_no_bets: u64,          // 8 bytes
    pub fee_percentage: u16,         // 2 bytes (100 = 1%)
    pub bump: u8,                    // 1 byte
}
```

#### Bet Account (~83 bytes)
```rust
pub struct Bet {
    pub bettor: Pubkey,    // 32 bytes
    pub market: Pubkey,    // 32 bytes
    pub amount: u64,       // 8 bytes
    pub outcome: bool,     // 1 byte
    pub claimed: bool,     // 1 byte
    pub bump: u8,          // 1 byte
}
```

### PDA Derivation

```rust
// Market PDA
let (market_pda, bump) = Pubkey::find_program_address(
    &[b"market", creator.key().as_ref(), question_hash.as_ref()],
    &program_id
);

// Bet PDA
let (bet_pda, bump) = Pubkey::find_program_address(
    &[b"bet", market.key().as_ref(), bettor.key().as_ref()],
    &program_id
);
```

### Instructions

1. **initialize_market** - Create new prediction market
   - Params: question, end_time, question_hash
   - Accounts: creator, market PDA
   - Fee: Rent for market account (~0.003 SOL)

2. **place_bet** - Bet on market outcome
   - Params: amount, outcome (true=Yes, false=No)
   - Accounts: bettor, market PDA, bet PDA
   - Transfers: SOL from bettor to market PDA
   - Fee: Rent for bet account (~0.002 SOL)

3. **resolve_market** - Set market outcome
   - Params: outcome (true=Yes, false=No)
   - Accounts: resolution_authority, market PDA
   - Authorization: Only resolution_authority can call

4. **claim_winnings** - Claim proportional payout
   - Accounts: bettor, market PDA, bet PDA
   - Transfers: SOL from market PDA to bettor
   - Formula: (user_bet / winning_pool) × (total_pool × (1 - fee%))

## 🔗 Integration with Backend

### Step 1: Generate IDL Types

The IDL (Interface Description Language) file describes the program interface:

```bash
cd smart-contract

# IDL is generated during build at:
# target/idl/prediction_market.json

# Copy IDL to backend
cp target/idl/prediction_market.json ../backend/prediction_market_idl.json
```

### Step 2: Create Rust Client Module

Create `backend/src/solana_prediction_client.rs`:

```rust
use anchor_client::{Client, Cluster, Program};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::rc::Rc;

pub struct SolanaPredictionClient {
    program: Program,
    payer: Rc<Keypair>,
}

impl SolanaPredictionClient {
    pub fn new(cluster: Cluster, payer: Keypair) -> Self {
        let payer = Rc::new(payer);
        let client = Client::new_with_options(
            cluster,
            payer.clone(),
            CommitmentConfig::confirmed(),
        );
        
        let program_id = "3LHuBziG2Tp1UrxgoTAZDDbvDK46quk6T99kHkgt8UQg"
            .parse::<Pubkey>()
            .unwrap();
            
        let program = client.program(program_id);
        
        Self { program, payer }
    }
    
    // Implement methods for each instruction...
}
```

### Step 3: Update Backend Dependencies

Add to `backend/Cargo.toml`:

```toml
[dependencies]
anchor-client = "0.31"
anchor-lang = "0.31"
```

### Step 4: Update API Endpoints

Modify `backend/src/api_prediction_only.rs` to use on-chain data:

```rust
// Replace simulated markets with on-chain queries
let markets_route = {
    let client = prediction_client.clone();
    
    warp::path!("markets")
        .and(warp::get())
        .and_then(move || {
            let client = client.clone();
            async move {
                // Fetch markets from Solana blockchain
                let markets = client.get_all_markets().await?;
                Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                    markets,
                    "On-chain markets retrieved"
                )))
            }
        })
};
```

## 📈 Cost Estimates

### Devnet (Free)
- ✅ Deployment: Free
- ✅ Transactions: Free
- ✅ Airdrop available for testing

### Mainnet
- **Deployment**: ~2-5 SOL (one-time)
- **Create Market**: ~0.003 SOL (rent)
- **Place Bet**: ~0.002 SOL (rent) + 0.000005 SOL (transaction fee)
- **Resolve Market**: ~0.000005 SOL (transaction fee)
- **Claim Winnings**: ~0.000005 SOL (transaction fee)

## 🔐 Security Considerations

### Smart Contract Security
✅ **Implemented**:
- Time validation (no betting after end time)
- Authorization checks (only authority can resolve)
- Math overflow protection (checked arithmetic)
- Reentrancy protection (native transfers)
- Double-claim prevention (claimed flag)

⚠️ **Recommendations**:
- Complete security audit before mainnet
- Bug bounty program
- Gradual rollout with limits
- Multi-sig for program authority

### Backend Security
- Store wallet private keys securely (environment variables, Vault)
- Use read-only RPC for queries
- Implement rate limiting
- Transaction retry with exponential backoff
- Monitor for suspicious activity

## 🐛 Troubleshooting

### Build Errors

**Error**: `anchor: command not found`
```bash
# Reinstall Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest
```

**Error**: `Rust version too old`
```bash
# Update Rust
rustup update stable
```

### Deployment Errors

**Error**: `Insufficient funds`
```bash
# Check balance
solana balance

# Airdrop (devnet only)
solana airdrop 2
```

**Error**: `Program account not found`
```bash
# Rebuild and redeploy
anchor clean
anchor build
anchor deploy
```

### Transaction Errors

**Error**: `Custom program error: 0x1770` (6000)
- This is `InvalidEndTime` - End time must be in future

**Error**: `Custom program error: 0x1771` (6001)
- This is `BettingPeriodEnded` - Cannot bet after end time

**Error**: `Custom program error: 0x1772` (6002)
- This is `MarketAlreadyResolved` - Market is already resolved

## 📚 Additional Resources

### Official Documentation
- [Anchor Framework](https://www.anchor-lang.com/)
- [Solana Documentation](https://docs.solana.com/)
- [Solana Cookbook](https://solanacookbook.com/)
- [Anchor Client (Rust)](https://docs.rs/anchor-client/)

### Example Transactions (Devnet)
- Initialize Market: [View on Solscan](https://solscan.io/tx/JFNNLiHVZTJJbCXZvdpUADnvzqKRXRpdnd6uKktn4ArNdo9iMwyxKbRBFe5LifGxY19ahLPmskLqTPCyZTAnGAE?cluster=devnet)
- Place Bet (Yes): [View on Solscan](https://solscan.io/tx/58RUoGYvdUP7mWzwbEQoM4ghu14LqpXVaGiesB2RLJxhdTxGsaxNaoaVRqYgPGY3CvScpeyUhmWWMYJLvuR6uY8Z?cluster=devnet)
- Place Bet (No): [View on Solscan](https://solscan.io/tx/4kmgr28sduuFnYVE19TRb2U9WFsDzNwuduh9QBDfXF2PxE3Yi3J2daPR34FthyukcYQXckhfBkn83r255EaFxuZX?cluster=devnet)

### Tools
- [Solana Explorer](https://explorer.solana.com/)
- [Solscan](https://solscan.io/)
- [Anchor Playground](https://beta.solpg.io/)

## 🎯 Next Steps

1. ✅ **Complete** - Fork and clone smart contract
2. 🔄 **Current** - Build and test locally
3. ⏳ **Next** - Deploy to devnet
4. ⏳ **Next** - Create Rust client module
5. ⏳ **Next** - Integrate with backend API
6. ⏳ **Next** - Test end-to-end flow
7. ⏳ **Future** - Security audit
8. ⏳ **Future** - Mainnet deployment

## 📞 Support

For issues with:
- **Smart Contract**: See [roswelly's repo](https://github.com/roswelly/solana-prediction-market-smart-contract)
- **Anchor Framework**: [Anchor Discord](https://discord.gg/anchor)
- **Solana**: [Solana Discord](https://discord.gg/solana)

---

**Status**: Smart contract forked and ready for deployment ✅  
**Next Action**: Build and deploy to devnet for testing
