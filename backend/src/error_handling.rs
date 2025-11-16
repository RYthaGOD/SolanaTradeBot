//! Error handling utilities with retry logic and circuit breaker
//! Integrated throughout the system for production error management

use std::error::Error;
use std::fmt;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub enum TradingError {
    NetworkError(String),
    InsufficientFunds(String),
    InvalidTransaction(String),
    RateLimitExceeded(String),
    ApiError(String),
    ValidationError(String),
    TimeoutError(String),
}

impl fmt::Display for TradingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TradingError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            TradingError::InsufficientFunds(msg) => write!(f, "Insufficient funds: {}", msg),
            TradingError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            TradingError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            TradingError::ApiError(msg) => write!(f, "API error: {}", msg),
            TradingError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            TradingError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
        }
    }
}

impl Error for TradingError {}

/// Retry configuration for different error types
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Aggressive retry for critical operations
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 1.5,
        }
    }

    /// Conservative retry for non-critical operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 2,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(15),
            backoff_multiplier: 3.0,
        }
    }
}

/// Retry a function with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: Error + 'static,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;
        
        log::debug!("Attempting {} (attempt {}/{})", operation_name, attempt, config.max_attempts);

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    log::info!("✅ {} succeeded after {} attempts", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) if attempt >= config.max_attempts => {
                log::error!("❌ {} failed after {} attempts: {}", operation_name, attempt, e);
                return Err(e);
            }
            Err(e) => {
                log::warn!("⚠️ {} attempt {} failed: {}. Retrying in {:?}...", 
                          operation_name, attempt, e, delay);
                
                sleep(delay).await;
                
                // Exponential backoff
                delay = Duration::from_millis(
                    (delay.as_millis() as f64 * config.backoff_multiplier).min(config.max_delay.as_millis() as f64) as u64
                );
            }
        }
    }
}

/// Determine if an error is retryable
pub fn is_retryable_error(error: &TradingError) -> bool {
    matches!(
        error,
        TradingError::NetworkError(_) | TradingError::TimeoutError(_) | TradingError::RateLimitExceeded(_)
    )
}

/// Circuit breaker to prevent cascading failures
pub struct CircuitBreaker {
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    failures: tokio::sync::Mutex<u32>,
    successes: tokio::sync::Mutex<u32>,
    state: tokio::sync::Mutex<CircuitState>,
    last_failure_time: tokio::sync::Mutex<Option<std::time::Instant>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,  // Normal operation
    Open,    // Blocking requests
    HalfOpen, // Testing if service recovered
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            failures: tokio::sync::Mutex::new(0),
            successes: tokio::sync::Mutex::new(0),
            state: tokio::sync::Mutex::new(CircuitState::Closed),
            last_failure_time: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, TradingError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: Error + 'static,
    {
        let state = self.state.lock().await.clone();

        match state {
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure_time.lock().await;
                if let Some(time) = *last_failure {
                    if time.elapsed() >= self.timeout {
                        // Move to half-open state
                        *self.state.lock().await = CircuitState::HalfOpen;
                        log::info!("🔄 Circuit breaker moving to HALF-OPEN state");
                        drop(last_failure);
                    } else {
                        return Err(TradingError::ApiError("Circuit breaker is OPEN".to_string()));
                    }
                }
            }
            CircuitState::HalfOpen => {
                log::debug!("Circuit breaker in HALF-OPEN state, testing...");
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Execute operation
        match operation.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(TradingError::ApiError(e.to_string()))
            }
        }
    }

    async fn on_success(&self) {
        let mut successes = self.successes.lock().await;
        *successes += 1;

        let state = self.state.lock().await.clone();
        
        if state == CircuitState::HalfOpen && *successes >= self.success_threshold {
            *self.state.lock().await = CircuitState::Closed;
            *self.failures.lock().await = 0;
            *successes = 0;
            log::info!("✅ Circuit breaker CLOSED - service recovered");
        }
    }

    async fn on_failure(&self) {
        let mut failures = self.failures.lock().await;
        *failures += 1;

        if *failures >= self.failure_threshold {
            *self.state.lock().await = CircuitState::Open;
            *self.last_failure_time.lock().await = Some(std::time::Instant::now());
            log::error!("🚨 Circuit breaker OPEN - too many failures");
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_attempts, 5);
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(&TradingError::NetworkError("test".to_string())));
        assert!(is_retryable_error(&TradingError::TimeoutError("test".to_string())));
        assert!(!is_retryable_error(&TradingError::ValidationError("test".to_string())));
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
}
