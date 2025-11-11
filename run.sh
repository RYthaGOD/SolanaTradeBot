#!/bin/bash

set -e

echo "🚀 Starting AgentBurn Solana Trader..."
echo ""

echo "📋 Installing frontend dependencies..."
cd frontend
if [ ! -d "node_modules" ]; then
    npm install
fi
cd ..

echo "🦀 Building Rust backend..."
cd backend
cargo build --release 2>&1 | grep -v "^   " | grep -v "Compiling" || true
cd ..

echo ""
echo "✅ Build complete!"
echo ""

echo "🌐 Starting backend server on port 8080..."
cd backend
RUST_LOG=info ./target/release/agentburn-backend &
BACKEND_PID=$!
cd ..

sleep 3

echo "⚛️ Starting frontend development server on port 5000..."
cd frontend
npm run dev &
FRONTEND_PID=$!
cd ..

cleanup() {
    echo ""
    echo "🛑 Shutting down servers..."
    kill $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
    exit 0
}

trap cleanup INT TERM

echo ""
echo "🎉 AgentBurn Solana Trader is running!"
echo ""
echo "📊 Frontend: http://0.0.0.0:5000"
echo "🔧 Backend API: http://localhost:8080"
echo ""
echo "Press Ctrl+C to stop servers"
echo ""

wait
