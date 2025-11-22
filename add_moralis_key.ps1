# PowerShell script to add Moralis API key to .env file

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║      🔐 Moralis API Key Setup                           ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "This script will add your Moralis API key to the .env file.`n" -ForegroundColor Yellow
Write-Host "Get your API key from: https://admin.moralis.com/`n" -ForegroundColor Yellow

$apiKey = Read-Host "Enter your Moralis API key"

# Validate format (Moralis API keys don't have a specific prefix)
if ([string]::IsNullOrWhiteSpace($apiKey)) {
    Write-Host "`n❌ Error: API key cannot be empty" -ForegroundColor Red
    exit 1
}

if ($apiKey.Length -lt 10) {
    Write-Host "`n❌ Error: API key too short" -ForegroundColor Red
    Write-Host "   API keys should be at least 10 characters" -ForegroundColor Red
    exit 1
}

Write-Host "`n✅ API key format validated!" -ForegroundColor Green

# Read or create .env file
$envPath = ".env"
$envContent = ""

if (Test-Path $envPath) {
    $envContent = Get-Content $envPath -Raw
    if ($null -eq $envContent) {
        $envContent = ""
    }
} else {
    Write-Host "`n📄 Creating new .env file..." -ForegroundColor Yellow
    $envContent = ""
}

# Check if MORALIS_API_KEY already exists
if ($envContent -match "MORALIS_API_KEY=") {
    $envContent = $envContent -replace "MORALIS_API_KEY=.*", "MORALIS_API_KEY=$apiKey"
    Write-Host "📝 Updated existing MORALIS_API_KEY in .env file" -ForegroundColor Green
} else {
    if ($envContent -and -not $envContent.EndsWith("`n")) {
        $envContent += "`n"
    }
    $envContent += "`n# Moralis API Configuration (pump.fun token prices)`n"
    $envContent += "MORALIS_API_KEY=$apiKey`n"
    Write-Host "📝 Added MORALIS_API_KEY to .env file" -ForegroundColor Green
}

# Write back to .env
Set-Content -Path $envPath -Value $envContent -NoNewline

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           ✅ Setup Complete!                            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "Your Moralis API key has been stored in .env file.`n" -ForegroundColor Green
Write-Host "🚀 You can now fetch real-time pump.fun token prices!`n" -ForegroundColor Yellow








