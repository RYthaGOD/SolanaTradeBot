# PowerShell script to add Solana wallet private key to .env file

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║      🔐 Solana Wallet Private Key Setup                  ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "This script will add your Solana wallet private key to the .env file.`n" -ForegroundColor Yellow
Write-Host "⚠️  WARNING: Keep your private key secure! Never share it publicly.`n" -ForegroundColor Red

Write-Host "Your private key should be in base58 format (typically 88 characters)." -ForegroundColor Yellow
Write-Host "You can export it from Phantom, Solflare, or your Solana CLI wallet.`n" -ForegroundColor Yellow

$privateKey = Read-Host "Enter your Solana wallet private key (base58 format)"

# Validate format
if ([string]::IsNullOrWhiteSpace($privateKey)) {
    Write-Host "`n❌ Error: Private key cannot be empty" -ForegroundColor Red
    exit 1
}

# Base58 Solana private keys are typically 88 characters (64 bytes encoded)
if ($privateKey.Length -lt 64) {
    Write-Host "`n❌ Error: Private key too short" -ForegroundColor Red
    Write-Host "   Private keys should be at least 64 characters (base58 encoded)" -ForegroundColor Red
    exit 1
}

if ($privateKey.Length -gt 200) {
    Write-Host "`n❌ Error: Private key too long" -ForegroundColor Red
    Write-Host "   Private keys should not exceed 200 characters" -ForegroundColor Red
    exit 1
}

Write-Host "`n✅ Private key format validated!" -ForegroundColor Green
Write-Host "   Length: $($privateKey.Length) characters" -ForegroundColor Gray

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

# Check if WALLET_PRIVATE_KEY already exists
if ($envContent -match "WALLET_PRIVATE_KEY=") {
    $envContent = $envContent -replace "WALLET_PRIVATE_KEY=.*", "WALLET_PRIVATE_KEY=$privateKey"
    Write-Host "📝 Updated existing WALLET_PRIVATE_KEY in .env file" -ForegroundColor Green
} else {
    if ($envContent -and -not $envContent.EndsWith("`n")) {
        $envContent += "`n"
    }
    $envContent += "`n# Solana Wallet Configuration`n"
    $envContent += "WALLET_PRIVATE_KEY=$privateKey`n"
    Write-Host "📝 Added WALLET_PRIVATE_KEY to .env file" -ForegroundColor Green
}

# Write back to .env
Set-Content -Path $envPath -Value $envContent -NoNewline

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           ✅ Setup Complete!                            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "Your Solana wallet private key has been stored in .env file.`n" -ForegroundColor Green
Write-Host "🔒 Security reminder:" -ForegroundColor Yellow
Write-Host "   - Never commit the .env file to version control" -ForegroundColor Yellow
Write-Host "   - Keep your private key secure and never share it" -ForegroundColor Yellow
Write-Host "   - Consider using a dedicated trading wallet`n" -ForegroundColor Yellow
Write-Host "🚀 The system will now be able to:" -ForegroundColor Green
Write-Host "   - Derive your treasury PDA address" -ForegroundColor Green
Write-Host "   - Deposit SOL into the PDA for trading" -ForegroundColor Green
Write-Host "   - Execute trades on Solana blockchain`n" -ForegroundColor Green








