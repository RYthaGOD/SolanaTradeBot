# 🚀 Wallet Integration Quick Start

## Installation (2 minutes)

### Option 1: Using Setup Script (Recommended)
```bash
./setup-wallet-integration.sh
```

### Option 2: Manual Installation
```bash
cd frontend
npm install @solana/wallet-adapter-base \
            @solana/wallet-adapter-react \
            @solana/wallet-adapter-react-ui \
            @solana/wallet-adapter-wallets \
            @solana/web3.js
```

## Running the App

```bash
# Terminal 1: Start backend
cd backend
cargo run --bin prediction-markets

# Terminal 2: Start frontend
cd frontend
npm run dev
```

Visit: http://localhost:5000

## Using the Wallet

1. **Install a Solana Wallet** (if you don't have one):
   - [Phantom](https://phantom.app/) - Most popular (Chrome, Firefox, Edge, Brave, Mobile)
   - [Solflare](https://solflare.com/) - Feature-rich
   - [Backpack](https://backpack.app/) - Modern UI

2. **Connect Wallet**:
   - Click "Select Wallet" button in the header
   - Choose your wallet from the modal
   - Approve the connection in your wallet extension

3. **View Balance**:
   - Once connected, your SOL balance will display in the header
   - Balance updates every 10 seconds

4. **Trade Prediction Markets**:
   - Connected wallet is required to view markets
   - Use your wallet to sign transactions
   - All trades happen on-chain with Monaco Protocol

## Features Included

✅ **5 Major Wallet Support**:
- Phantom
- Solflare
- Backpack
- Coinbase Wallet
- Trust Wallet

✅ **Real-time Balance Display**
✅ **Auto-reconnect on Page Refresh**
✅ **Mobile Wallet Support**
✅ **Beautiful Custom UI**
✅ **Network Selection (Mainnet/Devnet)**

## Network Configuration

Edit `frontend/src/contexts/WalletContextProvider.tsx`:

```typescript
// For testing on devnet:
const network = 'devnet';

// For production on mainnet:
const network = 'mainnet-beta';

// For custom RPC (recommended for production):
const endpoint = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY";
```

## Using Wallet in Your Code

```typescript
import { useWallet, useConnection } from '@solana/wallet-adapter-react';

function MyComponent() {
  const { publicKey, connected, sendTransaction } = useWallet();
  const { connection } = useConnection();

  const handleTrade = async () => {
    if (!connected || !publicKey) {
      alert('Please connect wallet');
      return;
    }

    // Your trading logic here
    const transaction = /* build transaction */;
    const signature = await sendTransaction(transaction, connection);
    console.log('Transaction:', signature);
  };

  return (
    <div>
      {connected ? (
        <p>Connected: {publicKey.toBase58()}</p>
      ) : (
        <p>Not connected</p>
      )}
    </div>
  );
}
```

## Troubleshooting

### Issue: "Wallet adapter styles not loading"
**Solution**: Make sure CSS is imported in `WalletContextProvider.tsx`:
```typescript
import '@solana/wallet-adapter-react-ui/styles.css';
```

### Issue: "Connection to RPC failed"
**Solution**: 
1. Check internet connection
2. Try different RPC endpoint (Helius, QuickNode)
3. Switch to devnet for testing

### Issue: "Wallet not detected"
**Solution**:
1. Install wallet extension
2. Refresh page
3. Check browser permissions

### Issue: "Transaction failed"
**Solution**:
1. Check SOL balance (need some for gas)
2. Verify network (mainnet vs devnet)
3. Check transaction parameters

## Testing on Devnet

1. Switch to devnet in `WalletContextProvider.tsx`
2. Get free devnet SOL: https://solfaucet.com/
3. Test trades without risk

## Production Checklist

Before going live:

- [ ] Switch to mainnet-beta network
- [ ] Setup premium RPC (Helius/QuickNode)
- [ ] Test with real wallet
- [ ] Verify all wallet types work
- [ ] Test mobile wallets
- [ ] Add error handling
- [ ] Monitor RPC rate limits
- [ ] Setup transaction retry logic

## Security Notes

🔐 **The wallet adapter NEVER accesses private keys**

✅ Safe practices:
- Wallets sign transactions locally
- Private keys never leave the wallet
- Users approve every transaction
- Transactions are transparent

❌ Never do:
- Ask for private keys
- Send SOL to unknown addresses
- Auto-approve transactions

## Resources

- [Full Integration Guide](./WALLET_INTEGRATION_GUIDE.md)
- [Solana Wallet Adapter Docs](https://github.com/anza-xyz/wallet-adapter)
- [Monaco Protocol Docs](https://docs.monacoprotocol.xyz/)
- [Solana Web3.js Docs](https://solana-labs.github.io/solana-web3.js/)

## Support

For issues:
1. Check [Wallet Integration Guide](./WALLET_INTEGRATION_GUIDE.md)
2. Review console errors
3. Test with devnet
4. Check wallet extension is up to date

---

**Ready to trade! 🚀**

Your prediction markets trader now has full Solana wallet integration with 5 major wallets supported.
