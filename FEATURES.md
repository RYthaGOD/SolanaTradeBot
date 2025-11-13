# 🚀 AgentBurn Solana Trader - Enhanced Features

## New Features Added (Cherry-picked from agentburn repo)

### 🧠 1. DeepSeek AI Integration

**AI-Powered Trading Decisions**
- Uses DeepSeek V3 AI model for intelligent trading analysis
- Free tier: 5M tokens/month (more than enough for trading operations)
- Analyzes market conditions, technical indicators, and risk levels
- Provides confidence scores and reasoning for each decision

**Key Capabilities:**
- Real-time market analysis with SMA-10/SMA-20 crossovers
- Volume profile analysis
- Risk assessment (LOW, MEDIUM, HIGH)
- Dynamic position sizing based on confidence
- Stop-loss and take-profit recommendations

**API Endpoint:**
```bash
GET /ai/status
# Returns: DeepSeek configuration and status
```

**Configuration:**
```bash
# Get your free API key from: https://platform.deepseek.com/api_keys
DEEPSEEK_API_KEY=your_api_key_here
```

**Usage Example:**
The AI analyzes each trading opportunity considering:
- Current price trends and momentum
- Moving average signals (bullish/bearish crossovers)
- Volume confirmation
- Portfolio risk (never exceeds 10% per trade)
- Market conditions

---

### 🔄 2. Jupiter DEX Integration

**Real Solana Token Swaps**
- Integration with Jupiter Aggregator (best price routing)
- Access to all Solana DEX liquidity
- Automatic route optimization
- Low slippage execution

**API Endpoints:**
```bash
GET /jupiter/quote/{input_mint}/{output_mint}/{amount}
# Returns: Best swap quote with price impact

# Example: Get SOL to USDC quote
GET /jupiter/quote/So11111111111111111111111111111111111111112/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/1000000000
```

**Features:**
- Real-time price quotes
- Multiple DEX routing (Raydium, Orca, Serum, etc.)
- Price impact calculation
- Slippage protection (default: 0.5%)

---

### 🛡️ 3. Jito MEV Protection

**Bundle Atomic Transactions**
- Protects trades from MEV (Maximal Extractable Value) attacks
- Guarantees transaction ordering
- Private transaction submission
- Fast finality with priority tips

**Key Benefits:**
- Atomic execution (all transactions succeed or fail together)
- MEV protection via TEE (Trusted Execution Environment)
- Reduced slippage from front-running
- Better execution prices

**Configuration:**
```rust
// Jito BAM Service with regional endpoints
JitoBamService::new("mainnet", 10_000) // 0.00001 SOL tip
```

---

### 🔒 4. Enhanced Security

**Security Middleware:**
- Rate limiting (60 requests/minute default)
- CORS configuration with whitelist
- Input validation and sanitization
- Security headers (X-Content-Type-Options, etc.)

**Features:**
- IP-based rate limiting
- Wallet address validation
- Amount validation (positive, finite)
- Symbol sanitization
- Protection against common web attacks

**API Protection:**
```rust
// Rate limiter automatically protects all endpoints
RateLimiter::new(60, Duration::from_secs(60))
```

---

### 📡 5. WebSocket Real-Time Updates

**Live Market Data Streaming**
- Real-time price updates
- Trade execution notifications
- Portfolio value changes
- Bid/ask spreads

**WebSocket Endpoint:**
```
ws://localhost:8080/ws
```

**Message Types:**
```json
// Market Update
{
  "type": "MarketUpdate",
  "symbol": "SOL/USDC",
  "price": 105.50,
  "volume": 2500000,
  "timestamp": 1699876543,
  "change_24h": 2.5
}

// Trade Update
{
  "type": "TradeUpdate",
  "id": "trade_123",
  "symbol": "SOL/USDC",
  "action": "BUY",
  "price": 105.50,
  "size": 10.0,
  "timestamp": 1699876543
}

// Portfolio Update
{
  "type": "PortfolioUpdate",
  "total_value": 10500.0,
  "cash": 5000.0,
  "positions": [["SOL", 50.0], ["BTC", 0.1]]
}
```

**Client Example:**
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

---

## 🎯 Trading Strategy Enhancement

### AI-Powered Decision Flow

1. **Market Data Collection**
   - Price history (20+ periods for SMA calculation)
   - Volume analysis (10-period average)
   - Technical indicators (SMA-10, SMA-20)

2. **DeepSeek AI Analysis**
   - Processes market context
   - Evaluates trend strength
   - Assesses volume confirmation
   - Calculates risk/reward ratio

3. **Risk Management Validation**
   - Portfolio drawdown check
   - Position size limit (max 10%)
   - Confidence threshold (min 50%)
   - Kelly criterion application

4. **Execution via Jupiter**
   - Get best route quote
   - Check price impact
   - Execute swap with slippage protection
   - Verify on-chain confirmation

5. **Real-time Notification**
   - Broadcast via WebSocket
   - Update portfolio state
   - Log transaction details

---

## 📊 API Endpoints Summary

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/portfolio` | GET | Portfolio data with positions |
| `/performance` | GET | Trading metrics (Sharpe, win rate, etc.) |
| `/market-data` | GET | Current market prices |
| `/signals` | GET | Recent trading signals |
| `/ws` | WebSocket | Real-time updates stream |
| `/jupiter/quote/{in}/{out}/{amt}` | GET | Jupiter swap quote |
| `/ai/status` | GET | DeepSeek AI configuration |

---

## 🔧 Configuration

### Environment Variables

```bash
# Required for AI trading
DEEPSEEK_API_KEY=sk-xxxxxxxxxxxx

# Optional Solana configuration
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
WALLET_PRIVATE_KEY=base58_encoded_key

# Trading parameters
MAX_POSITION_SIZE_PERCENT=10
CONFIDENCE_THRESHOLD=0.5
```

### Running with AI Features

```bash
# 1. Set up environment
cp .env.example .env
# Edit .env and add your DEEPSEEK_API_KEY

# 2. Build backend
cd backend
cargo build --release

# 3. Start server
RUST_LOG=info ./target/release/agentburn-backend
```

---

## 🚨 Important Notes

### DeepSeek Free Tier Limits
- 5 million tokens per month
- ~50,000 trading analyses per month
- Resets monthly
- No credit card required

### Risk Warnings
- Always test on devnet first
- Never risk more than you can afford to lose
- AI decisions are not financial advice
- Market conditions can change rapidly

### Security Best Practices
- Never commit API keys to git
- Use environment variables for secrets
- Enable rate limiting in production
- Validate all user inputs
- Use HTTPS in production

---

## 📈 Performance Improvements

### Before (Simulation Only)
- ❌ Simulated market data only
- ❌ Simple moving average strategy
- ❌ No real DEX integration
- ❌ Polling-based updates

### After (Cherry-picked Features)
- ✅ AI-powered decision making
- ✅ Real Jupiter DEX integration
- ✅ MEV protection via Jito
- ✅ Real-time WebSocket updates
- ✅ Enhanced security middleware
- ✅ Production-ready architecture

---

## 🎓 Learning Resources

- **DeepSeek API**: https://platform.deepseek.com/docs
- **Jupiter Docs**: https://station.jup.ag/docs
- **Jito MEV**: https://docs.jito.wtf/
- **Solana Web3**: https://docs.solana.com/

---

## 🤝 Contributing

Contributions welcome! See main README for guidelines.

---

**Built with features from agentburn repository** 🔥
