# PowerShell script to add DeepSeek API key to .env file

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║      🔐 DeepSeek API Key Setup                        ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "This script will add your DeepSeek API key to the .env file.`n" -ForegroundColor Yellow

$apiKey = Read-Host "Enter your DeepSeek API key (starts with 'sk-')"

# Validate format
if (-not $apiKey.StartsWith("sk-")) {
    Write-Host "`n❌ Error: Invalid API key format" -ForegroundColor Red
    Write-Host "   API keys must start with 'sk-'" -ForegroundColor Red
    exit 1
}

if ($apiKey.Length -lt 32) {
    Write-Host "`n❌ Error: API key too short" -ForegroundColor Red
    Write-Host "   API keys must be at least 32 characters" -ForegroundColor Red
    exit 1
}

Write-Host "`n✅ API key format validated!" -ForegroundColor Green

# Read or create .env file
$envPath = ".env"
$envContent = ""

if (Test-Path $envPath) {
    $envContent = Get-Content $envPath -Raw
} else {
    Write-Host "`n📄 Creating new .env file..." -ForegroundColor Yellow
}

# Check if DEEPSEEK_API_KEY already exists
if ($envContent -match "DEEPSEEK_API_KEY=") {
    $envContent = $envContent -replace "DEEPSEEK_API_KEY=.*", "DEEPSEEK_API_KEY=$apiKey"
    Write-Host "📝 Updated existing DEEPSEEK_API_KEY in .env file" -ForegroundColor Green
} else {
    if ($envContent -and -not $envContent.EndsWith("`n")) {
        $envContent += "`n"
    }
    $envContent += "`n# DeepSeek AI Configuration`n"
    $envContent += "DEEPSEEK_API_KEY=$apiKey`n"
    Write-Host "📝 Added DEEPSEEK_API_KEY to .env file" -ForegroundColor Green
}

# Write back to .env
Set-Content -Path $envPath -Value $envContent -NoNewline

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           ✅ Setup Complete!                            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "Your DeepSeek API key has been stored in .env file.`n" -ForegroundColor Green
Write-Host "🚀 You can now start the trading system with AI enabled!`n" -ForegroundColor Yellow

