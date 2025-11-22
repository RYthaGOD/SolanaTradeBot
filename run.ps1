# PowerShell script to run SolanaTradeBot on Windows

# Try to set execution policy (ignore errors if it fails)
try {
    Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope Process -Force -ErrorAction Stop | Out-Null
} catch {
    # Execution policy may not be needed or may fail - continue anyway
    Write-Host "ℹ️  Execution policy: Using default (this is usually fine)" -ForegroundColor Gray
}

Write-Host "🚀 Starting SolanaTradeBot..." -ForegroundColor Green
Write-Host ""

# Check if backend is built
if (-not (Test-Path "backend\target\debug\agentburn-backend.exe") -and -not (Test-Path "backend\target\release\agentburn-backend.exe")) {
    Write-Host "📋 Building Rust backend..." -ForegroundColor Yellow
    Set-Location backend
    cargo build --bin agentburn-backend
    Set-Location ..
}

Write-Host ""
Write-Host "✅ Build complete!" -ForegroundColor Green
Write-Host ""

# Start backend in background
Write-Host "🌐 Starting backend server on port 8080..." -ForegroundColor Cyan
Set-Location backend
Start-Process powershell -ArgumentList "-NoExit", "-Command", "`$env:RUST_LOG='info'; cargo run --bin agentburn-backend"
Set-Location ..

Start-Sleep -Seconds 3

# Start frontend in background
Write-Host "⚛️ Starting frontend development server on port 5000..." -ForegroundColor Cyan
Set-Location frontend
Start-Process powershell -ArgumentList "-NoExit", "-Command", "npm run dev"
Set-Location ..

Write-Host ""
Write-Host "🎉 SolanaTradeBot is running!" -ForegroundColor Green
Write-Host ""
Write-Host "📊 Frontend: http://localhost:5000" -ForegroundColor Yellow
Write-Host "🔧 Backend API (v1): http://localhost:8080" -ForegroundColor Yellow
Write-Host "🤖 Backend API (v2): http://localhost:8081" -ForegroundColor Yellow
Write-Host ""
Write-Host "Press Ctrl+C to stop (close the PowerShell windows)" -ForegroundColor Gray

