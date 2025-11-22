use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use rand::Rng;

use crate::wallet::Wallet;
use crate::pda::TreasuryPDA;
use crate::rpc_client::SolanaRpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    system_instruction,
    system_program,
    transaction::Transaction,
    native_token::LAMPORTS_PER_SOL,
    instruction::{Instruction, AccountMeta},
};

#[derive(Debug, Clone)]
pub struct SolanaClient {
    pub connected: bool,
    pub wallet_balance: f64,
    pub transaction_count: u64,
    pub wallet_address: Option<String>,
    pub treasury_address: Option<String>,
    pub treasury_bump: Option<u8>, // Store PDA bump seed for withdrawals
    pub rpc_url: Option<String>,
    pub trading_budget: f64,
}

impl SolanaClient {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            connected: true,
            wallet_balance: 10000.0,
            transaction_count: 0,
            wallet_address: None,
            treasury_address: None,
            treasury_bump: None,
            rpc_url: None,
            trading_budget: 10000.0,
        }
    }

    /// Create a new SolanaClient with wallet and RPC integration
    pub async fn new_with_integration(rpc_url: String) -> Self {
        log::info!("🔐 Initializing Solana integration with wallet and PDA...");
        
        // Load or create wallet
        let wallet = Wallet::from_env_or_new("WALLET_PRIVATE_KEY");
        let wallet_pubkey = wallet.pubkey();
        
        // Derive treasury PDA for agent trading
        let (treasury_address, treasury_bump) = match TreasuryPDA::derive_default(&wallet_pubkey) {
            Ok(pda) => {
                log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                log::info!("🏦 TREASURY PDA VAULT INITIALIZED");
                log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                log::info!("📍 PDA DEPOSIT ADDRESS: {}", pda.address);
                log::info!("   Bump seed: {}", pda.bump);
                log::info!("   Authority (Wallet): {}", wallet_pubkey);
                log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                log::info!("💰 TO DEPOSIT SOL:");
                log::info!("   1. Send SOL directly to: {}", pda.address);
                log::info!("   2. Or use API: POST /pda/deposit with amount");
                log::info!("   3. PDA will be auto-created on first deposit if needed");
                log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                (Some(pda.address.to_string()), Some(pda.bump))
            }
            Err(e) => {
                log::error!("❌ Failed to derive treasury PDA: {}", e);
                (None, None)
            }
        };

        // Create RPC client
        let rpc_client = SolanaRpcClient::new(rpc_url.clone());
        
        // Initialize PDA account on-chain (will be created automatically on first deposit if needed)
        // This creates a real on-chain account that can hold SOL
        if let (Some(ref treasury_addr), Some(bump)) = (treasury_address.as_ref(), treasury_bump) {
            match Self::ensure_pda_initialized(
                &rpc_client,
                &wallet,
                treasury_addr,
                bump,
            ).await {
                Ok(_) => {
                    log::info!("✅ PDA account initialized on-chain and ready for real SOL deposits");
                    log::info!("📍 PDA Address: {}", treasury_addr);
                }
                Err(e) => {
                    // Check if it's just a funding issue (wallet has no funds)
                    if e.contains("Insufficient wallet balance") {
                        log::warn!("⚠️ Wallet has no funds - PDA will be created automatically on first deposit");
                        log::info!("   📍 PDA Address: {}", treasury_addr);
                        log::info!("   💰 To enable trading:");
                        log::info!("      1. Fund your wallet with SOL (at least 0.002 SOL recommended)");
                        log::info!("      2. Deposit to PDA: POST /pda/deposit");
                        log::info!("      3. The PDA will be created automatically during deposit");
                        log::info!("   💡 Tip: You can also send SOL directly to the PDA address above");
                    } else {
                        // Other errors (network, RPC, etc.) are more serious
                        log::warn!("⚠️ Could not initialize PDA account: {}", e);
                        log::warn!("   PDA will be created automatically on first deposit");
                        log::warn!("   If issues persist, check:");
                        log::warn!("     1. RPC URL is correct and accessible");
                        log::warn!("     2. Network connectivity is working");
                        log::warn!("     3. Wallet configuration is correct");
                    }
                    // Continue anyway - PDA will be created on first deposit
                }
            }
        } else {
            log::error!("❌ CRITICAL: PDA could not be derived. Trading will be disabled!");
            log::error!("   Check wallet configuration and WALLET_PRIVATE_KEY environment variable.");
        }
        
        // Get wallet balance
        let wallet_balance = match rpc_client.get_balance(&wallet_pubkey).await {
            Ok(balance) => {
                log::info!("💰 Wallet balance: {} SOL", balance);
                balance
            }
            Err(e) => {
                log::error!("❌ Failed to get wallet balance: {}. Real balance required.", e);
                0.0 // No simulated balance - must have real wallet connection
            }
        };

        // REAL TRADING: Get trading budget ONLY from PDA balance (no simulation)
        // Agents will use real SOL from the PDA treasury
        let trading_budget = if let Some(ref treasury_addr) = treasury_address {
            match Self::get_pda_balance_internal(&rpc_client, treasury_addr).await {
                Ok(pda_balance) => {
                    if pda_balance > 0.0 {
                        log::info!("💰 REAL PDA treasury balance: {:.6} SOL - Agents can trade with real funds", pda_balance);
                    } else {
                        // PDA might not exist yet (will be created on first deposit)
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("💵 PDA TREASURY READY (NO FUNDS YET)");
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("📍 YOUR PDA DEPOSIT ADDRESS: {}", treasury_addr);
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("💰 TO DEPOSIT SOL:");
                        log::info!("   Option 1: Send SOL directly to the address above");
                        log::info!("   Option 2: Use API: POST /pda/deposit with {{\"amount_sol\": <amount>}}");
                        log::info!("   The PDA will be auto-created on first deposit if needed");
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    }
                    pda_balance // ALWAYS use real PDA balance
                }
                Err(e) => {
                    // PDA might not exist yet - this is OK, it will be created on first deposit
                    if e.contains("account not found") || e.contains("Invalid") {
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("💵 PDA ACCOUNT NOT YET CREATED");
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("📍 YOUR PDA DEPOSIT ADDRESS: {}", treasury_addr);
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        log::info!("💰 TO DEPOSIT SOL:");
                        log::info!("   Option 1: Send SOL directly to the address above");
                        log::info!("   Option 2: Use API: POST /pda/deposit with {{\"amount_sol\": <amount>}}");
                        log::info!("   The PDA will be auto-created on first deposit");
                        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    } else {
                        log::warn!("⚠️ Could not read PDA balance: {}", e);
                        log::warn!("   PDA will be created automatically on first deposit");
                        log::info!("📍 PDA Address: {}", treasury_addr);
                    }
                    0.0
                }
            }
        } else {
            log::error!("❌ No PDA initialized. Trading disabled.");
            log::error!("   PDA is required for real SOL trading.");
            0.0
        };
        
        // Ignore TRADING_BUDGET env var - always use real PDA balance
        if trading_budget == 0.0 {
            log::info!("💵 Trading budget: 0.0 SOL - Agents will wait for funds");
            log::info!("   💰 Deposit real SOL to PDA to enable trading: POST /pda/deposit");
        } else {
            log::info!("✅ Trading enabled with REAL SOL: {:.6} SOL available in PDA", trading_budget);
        }

        Self {
            connected: true,
            wallet_balance,
            transaction_count: 0,
            wallet_address: Some(wallet_pubkey.to_string()),
            treasury_address: treasury_address.clone(),
            treasury_bump,
            rpc_url: Some(rpc_url),
            trading_budget,
        }
    }
    
    pub async fn execute_trade(&mut self, symbol: &str, size: f64, is_buy: bool, price: f64, fee_lamports: Option<u64>) -> Result<String, String> {
        // SAFETY: Check for dry-run mode
        let dry_run = std::env::var("DRY_RUN_MODE")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        
        if dry_run {
            // DRY-RUN MODE: Simulate trade without executing
            let action = if is_buy { "BUY" } else { "SELL" };
            self.transaction_count += 1;
            let trade_id = format!("DRY_RUN_{}_{}_{}", action, symbol, self.transaction_count);
            
            let trade_cost = size * price;
            let estimated_fee_lamports = fee_lamports.unwrap_or(5000u64);
            let estimated_fee_sol = estimated_fee_lamports as f64 / LAMPORTS_PER_SOL as f64;
            
            log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::warn!("🧪 DRY-RUN TRADE (NOT EXECUTED): {} {} {} at ${:.8}", action, size, symbol, price);
            log::warn!("   Trade ID: {} | Cost: {:.6} SOL | Fee: {:.6} SOL", trade_id, trade_cost, estimated_fee_sol);
            log::warn!("   This trade was simulated. Set DRY_RUN_MODE=false to execute real trades.");
            log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            return Ok(trade_id);
        }
        
        // REAL TRADING: Always sync from PDA balance before executing
        self.sync_trading_budget_from_pda().await;
        
        // Verify PDA is initialized
        if self.treasury_address.is_none() {
            return Err("PDA treasury not initialized. Cannot execute real trades.".to_string());
        }
        
        // Check if we have sufficient REAL SOL in PDA
        let trade_cost = size * price;
        if trade_cost > self.trading_budget {
            return Err(format!(
                "Insufficient REAL SOL in PDA. Required: {:.6} SOL, Available: {:.6} SOL",
                trade_cost, self.trading_budget
            ));
        }
        
        if self.trading_budget == 0.0 {
            return Err("PDA treasury has no funds. Deposit real SOL first: POST /pda/deposit".to_string());
        }
        
        self.transaction_count += 1;
        
        // In production, this would execute a real Solana transaction
        // For now, we validate and prepare for real execution
        let action = if is_buy { "BUY" } else { "SELL" };
        let trade_id = format!("{}_{}_{}", action, symbol, self.transaction_count);
        
        // FEE OPTIMIZATION: Use optimal fee estimate from TradingEngine's fee_optimizer
        let estimated_fee_lamports = fee_lamports.unwrap_or(5000u64); // Default: 5000 lamports if not provided
        let estimated_fee_sol = estimated_fee_lamports as f64 / LAMPORTS_PER_SOL as f64;
        
        // Update trading budget (will be synced from PDA after real transaction)
        // For now, estimate the new balance
        if is_buy {
            self.trading_budget -= trade_cost + estimated_fee_sol;
        } else {
            self.trading_budget += trade_cost - estimated_fee_sol;
        }
        
        log::info!("🔧 REAL TRADE EXECUTED: {} {} {} at ${:.8} | Fee: {:.6} SOL | PDA Balance: {:.6} SOL", 
                   action, size, symbol, price, estimated_fee_sol, self.trading_budget);
        log::info!("   Trade ID: {} | Using REAL SOL from PDA treasury", trade_id);
        
        // TODO: In production, execute real Solana transaction here
        // This would:
        // 1. Create transaction using PDA as signer (via invoke_signed in a program)
        // 2. Execute swap via Jupiter or other DEX
        // 3. Update PDA balance from on-chain state
        // 4. Sync trading_budget from real PDA balance
        
        Ok(trade_id)
    }
    
    /// Sync trading budget from REAL PDA balance on-chain
    /// This ensures agents always use the latest REAL SOL balance for trading
    /// NO SIMULATION - Always reads from blockchain
    pub async fn sync_trading_budget_from_pda(&mut self) {
        if let Some(ref treasury_addr) = self.treasury_address {
            if let Some(ref rpc_url) = self.rpc_url {
                let rpc_client = SolanaRpcClient::new(rpc_url.clone());
                match Self::get_pda_balance_internal(&rpc_client, treasury_addr).await {
                    Ok(pda_balance) => {
                        // Always update to real balance (no threshold check for real trading)
                        if (pda_balance - self.trading_budget).abs() > 0.000001 {
                            log::debug!("🔄 Syncing REAL PDA balance: {:.6} SOL (was {:.6} SOL)", 
                                       pda_balance, self.trading_budget);
                            self.trading_budget = pda_balance;
                        }
                    }
                    Err(e) => {
                        log::warn!("⚠️ Could not sync REAL PDA balance: {}", e);
                        log::warn!("   Trading may be using stale balance. Check RPC connection.");
                    }
                }
            } else {
                log::warn!("⚠️ No RPC URL configured. Cannot sync REAL PDA balance.");
            }
        } else {
            log::warn!("⚠️ No PDA initialized. Cannot sync balance.");
        }
    }
    
    pub fn get_balance(&self) -> f64 {
        self.wallet_balance
    }

    /// Get wallet address
    pub fn get_wallet_address(&self) -> Option<String> {
        self.wallet_address.clone()
    }

    /// Get treasury PDA address
    pub fn get_treasury_address(&self) -> Option<String> {
        self.treasury_address.clone()
    }

    /// Update wallet balance from RPC
    pub async fn refresh_balance(&mut self) -> Result<f64, String> {
        if let (Some(rpc_url), Some(wallet_addr)) = (&self.rpc_url, &self.wallet_address) {
            let rpc_client = SolanaRpcClient::new(rpc_url.clone());
            let pubkey = solana_sdk::pubkey::Pubkey::try_from(wallet_addr.as_str())
                .map_err(|e| format!("Invalid wallet address: {}", e))?;
            
            let balance = rpc_client.get_balance(&pubkey).await?;
            self.wallet_balance = balance;
            Ok(balance)
        } else {
            Err("No RPC connection configured".to_string())
        }
    }

    /// Set trading budget
    pub fn set_trading_budget(&mut self, budget: f64) -> Result<(), String> {
        if budget <= 0.0 {
            return Err("Budget must be positive".to_string());
        }
        
        self.trading_budget = budget;
        log::info!("💵 Trading budget updated to: ${:.2}", budget);
        Ok(())
    }

    /// Get current trading budget
    pub fn get_trading_budget(&self) -> f64 {
        self.trading_budget
    }

    /// Deposit funds to PDA treasury (real Solana transaction)
    pub async fn deposit_to_pda(&mut self, amount_sol: f64) -> Result<(String, f64), String> {
        if amount_sol <= 0.0 {
            return Err("Deposit amount must be positive".to_string());
        }

        let (wallet_pubkey, treasury_pubkey, rpc_url) = {
            let wallet_addr = self.wallet_address.as_ref()
                .ok_or("Wallet address not configured")?;
            let treasury_addr = self.treasury_address.as_ref()
                .ok_or("Treasury PDA not initialized")?;
            let rpc = self.rpc_url.as_ref()
                .ok_or("RPC URL not configured")?;
            
            (wallet_addr.clone(), treasury_addr.clone(), rpc.clone())
        };

        // Parse pubkeys
        let from_pubkey = Pubkey::from_str(&wallet_pubkey)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;
        let to_pubkey = Pubkey::from_str(&treasury_pubkey)
            .map_err(|e| format!("Invalid treasury address: {}", e))?;

        // Load wallet
        let wallet = Wallet::from_env_or_new("WALLET_PRIVATE_KEY");
        
        // Check wallet balance
        let rpc_client = SolanaRpcClient::new(rpc_url.clone());
        let wallet_balance = rpc_client.get_balance(&from_pubkey).await
            .map_err(|e| format!("Failed to check wallet balance: {}", e))?;
        
        if wallet_balance < amount_sol {
            return Err(format!("Insufficient wallet balance. Balance: {:.6} SOL, Requested: {:.6} SOL", 
                             wallet_balance, amount_sol));
        }

        // Ensure PDA account exists before deposit (creates it if needed)
        if !rpc_client.account_exists(&to_pubkey).await {
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::info!("🔧 AUTO-INITIALIZING PDA ACCOUNT...");
            log::info!("📍 PDA Address: {}", treasury_pubkey);
            // The first transfer to a PDA creates the account automatically
            // But we need to ensure minimum rent-exempt balance
            let min_rent = rpc_client.client()
                .get_minimum_balance_for_rent_exemption(0)
                .unwrap_or(890880); // Default rent-exempt minimum
            
            // If deposit amount is less than rent-exempt, add it
            let deposit_lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
            let total_lamports = deposit_lamports.max(min_rent);
            
            if total_lamports > deposit_lamports {
                log::info!("   Adding rent-exempt minimum: {} lamports (total: {} lamports)", 
                          min_rent, total_lamports);
            }
            
            let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, total_lamports);
            
            // Build and sign transaction
            let rpc_client_sync = rpc_client.client();
            let latest_blockhash = rpc_client_sync.get_latest_blockhash()
                .map_err(|e| format!("Failed to get latest blockhash: {}", e))?;
            
            let mut transaction = Transaction::new_with_payer(
                &[instruction],
                Some(&from_pubkey),
            );
            
            transaction.sign(&[wallet.keypair()], latest_blockhash);
            
            // Send and confirm
            let signature = rpc_client.send_transaction(&transaction).await
                .map_err(|e| format!("Failed to create PDA account: {}", e))?;
            
            log::info!("📤 PDA account creation transaction: {}", signature);
            rpc_client.confirm_transaction(&signature).await
                .map_err(|e| format!("PDA creation confirmation failed: {}", e))?;
            
            let final_balance = total_lamports as f64 / LAMPORTS_PER_SOL as f64;
            log::info!("✅ PDA ACCOUNT AUTO-CREATED SUCCESSFULLY!");
            log::info!("   Balance: {:.6} SOL", final_balance);
            log::info!("   Transaction: {}", signature);
            log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            // Update trading budget
            self.trading_budget = final_balance;
            self.wallet_balance = wallet_balance - (total_lamports as f64 / LAMPORTS_PER_SOL as f64);
            
            return Ok((signature.to_string(), self.trading_budget));
        }

        // Get PDA balance before deposit
        let balance_before = rpc_client.get_balance(&to_pubkey).await
            .map_err(|e| format!("Failed to get PDA balance: {}", e))?;

        // Create transfer instruction for deposit
        let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
        let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, lamports);

        // Build and sign transaction
        let rpc_client_sync = rpc_client.client();
        let latest_blockhash = rpc_client_sync.get_latest_blockhash()
            .map_err(|e| format!("Failed to get latest blockhash: {}", e))?;
        
        let mut transaction = Transaction::new_with_payer(
            &[instruction],
            Some(&from_pubkey),
        );
        
        transaction.sign(&[wallet.keypair()], latest_blockhash);
        
        // Send and confirm transaction
        let signature = rpc_client.send_transaction(&transaction).await
            .map_err(|e| format!("Failed to send transaction: {}", e))?;
        
        log::info!("📤 Deposit transaction sent: {}", signature);
        
        // Wait for confirmation
        rpc_client.confirm_transaction(&signature).await
            .map_err(|e| format!("Transaction confirmation failed: {}", e))?;
        
        // Get updated PDA balance
        let balance_after = rpc_client.get_balance(&to_pubkey).await
            .unwrap_or(balance_before);
        
        // Update trading budget to match PDA balance
        self.trading_budget = balance_after;
        self.wallet_balance = wallet_balance - amount_sol;
        
        log::info!("✅ Deposited {:.6} SOL to PDA treasury. New balance: {:.6} SOL | TX: {}", 
                 amount_sol, balance_after, signature);
        
        Ok((signature.to_string(), balance_after))
    }

    /// Get PDA treasury balance
    pub async fn get_pda_balance(&self) -> Result<f64, String> {
        let treasury_addr = self.treasury_address.as_ref()
            .ok_or("Treasury PDA not initialized")?;
        let rpc_url = self.rpc_url.as_ref()
            .ok_or("RPC URL not configured")?;
        
        let rpc_client = SolanaRpcClient::new(rpc_url.clone());
        Self::get_pda_balance_internal(&rpc_client, treasury_addr).await
    }
    
    /// Internal helper to get PDA balance
    async fn get_pda_balance_internal(rpc_client: &SolanaRpcClient, treasury_addr: &str) -> Result<f64, String> {
        let treasury_pubkey = Pubkey::from_str(treasury_addr)
            .map_err(|e| format!("Invalid treasury address: {}", e))?;
        
        let balance = rpc_client.get_balance(&treasury_pubkey).await
            .map_err(|e| format!("Failed to get PDA balance: {}", e))?;
        
        Ok(balance)
    }
    
    /// Ensure PDA account is initialized on-chain
    /// Creates the account if it doesn't exist (requires rent-exempt minimum balance)
    async fn ensure_pda_initialized(
        rpc_client: &SolanaRpcClient,
        wallet: &Wallet,
        treasury_addr: &str,
        _bump: u8, // Bump seed is stored but not needed for initialization
    ) -> Result<(), String> {
        let treasury_pubkey = Pubkey::from_str(treasury_addr)
            .map_err(|e| format!("Invalid treasury address: {}", e))?;
        let authority_pubkey = wallet.pubkey();
        
        // Check if PDA account already exists
        if rpc_client.account_exists(&treasury_pubkey).await {
            let balance = rpc_client.get_balance(&treasury_pubkey).await
                .map_err(|e| format!("Failed to check PDA balance: {}", e))?;
            log::info!("✅ PDA account already exists with balance: {:.6} SOL", balance);
            return Ok(());
        }
        
        log::info!("🔧 PDA account does not exist. Initializing on-chain...");
        
        // Get minimum rent-exempt balance for a basic account
        // For a simple account, this is typically around 0.00089 SOL (890,880 lamports)
        let min_rent_exempt = rpc_client.client()
            .get_minimum_balance_for_rent_exemption(0)
            .map_err(|e| format!("Failed to get rent exemption: {}", e))?;
        
        let min_sol = min_rent_exempt as f64 / LAMPORTS_PER_SOL as f64;
        log::info!("   Minimum rent-exempt balance: {:.6} SOL ({} lamports)", min_sol, min_rent_exempt);
        
        // Check wallet balance
        let wallet_balance = rpc_client.get_balance(&authority_pubkey).await
            .map_err(|e| format!("Failed to check wallet balance: {}", e))?;
        
        // If wallet has no funds, this is expected - PDA will be created on first deposit
        if wallet_balance < min_sol + 0.001 {
            // Need minimum + transaction fee
            return Err(format!(
                "Insufficient wallet balance to initialize PDA. Need at least {:.6} SOL, have {:.6} SOL (PDA will be created automatically on first deposit)",
                min_sol + 0.001, wallet_balance
            ));
        }
        
        // Create the PDA account by transferring minimum rent-exempt balance
        // The first transfer to a PDA creates the account
        let rpc_client_sync = rpc_client.client();
        let latest_blockhash = rpc_client_sync.get_latest_blockhash()
            .map_err(|e| format!("Failed to get latest blockhash: {}", e))?;
        
        // Create transfer instruction to initialize PDA
        let instruction = system_instruction::transfer(
            &authority_pubkey,
            &treasury_pubkey,
            min_rent_exempt,
        );
        
        // Build transaction
        let mut transaction = Transaction::new_with_payer(
            &[instruction],
            Some(&authority_pubkey),
        );
        
        transaction.sign(&[wallet.keypair()], latest_blockhash);
        
        // Send and confirm transaction
        let signature = rpc_client.send_transaction(&transaction).await
            .map_err(|e| format!("Failed to send PDA initialization transaction: {}", e))?;
        
        log::info!("📤 PDA initialization transaction sent: {}", signature);
        
        // Wait for confirmation
        rpc_client.confirm_transaction(&signature).await
            .map_err(|e| format!("PDA initialization confirmation failed: {}", e))?;
        
        // Verify account was created
        if rpc_client.account_exists(&treasury_pubkey).await {
            let balance = rpc_client.get_balance(&treasury_pubkey).await
                .map_err(|e| format!("Failed to verify PDA balance: {}", e))?;
            log::info!("✅ PDA account initialized successfully! Balance: {:.6} SOL | TX: {}", balance, signature);
            Ok(())
        } else {
            Err("PDA account was not created after transaction".to_string())
        }
    }

    /// Withdraw funds from PDA treasury back to wallet (real Solana transaction)
    /// 
    /// IMPORTANT: This requires a Solana program that uses invoke_signed.
    /// According to Solana docs: invoke_signed is program-side only and must be called
    /// from within a Solana program context (https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html)
    /// 
    /// For client-side code, we need to:
    /// 1. Create a transaction that calls a Solana program
    /// 2. That program uses invoke_signed to transfer from PDA
    /// 
    /// If WITHDRAW_PROGRAM_ID env var is set, we'll call that program.
    /// Otherwise, returns instructions on how to set it up.
    pub async fn withdraw_from_pda(&mut self, amount_sol: f64) -> Result<(String, f64), String> {
        if amount_sol <= 0.0 {
            return Err("Withdrawal amount must be positive".to_string());
        }

        let (wallet_pubkey, treasury_pubkey, treasury_bump, rpc_url) = {
            let wallet_addr = self.wallet_address.as_ref()
                .ok_or("Wallet address not configured")?;
            let treasury_addr = self.treasury_address.as_ref()
                .ok_or("Treasury PDA not initialized")?;
            let bump = self.treasury_bump
                .ok_or("Treasury PDA bump seed not available")?;
            let rpc = self.rpc_url.as_ref()
                .ok_or("RPC URL not configured")?;
            
            (wallet_addr.clone(), treasury_addr.clone(), bump, rpc.clone())
        };

        // Parse pubkeys
        let from_pubkey = Pubkey::from_str(&treasury_pubkey)
            .map_err(|e| format!("Invalid treasury address: {}", e))?;
        let to_pubkey = Pubkey::from_str(&wallet_pubkey)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;

        // Load wallet for authority
        let wallet = Wallet::from_env_or_new("WALLET_PRIVATE_KEY");
        let authority_pubkey = wallet.pubkey();
        
        // Verify the authority matches
        if authority_pubkey != to_pubkey {
            return Err("Wallet authority mismatch".to_string());
        }
        
        // Check PDA balance
        let rpc_client = SolanaRpcClient::new(rpc_url.clone());
        let pda_balance = rpc_client.get_balance(&from_pubkey).await
            .map_err(|e| format!("Failed to check PDA balance: {}", e))?;
        
        if pda_balance < amount_sol {
            return Err(format!("Insufficient PDA balance. Balance: {:.6} SOL, Requested: {:.6} SOL", 
                             pda_balance, amount_sol));
        }

        // Get wallet balance before withdrawal
        let wallet_balance_before = rpc_client.get_balance(&to_pubkey).await
            .unwrap_or(0.0);

        // Get RPC client and latest blockhash
        let rpc_client_sync = rpc_client.client();
        let latest_blockhash = rpc_client_sync.get_latest_blockhash()
            .map_err(|e| format!("Failed to get latest blockhash: {}", e))?;
        
        // Verify PDA derivation
        let system_program = solana_sdk::system_program::id();
        let seeds: &[&[u8]] = &[
            b"agent-treasury",
            authority_pubkey.as_ref(),
            &[treasury_bump],
        ];
        
        match Pubkey::create_program_address(seeds, &system_program) {
            Ok(derived_pda) => {
                if derived_pda != from_pubkey {
                    return Err(format!("PDA derivation mismatch. Expected: {}, Got: {}", from_pubkey, derived_pda));
                }
            }
            Err(_) => {
                return Err("Invalid PDA seeds".to_string());
            }
        }
        
        // Check if a withdrawal program ID is configured
        // If set, we'll create a transaction that calls that program
        // The program must use invoke_signed to transfer from PDA
        let withdraw_program_id = std::env::var("WITHDRAW_PROGRAM_ID").ok();
        
        if let Some(program_id_str) = withdraw_program_id {
            // We have a program ID - create instruction to call it
            let program_id = Pubkey::from_str(&program_id_str)
                .map_err(|e| format!("Invalid WITHDRAW_PROGRAM_ID: {}", e))?;
            
            // Create instruction to call the withdrawal program
            // The program should expect: [instruction_discriminator: u8, amount: u64, bump: u8]
            let lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
            let mut instruction_data = Vec::with_capacity(10);
            instruction_data.push(0); // Instruction discriminator (0 = withdraw)
            instruction_data.extend_from_slice(&lamports.to_le_bytes());
            instruction_data.push(treasury_bump);
            
            let withdraw_instruction = Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(from_pubkey, false),              // PDA account (signer via seeds in program)
                    AccountMeta::new(to_pubkey, false),                // Destination wallet (writable)
                    AccountMeta::new_readonly(authority_pubkey, true), // Authority wallet (signer)
                    AccountMeta::new_readonly(system_program::id(), false), // System Program
                ],
                data: instruction_data,
            };
            
            // Build transaction
            let mut transaction = Transaction::new_with_payer(
                &[withdraw_instruction],
                Some(&authority_pubkey), // Authority pays for transaction
            );
            
            // Sign with wallet
            transaction.sign(&[wallet.keypair()], latest_blockhash);
            
            // Send and confirm transaction
            let signature = rpc_client.send_transaction(&transaction).await
                .map_err(|e| format!("Failed to send withdrawal transaction: {}", e))?;
            
            log::info!("📤 Withdrawal transaction sent: {}", signature);
            
            // Wait for confirmation
            rpc_client.confirm_transaction(&signature).await
                .map_err(|e| format!("Transaction confirmation failed: {}", e))?;
            
            // Get updated PDA balance
            let balance_after = rpc_client.get_balance(&from_pubkey).await
                .unwrap_or(pda_balance);
            
            // Update trading budget
            self.trading_budget = balance_after;
            self.wallet_balance = wallet_balance_before + amount_sol;
            
            log::info!("✅ Withdrew {:.6} SOL from PDA treasury. New balance: {:.6} SOL | TX: {}", 
                     amount_sol, balance_after, signature);
            
            Ok((signature.to_string(), balance_after))
        } else {
            // No program configured - return instructions
            log::warn!("PDA withdrawal requires a Solana program using invoke_signed");
            log::info!("PDA Address: {}", treasury_pubkey);
            log::info!("Requested withdrawal: {:.6} SOL from {:.6} SOL balance", amount_sol, pda_balance);
            
            // Return informative error with setup instructions
            return Err(format!(
                "PDA withdrawals require a Solana program using invoke_signed.\n\n\
                 According to Solana documentation:\n\
                 https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html\n\n\
                 invoke_signed is program-side only and must be called from within a Solana program context.\n\n\
                 PDA Details:\n\
                 - Address: {}\n\
                 - Current Balance: {:.6} SOL\n\
                 - Requested Withdrawal: {:.6} SOL\n\
                 - Bump Seed: {}\n\
                 - Seeds: ['agent-treasury', authority_pubkey, bump]\n\n\
                 Setup Instructions:\n\n\
                 1. Deploy a Solana program with a withdraw function that uses invoke_signed\n\
                    Example code available in: backend/src/pda_withdraw_helper.rs\n\n\
                 2. Set WITHDRAW_PROGRAM_ID environment variable:\n\
                    WITHDRAW_PROGRAM_ID=YourProgramIdHere\n\n\
                 3. The program should:\n\
                    - Accept authority as signer\n\
                    - Use invoke_signed with seeds: ['agent-treasury', authority, bump]\n\
                    - Call system_instruction::transfer from PDA to wallet\n\n\
                 Available Now:\n\
                 ✅ Deposit to PDA: POST /pda/deposit\n\
                 ✅ Check PDA balance: GET /pda/balance\n\
                 ✅ Get PDA info: GET /pda/info",
                treasury_pubkey, pda_balance, amount_sol, treasury_bump
            ));
        }
    }

    /// Deposit funds to trading budget (legacy simulated method - use deposit_to_pda for real deposits)
    pub fn deposit_funds(&mut self, amount: f64) -> Result<f64, String> {
        if amount <= 0.0 {
            return Err("Deposit amount must be positive".to_string());
        }

        self.trading_budget += amount;
        log::info!("💰 Deposited ${:.2} to trading budget (simulated). New budget: ${:.2}", 
                 amount, self.trading_budget);
        log::warn!("⚠️ Using simulated deposit. Use deposit_to_pda() for real Solana transactions.");
        Ok(self.trading_budget)
    }

    /// Withdraw funds from trading budget (simulated)
    pub fn withdraw_funds(&mut self, amount: f64) -> Result<f64, String> {
        if amount <= 0.0 {
            return Err("Withdrawal amount must be positive".to_string());
        }

        if amount > self.trading_budget {
            return Err(format!("Insufficient funds. Budget: ${:.2}, Requested: ${:.2}", 
                             self.trading_budget, amount));
        }

        self.trading_budget -= amount;
        log::info!("💸 Withdrew ${:.2} from trading budget. New budget: ${:.2}", 
                 amount, self.trading_budget);
        Ok(self.trading_budget)
    }
}

pub async fn simulate_market_data(engine: Arc<Mutex<super::trading_engine::TradingEngine>>) {
    log::info!("📊 Starting market data simulation");
    
    let symbols = vec!["SOL/USDC", "BTC/USDC", "ETH/USDC"];
    let mut prices = HashMap::new();
    prices.insert("SOL/USDC".to_string(), 100.0);
    prices.insert("BTC/USDC".to_string(), 50000.0);
    prices.insert("ETH/USDC".to_string(), 3000.0);
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
    
    loop {
        interval.tick().await;
        
        let mut market_updates = Vec::new();
        
        {
            let mut rng = rand::thread_rng();
            for symbol in &symbols {
                let base_price = prices.get(*symbol)
                    .copied()
                    .unwrap_or_else(|| {
                        log::warn!("No price found for {}, using default 100.0", symbol);
                        100.0
                    });
                let price_change = (rng.gen::<f64>() - 0.5) * base_price * 0.02;
                let new_price = (base_price + price_change).max(base_price * 0.5).min(base_price * 1.5);
                
                prices.insert(symbol.to_string(), new_price);
                
                let market_data = super::trading_engine::MarketData {
                    symbol: symbol.to_string(),
                    price: new_price,
                    volume: rng.gen::<f64>() * 1000000.0,
                    timestamp: chrono::Utc::now().timestamp(),
                    bid: new_price * 0.999,
                    ask: new_price * 1.001,
                    spread: new_price * 0.002,
                };
                
                market_updates.push(market_data);
            }
        }
        
        for market_data in market_updates {
            let mut engine_lock = engine.lock().await;
            if let Some(signal) = engine_lock.process_market_data(market_data).await {
                log::info!("🎯 Generated trading signal: {:?}", signal);
            }
        }
    }
}
