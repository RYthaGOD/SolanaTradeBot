use reqwest::Client;
/// Jito Block Engine + Bundle Atomic Marketplace (BAM) Integration
///
/// Jito provides MEV protection and atomic bundle execution on Solana.
/// This module integrates with Jito's Block Engine for:
/// - Atomic bundle submission (all-or-nothing execution)
/// - MEV protection for trades
/// - Priority fee optimization via tips
/// - Bundle status tracking
///
/// Documentation: https://jito-labs.gitbook.io/mev/
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

/// Jito Block Engine endpoints
pub const JITO_MAINNET_URL: &str = "https://mainnet.block-engine.jito.wtf";
pub const JITO_DEVNET_URL: &str = "https://dallas.testnet.block-engine.jito.wtf";

/// Jito tip accounts for priority fees (mainnet)
pub const JITO_TIP_ACCOUNTS: [&str; 8] = [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

/// Bundle submission request
#[derive(Debug, Serialize)]
pub struct BundleRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Vec<Vec<String>>,
}

/// Bundle submission response
#[derive(Debug, Deserialize)]
pub struct BundleResponse {
    pub jsonrpc: String,
    pub result: String, // Bundle UUID
    pub id: u64,
}

/// Bundle status
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BundleStatus {
    Pending,
    Processing,
    Landed,
    Failed,
    Dropped,
}

/// Bundle status response
#[derive(Debug, Deserialize)]
pub struct BundleStatusResponse {
    pub jsonrpc: String,
    pub result: BundleStatusResult,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct BundleStatusResult {
    pub context: BundleContext,
    pub value: Vec<BundleStatusValue>,
}

#[derive(Debug, Deserialize)]
pub struct BundleContext {
    pub slot: u64,
}

#[derive(Debug, Deserialize)]
pub struct BundleStatusValue {
    pub bundle_id: String,
    pub status: BundleStatus,
    pub landed_slot: Option<u64>,
}

/// Bundle configuration
#[derive(Debug, Clone)]
pub struct BundleConfig {
    pub tip_amount_lamports: u64,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            tip_amount_lamports: 10_000, // 0.00001 SOL tip
            max_retries: 3,
            timeout_ms: 30_000,
        }
    }
}

/// Jito BAM client for atomic bundle submission
pub struct JitoBamClient {
    client: Client,
    pub block_engine_url: String,
    pub tip_accounts: Vec<Pubkey>,
    config: BundleConfig,
}

impl JitoBamClient {
    /// Create a new Jito BAM client
    pub fn new(use_mainnet: bool) -> Self {
        let block_engine_url = if use_mainnet {
            JITO_MAINNET_URL.to_string()
        } else {
            JITO_DEVNET_URL.to_string()
        };

        let tip_accounts: Vec<Pubkey> = JITO_TIP_ACCOUNTS
            .iter()
            .filter_map(|addr| addr.parse().ok())
            .collect();

        Self {
            client: Client::new(),
            block_engine_url,
            tip_accounts,
            config: BundleConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(use_mainnet: bool, config: BundleConfig) -> Self {
        let mut client = Self::new(use_mainnet);
        client.config = config;
        client
    }

    /// Submit an atomic bundle to Jito Block Engine
    /// All transactions in the bundle execute atomically or none execute
    pub async fn submit_bundle(
        &self,
        transactions: Vec<Transaction>,
    ) -> Result<String, Box<dyn Error>> {
        if transactions.is_empty() {
            return Err("Bundle must contain at least one transaction".into());
        }

        // Serialize transactions to base58
        let serialized_txs: Vec<String> = transactions
            .iter()
            .map(|tx| {
                let serialized = bincode::serialize(tx).unwrap();
                bs58::encode(serialized).into_string()
            })
            .collect();

        let request = BundleRequest {
            jsonrpc: "2.0".to_string(),
            id: Self::generate_request_id(),
            method: "sendBundle".to_string(),
            params: vec![serialized_txs],
        };

        let response = self
            .client
            .post(&format!("{}/api/v1/bundles", self.block_engine_url))
            .json(&request)
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Jito API error: {}", response.status()).into());
        }

        let bundle_response: BundleResponse = response.json().await?;
        log::info!("Bundle submitted successfully: {}", bundle_response.result);

        Ok(bundle_response.result)
    }

    /// Check the status of a submitted bundle
    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<BundleStatus, Box<dyn Error>> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": Self::generate_request_id(),
            "method": "getBundleStatuses",
            "params": [[bundle_id]]
        });

        let response = self
            .client
            .post(&format!("{}/api/v1/bundles", self.block_engine_url))
            .json(&request)
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms))
            .send()
            .await?;

        let status_response: BundleStatusResponse = response.json().await?;

        if let Some(bundle_status) = status_response.result.value.first() {
            Ok(bundle_status.status.clone())
        } else {
            Err("Bundle status not found".into())
        }
    }

    /// Wait for bundle to land (or fail)
    pub async fn wait_for_bundle(&self, bundle_id: &str) -> Result<BundleStatus, Box<dyn Error>> {
        let start = SystemTime::now();
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        loop {
            let status = self.get_bundle_status(bundle_id).await?;

            match status {
                BundleStatus::Landed | BundleStatus::Failed | BundleStatus::Dropped => {
                    return Ok(status);
                }
                BundleStatus::Pending | BundleStatus::Processing => {
                    // Continue waiting
                }
            }

            if start.elapsed()? > timeout {
                return Err("Bundle timeout exceeded".into());
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    /// Get a random tip account for priority fees
    pub fn get_random_tip_account(&self) -> Option<&Pubkey> {
        use rand::seq::SliceRandom;
        self.tip_accounts.choose(&mut rand::thread_rng())
    }

    /// Submit bundle with automatic retry on failure
    pub async fn submit_bundle_with_retry(
        &self,
        transactions: Vec<Transaction>,
    ) -> Result<String, Box<dyn Error>> {
        let mut last_error = None;

        for attempt in 1..=self.config.max_retries {
            match self.submit_bundle(transactions.clone()).await {
                Ok(bundle_id) => return Ok(bundle_id),
                Err(e) => {
                    log::warn!("Bundle submission attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        tokio::time::sleep(std::time::Duration::from_millis(1000 * attempt as u64))
                            .await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "All retry attempts failed".into()))
    }

    fn generate_request_id() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// Helper struct for building atomic trading bundles
pub struct TradingBundleBuilder {
    transactions: Vec<Transaction>,
    description: String,
}

impl TradingBundleBuilder {
    pub fn new(description: &str) -> Self {
        Self {
            transactions: Vec::new(),
            description: description.to_string(),
        }
    }

    /// Add a transaction to the bundle
    pub fn add_transaction(mut self, tx: Transaction) -> Self {
        self.transactions.push(tx);
        self
    }

    /// Add multiple transactions
    pub fn add_transactions(mut self, txs: Vec<Transaction>) -> Self {
        self.transactions.extend(txs);
        self
    }

    /// Build the final bundle
    pub fn build(self) -> Vec<Transaction> {
        log::info!(
            "Built trading bundle '{}' with {} transactions",
            self.description,
            self.transactions.len()
        );
        self.transactions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jito_client_creation() {
        let client = JitoBamClient::new(true);
        assert_eq!(client.block_engine_url, JITO_MAINNET_URL);
        assert_eq!(client.tip_accounts.len(), 8);
    }

    #[test]
    fn test_bundle_builder() {
        let builder = TradingBundleBuilder::new("Test bundle");
        let bundle = builder.build();
        assert_eq!(bundle.len(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = BundleConfig::default();
        assert_eq!(config.tip_amount_lamports, 10_000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_ms, 30_000);
    }
}
