#!/bin/bash

# Solana Wallet Integration Setup Script
# This script installs all required dependencies for Solana wallet integration

set -e

echo "🔐 Setting up Solana Wallet Integration..."
echo ""

# Check if we're in the right directory
if [ ! -f "frontend/package.json" ]; then
    echo "❌ Error: Must be run from the repository root"
    exit 1
fi

cd frontend

echo "📦 Installing Solana Wallet Adapter dependencies..."
npm install \
    @solana/wallet-adapter-base@^0.9.23 \
    @solana/wallet-adapter-react@^0.15.35 \
    @solana/wallet-adapter-react-ui@^0.9.35 \
    @solana/wallet-adapter-wallets@^0.19.32 \
    @solana/web3.js@^1.95.0

echo ""
echo "✅ Dependencies installed successfully!"
echo ""
echo "📋 Installed packages:"
echo "   - @solana/wallet-adapter-base"
echo "   - @solana/wallet-adapter-react"
echo "   - @solana/wallet-adapter-react-ui"
echo "   - @solana/wallet-adapter-wallets"
echo "   - @solana/web3.js"
echo ""
echo "🎉 Wallet integration setup complete!"
echo ""
echo "📚 Next steps:"
echo "   1. Review WALLET_INTEGRATION_GUIDE.md for usage examples"
echo "   2. Run 'npm run dev' to start the development server"
echo "   3. Connect a Solana wallet (Phantom, Solflare, etc.)"
echo ""
echo "💡 Supported wallets:"
echo "   - Phantom (recommended)"
echo "   - Solflare"
echo "   - Backpack"
echo "   - Coinbase Wallet"
echo "   - Trust Wallet"
echo ""
