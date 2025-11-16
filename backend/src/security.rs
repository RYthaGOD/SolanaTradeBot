use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Simple rate limiter for API endpoints
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Check if a request from an IP should be allowed
    pub async fn check_rate_limit(&self, ip: String) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Get or create request history for this IP
        let ip_requests = requests.entry(ip).or_insert_with(Vec::new);

        // Remove expired requests
        ip_requests.retain(|&time| now.duration_since(time) < self.window);

        // Check if under limit
        if ip_requests.len() < self.max_requests {
            ip_requests.push(now);
            true
        } else {
            log::warn!("Rate limit exceeded for IP");
            false
        }
    }

    /// Clean up old entries periodically
    pub async fn cleanup(&self) {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        requests.retain(|_, times| {
            times.retain(|&time| now.duration_since(time) < self.window);
            !times.is_empty()
        });
    }
}

/// Security headers middleware
pub fn with_security_headers() -> warp::reply::WithHeader<warp::reply::Response> {
    let response = warp::reply::Response::new("".into());
    warp::reply::with_header(response, "X-Content-Type-Options", "nosniff")
}

/// CORS configuration for production
pub fn cors_config() -> warp::cors::Builder {
    warp::cors()
        .allow_origins(vec![
            "http://localhost:5000",
            "http://0.0.0.0:5000",
            "https://solana-trade-bot.vercel.app",
        ])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(vec!["content-type", "authorization", "x-api-key"])
        .max_age(3600)
}

/// Input validation helpers
pub fn validate_wallet_address(address: &str) -> bool {
    // Basic Solana address validation (base58, 32-44 chars)
    address.len() >= 32 && address.len() <= 44 && address.chars().all(|c| c.is_alphanumeric())
}

pub fn validate_amount(amount: f64) -> bool {
    amount > 0.0 && amount.is_finite()
}

pub fn sanitize_symbol(symbol: &str) -> String {
    // Remove any non-alphanumeric characters except /
    symbol
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '/')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wallet_address() {
        assert!(validate_wallet_address("11111111111111111111111111111111"));
        assert!(!validate_wallet_address("invalid"));
        assert!(!validate_wallet_address(""));
    }

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(1.0));
        assert!(validate_amount(0.0001));
        assert!(!validate_amount(0.0));
        assert!(!validate_amount(-1.0));
        assert!(!validate_amount(f64::NAN));
        assert!(!validate_amount(f64::INFINITY));
    }

    #[test]
    fn test_sanitize_symbol() {
        assert_eq!(sanitize_symbol("SOL/USDC"), "SOL/USDC");
        assert_eq!(sanitize_symbol("BTC-USDC"), "BTCUSDC");
        assert_eq!(sanitize_symbol("ETH@USDC"), "ETHUSDC");
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip = "127.0.0.1".to_string();

        // First 3 requests should pass
        assert!(limiter.check_rate_limit(ip.clone()).await);
        assert!(limiter.check_rate_limit(ip.clone()).await);
        assert!(limiter.check_rate_limit(ip.clone()).await);

        // 4th request should fail
        assert!(!limiter.check_rate_limit(ip.clone()).await);
    }
}
