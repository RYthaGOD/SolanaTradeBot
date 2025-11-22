# PowerShell script to add Mobula API key to .env file

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║      🔐 Mobula API Key Setup                            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "This script will add your Mobula API key to the .env file.`n" -ForegroundColor Yellow
Write-Host "Get your API key from: https://admin.mobula.io`n" -ForegroundColor Yellow

$apiKey = Read-Host "Enter your Mobula API key"

# Validate format (Mobula API keys don't have a specific prefix)
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

# Check if MOBULA_API_KEY already exists
if ($envContent -match "MOBULA_API_KEY=") {
    $envContent = $envContent -replace "MOBULA_API_KEY=.*", "MOBULA_API_KEY=$apiKey"
    Write-Host "📝 Updated existing MOBULA_API_KEY in .env file" -ForegroundColor Green
} else {
    if ($envContent -and -not $envContent.EndsWith("`n")) {
        $envContent += "`n"
    }
    $envContent += "`n# Mobula API Configuration (GMGN-compatible)`n"
    $envContent += "MOBULA_API_KEY=$apiKey`n"
    Write-Host "📝 Added MOBULA_API_KEY to .env file" -ForegroundColor Green
}

# Write back to .env
Set-Content -Path $envPath -Value $envContent -NoNewline

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           ✅ Setup Complete!                            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "Your Mobula API key has been stored in .env file.`n" -ForegroundColor Green
Write-Host "🚀 You can now use Mobula API for token discovery and analysis!`n" -ForegroundColor Yellow

