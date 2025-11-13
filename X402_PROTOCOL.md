# X402 Signal Trading Protocol

## Overview

X402 is a decentralized protocol for trading signals as assets. It enables autonomous agents to generate, buy, sell, and execute trading signals in a marketplace environment.

## Protocol Specification

### Message Format

```rust
pub struct X402Message {
    pub protocol_version: String,      // "1.0"
    pub message_type: X402MessageType,  // Type of message
    pub sender_id: String,              // Agent/provider ID
    pub receiver_id: Option<String>,    // Optional target recipient
    pub timestamp: i64,                 // Unix timestamp
    pub payload: X402Payload,           // Message payload
    pub signature: Option<String>,      // Optional cryptographic signature
}
```

### Message Types

#### 1. SignalOffer
Broadcast a trading signal to the marketplace.

```json
{
  "protocol_version": "1.0",
  "message_type": "SignalOffer",
  "sender_id": "provider1",
  "receiver_id": null,
  "timestamp": 1699876543,
  "payload": {
    "Signal": {
      "id": "abc123",
      "provider": "provider1",
      "symbol": "SOL/USD",
      "action": "Buy",
      "entry_price": 105.50,
      "target_price": 115.00,
      "stop_loss": 100.00,
      "confidence": 0.85,
      "timeframe": "4h",
      "data_sources": ["Switchboard Oracle", "DEX Screener"],
      "analysis": "Strong momentum detected",
      "timestamp": 1699876543,
      "expiry": 1699890943,
      "price": 15.0,
      "status": "Active"
    }
  },
  "signature": null
}
```

#### 2. SignalRequest
Request signals matching specific criteria.

```json
{
  "protocol_version": "1.0",
  "message_type": "SignalRequest",
  "sender_id": "trader1",
  "receiver_id": null,
  "timestamp": 1699876543,
  "payload": {
    "Request": {
      "symbols": ["SOL/USD", "ETH/USD"],
      "max_price": 20.0
    }
  },
  "signature": null
}
```

#### 3. SignalPurchase
Purchase a signal from the marketplace.

```json
{
  "protocol_version": "1.0",
  "message_type": "SignalPurchase",
  "sender_id": "trader1",
  "receiver_id": "marketplace",
  "timestamp": 1699876543,
  "payload": {
    "Purchase": {
      "signal_id": "abc123",
      "payment": 15.0
    }
  },
  "signature": null
}
```

#### 4. SignalConfirmation
Confirm signal purchase.

```json
{
  "protocol_version": "1.0",
  "message_type": "SignalConfirmation",
  "sender_id": "marketplace",
  "receiver_id": "trader1",
  "timestamp": 1699876543,
  "payload": {
    "Confirmation": {
      "signal_id": "abc123",
      "status": "purchased"
    }
  },
  "signature": null
}
```

## Signal Structure

```rust
pub struct TradingSignalData {
    pub id: String,                     // Unique signal ID
    pub provider: String,               // Provider ID
    pub symbol: String,                 // Trading pair (e.g., "SOL/USD")
    pub action: SignalAction,           // Buy/Sell/Hold
    pub entry_price: f64,               // Recommended entry price
    pub target_price: f64,              // Take profit target
    pub stop_loss: f64,                 // Stop loss level
    pub confidence: f64,                // 0.0 to 1.0
    pub timeframe: String,              // e.g., "1h", "4h", "1d"
    pub data_sources: Vec<String>,      // Data sources used
    pub analysis: String,               // Analysis description
    pub timestamp: i64,                 // Signal creation time
    pub expiry: i64,                    // Signal expiry time
    pub price: f64,                     // Price to purchase signal
    pub status: SignalStatus,           // Active/Filled/Expired/Cancelled
}
```

## Provider System

### Registration

Providers must register before publishing signals:

```bash
curl -X POST http://localhost:8080/signals/marketplace/provider/register \
  -H "Content-Type: application/json" \
  -d '{"id": "provider1", "name": "My Trading Signals"}'
```

### Reputation System

Providers are tracked with:
- **Success Rate**: Percentage of profitable signals
- **Total Signals**: Number of signals published
- **Successful Signals**: Number of profitable signals
- **Earnings**: Total revenue from signal sales
- **Reputation Score**: 0-100 dynamic score based on performance

## API Endpoints

### Marketplace Operations

```bash
# Get marketplace statistics
GET /signals/marketplace/stats

# Get all active signals
GET /signals/marketplace/active

# Get signals for specific symbol
GET /signals/marketplace/symbol/{symbol}

# Generate signals from all data sources
POST /signals/marketplace/generate/{provider_id}

# Register as provider
POST /signals/marketplace/provider/register
Body: {"id": "provider1", "name": "Provider Name"}

# Get provider statistics
GET /signals/marketplace/provider/{id}

# Purchase a signal
POST /signals/marketplace/purchase
Body: {"user_id": "trader1", "signal_id": "abc123", "payment": 15.0}
```

## Data Sources Integration

Signals are generated from three primary sources:

### 1. Switchboard Oracle
- **Real-time price feeds** for SOL, BTC, ETH, USDC
- **Confidence intervals** for price accuracy
- **On-chain data** via Solana blockchain
- Generates signals on significant price movements (>2%)

### 2. DEX Screener
- **Token discovery** and trending analysis
- **Liquidity monitoring** (minimum $5,000)
- **Volume analysis** for momentum detection
- **Opportunity scoring** based on multiple factors:
  - 5-minute momentum
  - 1-hour trends
  - Volume spikes
  - Liquidity depth

### 3. PumpFun
- **Meme coin launches** monitoring
- **Sentiment analysis** based on:
  - Reply count (engagement)
  - Market cap growth
  - Time since launch
  - Live status
- **Risk assessment** (Low/Medium/High/Extreme)
- **Hype level tracking** (Low/Medium/High/Extreme)

## Usage Examples

### Example 1: Generate and View Signals

```bash
# 1. Register as provider
curl -X POST http://localhost:8080/signals/marketplace/provider/register \
  -H "Content-Type: application/json" \
  -d '{"id":"algo_trader_1","name":"Algorithmic Trader"}'

# 2. Generate signals
curl -X POST http://localhost:8080/signals/marketplace/generate/algo_trader_1

# 3. View active signals
curl http://localhost:8080/signals/marketplace/active | jq .
```

### Example 2: Purchase and Execute Signal

```bash
# 1. View available signals
curl http://localhost:8080/signals/marketplace/active | jq '.data[0]'

# 2. Purchase a signal
curl -X POST http://localhost:8080/signals/marketplace/purchase \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "trader1",
    "signal_id": "abc123",
    "payment": 15.0
  }'

# 3. Execute trade based on signal (via trading engine)
```

### Example 3: Monitor Provider Performance

```bash
# Get provider statistics
curl http://localhost:8080/signals/marketplace/provider/algo_trader_1 | jq .

# Response includes:
# - success_rate: Percentage of profitable signals
# - total_signals: Total signals published
# - earnings: Total revenue
# - reputation_score: Dynamic score (0-100)
```

## Autonomous Agent Integration

The autonomous agent continuously:
1. **Monitors** all three data sources (Oracle, DEX, PumpFun)
2. **Analyzes** market conditions and generates composite signals
3. **Validates** signals against risk management rules
4. **Executes** trades based on signal confidence
5. **Publishes** successful strategies as tradeable signals

### Agent Configuration

```rust
pub struct AutonomousAgent {
    min_confidence: f64,        // Minimum confidence threshold (0.6)
    check_interval_secs: u64,   // Market check interval (60 seconds)
    // ... data source clients
}
```

## Security Considerations

1. **Rate Limiting**: API endpoints are protected
2. **Validation**: All inputs are validated and sanitized
3. **Expiry**: Signals automatically expire after timeframe
4. **Payment Verification**: Purchases require sufficient payment
5. **Provider Reputation**: Track record visible to all users

## Future Enhancements

- [ ] On-chain signal marketplace using Solana smart contracts
- [ ] Cryptographic signatures for all X402 messages
- [ ] Decentralized provider reputation via blockchain
- [ ] Signal performance tracking and automatic refunds
- [ ] Cross-chain signal support (Ethereum, BSC, etc.)
- [ ] Advanced filtering and recommendation engine
- [ ] Social features (follow providers, leaderboards)
- [ ] API rate limiting per provider tier

## Protocol Versioning

Current version: **1.0**

The protocol version is included in all X402 messages to ensure compatibility as the protocol evolves.

## License

X402 Protocol - Open for implementation and extension.
