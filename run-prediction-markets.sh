#!/bin/bash

# Prediction Markets Trading System Startup Script

echo "🔮 =========================================="
echo "🔮 Prediction Markets Trading System"
echo "🔮 =========================================="
echo ""

# Check if backend binary exists
if [ ! -f "backend/target/release/prediction-markets" ]; then
    echo "📦 Building prediction-markets binary..."
    cd backend
    cargo build --release --bin prediction-markets
    cd ..
fi

# Start backend
echo "🚀 Starting backend server..."
cd backend
cargo run --release --bin prediction-markets &
BACKEND_PID=$!
cd ..

# Wait for backend to start
echo "⏳ Waiting for backend to initialize..."
sleep 3

# Check if backend is running
if ! curl -s http://localhost:8080/health > /dev/null; then
    echo "❌ Backend failed to start"
    kill $BACKEND_PID 2>/dev/null
    exit 1
fi

echo "✅ Backend running on http://localhost:8080"
echo ""
echo "📡 API Endpoints:"
echo "   GET  /health          - Health check"
echo "   GET  /markets         - List all markets"
echo "   GET  /stats           - Market statistics"
echo "   GET  /signals         - All trading signals"
echo "   POST /trade           - Execute trade"
echo ""

# Check if frontend dependencies are installed
if [ ! -d "frontend/node_modules" ]; then
    echo "📦 Installing frontend dependencies..."
    cd frontend
    npm install
    cd ..
fi

# Start frontend
echo "🎨 Starting frontend..."
cd frontend
npm run dev &
FRONTEND_PID=$!
cd ..

echo ""
echo "✅ Frontend starting on http://localhost:5173"
echo ""
echo "🔮 Prediction Markets Trading System is ready!"
echo ""
echo "Press Ctrl+C to stop all services"
echo ""

# Handle shutdown
trap "echo ''; echo '🛑 Shutting down...'; kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit" INT TERM

# Wait for processes
wait
