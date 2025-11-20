# 🔐 Solana Wallet Integration Guide

Complete guide for integrating Solana wallets into the prediction markets trader frontend.

## 📦 Official Solana Wallet Adapter

**Repository**: [anza-xyz/wallet-adapter](https://github.com/anza-xyz/wallet-adapter) (1,952⭐)

The **official** and most widely-used wallet adapter for Solana applications. Supports all major Solana wallets.

### Supported Wallets

- **Phantom** (Most popular - 3M+ users)
- **Solflare**
- **Backpack**
- **Trust Wallet**
- **Ledger**
- **Trezor**
- **Coin98**
- **Slope**
- **Glow**
- **Brave Wallet**
- And 40+ more wallets

## 🚀 Quick Start Integration

### Step 1: Install Dependencies

```bash
cd frontend

npm install @solana/wallet-adapter-react \
            @solana/wallet-adapter-react-ui \
            @solana/wallet-adapter-wallets \
            @solana/web3.js

# Optional: Install specific wallet adapters
npm install @solana/wallet-adapter-phantom \
            @solana/wallet-adapter-solflare \
            @solana/wallet-adapter-backpack
```

### Step 2: Setup Wallet Provider

Create `frontend/src/contexts/WalletContextProvider.tsx`:

```typescript
import { FC, ReactNode, useMemo } from 'react';
import {
  ConnectionProvider,
  WalletProvider,
} from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  BackpackWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { clusterApiUrl } from '@solana/web3.js';

// Import wallet adapter CSS
import '@solana/wallet-adapter-react-ui/styles.css';

interface WalletContextProviderProps {
  children: ReactNode;
}

export const WalletContextProvider: FC<WalletContextProviderProps> = ({ children }) => {
  // Choose network: 'devnet', 'testnet', or 'mainnet-beta'
  const network = 'mainnet-beta';
  
  // You can also use a custom RPC endpoint (Helius, QuickNode, etc.)
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);
  // const endpoint = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY";
  
  // Configure which wallets to support
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new BackpackWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          {children}
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
};
```

### Step 3: Wrap App with Provider

Update `frontend/src/main.tsx`:

```typescript
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App-PredictionOnly'
import { WalletContextProvider } from './contexts/WalletContextProvider'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <WalletContextProvider>
      <App />
    </WalletContextProvider>
  </React.StrictMode>,
)
```

### Step 4: Add Wallet Connect Button

Create `frontend/src/components/WalletButton.tsx`:

```typescript
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

export const WalletButton = () => {
  return (
    <WalletMultiButton />
  );
};
```

### Step 5: Use Wallet in Components

Update `frontend/src/App-PredictionOnly.tsx`:

```typescript
import { useState, useEffect } from 'react'
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { LAMPORTS_PER_SOL } from '@solana/web3.js'
import PredictionMarkets from './components/PredictionMarkets'
import { WalletButton } from './components/WalletButton'
import axios from 'axios'

function App() {
  const [isConnected, setIsConnected] = useState(false)
  const [balance, setBalance] = useState<number | null>(null)
  
  // Wallet hooks
  const { publicKey, connected } = useWallet()
  const { connection } = useConnection()

  // Check server connection
  useEffect(() => {
    const checkConnection = async () => {
      try {
        const response = await axios.get('http://localhost:8080/health', { timeout: 2000 })
        setIsConnected(response.data.success)
      } catch {
        setIsConnected(false)
      }
    }
    
    checkConnection()
    const interval = setInterval(checkConnection, 10000)
    return () => clearInterval(interval)
  }, [])

  // Get wallet balance
  useEffect(() => {
    if (!publicKey) {
      setBalance(null)
      return
    }

    const getBalance = async () => {
      try {
        const bal = await connection.getBalance(publicKey)
        setBalance(bal / LAMPORTS_PER_SOL)
      } catch (error) {
        console.error('Error fetching balance:', error)
      }
    }

    getBalance()
    const interval = setInterval(getBalance, 10000)
    return () => clearInterval(interval)
  }, [publicKey, connection])

  return (
    <div className="app">
      <header className="header">
        <div className="header-content">
          <div className="brand">
            <h1 className="glow">🔮 Prediction Markets Trader</h1>
            <p className="subtitle pulse">EV-Based Trading with Kelly Criterion</p>
          </div>
          
          <div className="header-stats">
            {/* Server Status */}
            <div className="stat-card mini">
              <div className={`status-dot ${isConnected ? 'connected' : 'disconnected'}`}></div>
              <div className="stat-info">
                <div className="stat-label">Server</div>
                <div className="stat-value">{isConnected ? 'Connected' : 'Disconnected'}</div>
              </div>
            </div>

            {/* Wallet Status */}
            {connected && publicKey && (
              <div className="stat-card mini">
                <div className="stat-info">
                  <div className="stat-label">Wallet Balance</div>
                  <div className="stat-value">
                    {balance !== null ? `${balance.toFixed(4)} SOL` : 'Loading...'}
                  </div>
                </div>
              </div>
            )}

            {/* Wallet Button */}
            <WalletButton />
          </div>
        </div>
      </header>

      <main className="main-content modern">
        <div className="content-wrapper fade-in">
          {connected ? (
            <PredictionMarkets />
          ) : (
            <div className="connect-prompt">
              <h2>Connect Your Wallet</h2>
              <p>Please connect a Solana wallet to trade prediction markets</p>
              <WalletButton />
            </div>
          )}
        </div>
      </main>

      <footer className="footer modern">
        <div className="footer-content">
          <div className="footer-section">
            <span className="footer-label">Strategy:</span>
            <span className="tech-badge glow">Expected Value Analysis</span>
            <span className="tech-badge glow">Kelly Criterion</span>
          </div>
          <div className="footer-section">
            <span className="footer-label">Markets:</span>
            <span className="tech-badge pulse">Solana On-Chain</span>
            <span className="tech-badge pulse">Monaco Protocol</span>
          </div>
          {connected && publicKey && (
            <div className="footer-section">
              <span className="footer-label">Address:</span>
              <span className="tech-badge">
                {publicKey.toBase58().slice(0, 4)}...{publicKey.toBase58().slice(-4)}
              </span>
            </div>
          )}
        </div>
      </footer>
    </div>
  )
}

export default App
```

## 🎨 Styling (Optional but Recommended)

Add to `frontend/src/styles/futuristic.css`:

```css
/* Wallet Button Styling */
.wallet-adapter-button {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border: 2px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  padding: 12px 24px;
  font-family: 'SF Mono', 'Monaco', 'Courier New', monospace;
  font-weight: 600;
  transition: all 0.3s ease;
  box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4);
}

.wallet-adapter-button:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(102, 126, 234, 0.6);
  border-color: rgba(255, 255, 255, 0.2);
}

.wallet-adapter-button-trigger {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.wallet-adapter-modal-wrapper {
  background: rgba(0, 0, 0, 0.8) !important;
  backdrop-filter: blur(10px);
}

.wallet-adapter-modal {
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  border: 2px solid rgba(102, 126, 234, 0.3);
  border-radius: 20px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
}

.wallet-adapter-modal-title {
  color: #ffffff;
  font-family: 'SF Mono', 'Monaco', 'Courier New', monospace;
}

.wallet-adapter-modal-list {
  padding: 20px;
}

.wallet-adapter-modal-list-item {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  margin-bottom: 10px;
  transition: all 0.3s ease;
}

.wallet-adapter-modal-list-item:hover {
  background: rgba(102, 126, 234, 0.1);
  border-color: rgba(102, 126, 234, 0.5);
  transform: translateX(5px);
}

/* Connect Prompt */
.connect-prompt {
  text-align: center;
  padding: 60px 20px;
  background: linear-gradient(135deg, rgba(26, 26, 46, 0.8) 0%, rgba(22, 33, 62, 0.8) 100%);
  border: 2px solid rgba(102, 126, 234, 0.3);
  border-radius: 20px;
  backdrop-filter: blur(10px);
}

.connect-prompt h2 {
  color: #ffffff;
  font-size: 32px;
  margin-bottom: 16px;
  text-shadow: 0 0 20px rgba(102, 126, 234, 0.5);
}

.connect-prompt p {
  color: rgba(255, 255, 255, 0.7);
  font-size: 18px;
  margin-bottom: 30px;
}
```

## 🔧 Using Wallet for Trading

Example: Send a transaction to Monaco Protocol

```typescript
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { Transaction, SystemProgram, PublicKey } from '@solana/web3.js'

const { publicKey, sendTransaction } = useWallet()
const { connection } = useConnection()

async function placeBet(marketPubkey: string, amount: number) {
  if (!publicKey) {
    alert('Please connect your wallet')
    return
  }

  try {
    // Build transaction (this is a simplified example)
    const transaction = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: publicKey,
        toPubkey: new PublicKey(marketPubkey),
        lamports: amount * 1e9, // Convert SOL to lamports
      })
    )

    // Get recent blockhash
    const { blockhash } = await connection.getLatestBlockhash()
    transaction.recentBlockhash = blockhash
    transaction.feePayer = publicKey

    // Send transaction
    const signature = await sendTransaction(transaction, connection)
    
    // Wait for confirmation
    await connection.confirmTransaction(signature, 'confirmed')
    
    console.log('Transaction successful:', signature)
    return signature
  } catch (error) {
    console.error('Transaction failed:', error)
    throw error
  }
}
```

## 🔐 Security Best Practices

### 1. Never Request Private Keys
```typescript
// ❌ NEVER DO THIS
const privateKey = wallet.getPrivateKey() // This doesn't exist and shouldn't

// ✅ ALWAYS DO THIS
const { publicKey, signTransaction, sendTransaction } = useWallet()
```

### 2. Always Verify Transactions
```typescript
// Show transaction details before signing
const showTransactionDetails = (transaction: Transaction) => {
  alert(`
    To: ${transaction.instructions[0].keys[1].pubkey.toBase58()}
    Amount: ${transaction.instructions[0].data.toString()} lamports
  `)
}

// Then let user sign
await sendTransaction(transaction, connection)
```

### 3. Handle Errors Gracefully
```typescript
try {
  await sendTransaction(transaction, connection)
} catch (error: any) {
  if (error.message.includes('User rejected')) {
    alert('Transaction cancelled by user')
  } else if (error.message.includes('Insufficient')) {
    alert('Insufficient SOL balance')
  } else {
    alert('Transaction failed: ' + error.message)
  }
}
```

## 🎯 Monaco Protocol Integration

For trading on Monaco Protocol with wallet:

```typescript
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { PublicKey } from '@solana/web3.js'
import { Program, AnchorProvider } from '@coral-xyz/anchor'

const { publicKey, signTransaction, signAllTransactions } = useWallet()
const { connection } = useConnection()

async function placeMonacoBet(marketPubkey: string, outcome: string, stake: number) {
  if (!publicKey || !signTransaction) {
    throw new Error('Wallet not connected')
  }

  // Create Anchor provider from wallet
  const provider = new AnchorProvider(
    connection,
    { publicKey, signTransaction, signAllTransactions },
    { commitment: 'confirmed' }
  )

  // Load Monaco Protocol program
  const MONACO_PROGRAM_ID = new PublicKey('monacoUXKtUi6vKsQwaLyxmXKSievfNWEcYXTgkbCih')
  const program = new Program(IDL, MONACO_PROGRAM_ID, provider)

  // Build and send transaction
  const tx = await program.methods
    .createOrder({
      market: new PublicKey(marketPubkey),
      outcome,
      stake,
    })
    .accounts({
      user: publicKey,
      // ... other accounts
    })
    .rpc()

  return tx
}
```

## 📱 Mobile Wallet Support

For mobile dApp support (Phantom Mobile, Solflare Mobile):

```typescript
import { useMemo } from 'react'
import { WalletAdapterNetwork } from '@solana/wallet-adapter-base'
import { clusterApiUrl } from '@solana/web3.js'

export const WalletContextProvider: FC<WalletContextProviderProps> = ({ children }) => {
  const network = WalletAdapterNetwork.Mainnet
  const endpoint = useMemo(() => clusterApiUrl(network), [network])
  
  // Auto-select wallet for mobile
  const wallets = useMemo(() => {
    // Detect mobile
    const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent)
    
    if (isMobile) {
      // Use WalletConnect for mobile
      return []
    } else {
      // Desktop wallets
      return [
        new PhantomWalletAdapter(),
        new SolflareWalletAdapter(),
        new BackpackWalletAdapter(),
      ]
    }
  }, [])

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          {children}
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  )
}
```

## 🧪 Testing

Test with devnet first:

```typescript
// Use devnet for testing
const network = 'devnet'
const endpoint = clusterApiUrl('devnet')

// Get devnet SOL from faucet
// https://solfaucet.com/
```

## 📚 Resources

### Official Documentation
- [Solana Wallet Adapter Docs](https://github.com/anza-xyz/wallet-adapter)
- [Solana Web3.js Docs](https://solana-labs.github.io/solana-web3.js/)
- [Phantom Wallet Developer Docs](https://docs.phantom.app/)

### Example Apps
- [Demo App](https://anza-xyz.github.io/wallet-adapter/example)
- [Solana Cookbook](https://solanacookbook.com/guides/get-started.html#how-to-use-wallet-adapter)

### Alternative Solutions
- **Unified Wallet Kit** (87⭐) - TeamRaccoons/Unified-Wallet-Kit
  - Swiss Army Knife wallet adapter
  - Better UX, more customization
  - Drop-in replacement for official adapter

## 🚀 Production Checklist

Before going live:

- [ ] Test on devnet with real wallet
- [ ] Test all wallet types (Phantom, Solflare, etc.)
- [ ] Test mobile wallets
- [ ] Implement proper error handling
- [ ] Add transaction retry logic
- [ ] Setup RPC failover (multiple endpoints)
- [ ] Monitor RPC rate limits
- [ ] Test with slow internet
- [ ] Test wallet disconnect/reconnect
- [ ] Add transaction history
- [ ] Implement spending limits

## 💡 Common Issues & Solutions

### Issue: "Wallet not connected"
**Solution**: Check `connected` state before sending transactions

```typescript
if (!connected || !publicKey) {
  alert('Please connect your wallet first')
  return
}
```

### Issue: "Transaction too large"
**Solution**: Use lookup tables or split into multiple transactions

### Issue: "RPC rate limited"
**Solution**: Use paid RPC provider (Helius, QuickNode)

```typescript
const endpoint = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"
```

### Issue: "Wallet adapter styles not loading"
**Solution**: Import CSS in main file

```typescript
import '@solana/wallet-adapter-react-ui/styles.css'
```

## 🎉 Summary

**Complete Wallet Integration Stack**:
- ✅ **@solana/wallet-adapter-react** - Core functionality
- ✅ **@solana/wallet-adapter-react-ui** - Pre-built UI components
- ✅ **@solana/wallet-adapter-wallets** - Wallet adapters
- ✅ **@solana/web3.js** - Solana blockchain interaction

**Supported**: Phantom, Solflare, Backpack, and 40+ wallets

**Ready for**: Monaco Protocol trading, custom programs, token transfers

---

**Status**: Ready for implementation  
**Difficulty**: Easy (2-3 hours)  
**Cost**: Free (all open source)
