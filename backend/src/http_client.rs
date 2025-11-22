use std::sync::Arc;
use reqwest::Client;

/// Shared HTTP client with connection pooling for better performance
/// This reduces overhead from creating new connections for each request
pub struct SharedHttpClient;

impl SharedHttpClient {
    /// Create a new HTTP client with optimized settings for API calls
    /// - Connection pooling enabled
    /// - Keep-alive connections
    /// - Timeout configuration
    /// - User agent set
    pub fn new() -> Client {
        Client::builder()
            .pool_max_idle_per_host(10) // Keep 10 idle connections per host
            .pool_idle_timeout(std::time::Duration::from_secs(90)) // Keep connections alive for 90s
            .timeout(std::time::Duration::from_secs(30)) // 30s timeout
            .connect_timeout(std::time::Duration::from_secs(10)) // 10s connection timeout
            .tcp_keepalive(std::time::Duration::from_secs(60)) // TCP keep-alive
            .user_agent("AgentBurn-Solana-Trader/1.0")
            .build()
            .expect("Failed to create HTTP client")
    }
    
    /// Get a shared HTTP client instance (singleton pattern)
    /// This ensures all API clients use the same connection pool
    pub fn shared() -> Arc<Client> {
        use std::sync::OnceLock;
        static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();
        CLIENT.get_or_init(|| Arc::new(Self::new())).clone()
    }
}

