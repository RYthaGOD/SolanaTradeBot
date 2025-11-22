# Code Review - Improvements Summary

## 🔴 Critical Improvements Needed

### 1. **Use Circuit Breaker `.call()` Method Instead of Manual State Checks**

**Current Issue**: Code manually checks circuit breaker state with `get_state()` instead of using `.call()` method.

**Problem**:
- Circuit breaker doesn't properly track successes/failures
- Manual state checks don't integrate with circuit breaker's failure tracking
- Circuit breaker state can become stale

**Location**: 
- `backend/src/dex_screener.rs` (lines 276-277, 379)
- `backend/src/switchboard_oracle.rs` 
- `backend/src/pumpfun.rs`

**Fix**: Wrap API calls with `circuit_breaker.call()` which automatically tracks successes/failures.

**Example Current Code**:
```rust
if let Some(ref cb) = self.circuit_breaker {
    let cb_state = cb.lock().await.get_state().await;
    if matches!(cb_state, CircuitState::Open) {
        return Err("Circuit breaker is open".into());
    }
}
let result = client.get(&url).send().await?;
```

**Improved Code**:
```rust
if let Some(ref cb) = self.circuit_breaker {
    let cb_clone = cb.clone();
    cb_clone.lock().await.call(async move {
        client.get(&url).send().await
            .map_err(|e| TradingError::NetworkError(e.to_string()))
    }).await?;
}
```

### 2. **Use `retry_with_backoff_retryable` Instead of `retry_with_backoff`**

**Current Issue**: Code uses `retry_with_backoff` which retries ALL errors, even non-retryable ones.

**Problem**:
- Wastes resources retrying validation errors
- Slower failure for non-retryable errors
- Doesn't respect error semantics

**Location**: 
- `backend/src/dex_screener.rs` (recently added)
- Should check all API clients

**Fix**: Use `retry_with_backoff_retryable` for TradingError types, or add error checking in closure.

**Example Current Code**:
```rust
let result: Result<Response, TradingError> = retry_with_backoff(
    || { /* operation */ },
    RetryConfig::default(),
    "operation name",
).await;
```

**Improved Code**:
```rust
let result: Result<Response, TradingError> = retry_with_backoff_retryable(
    || { /* operation */ },
    RetryConfig::default(),
    "operation name",
).await;
```

### 3. **Map HTTP Status Codes to TradingError Before Retry Logic**

**Current Issue**: HTTP status codes aren't properly mapped to TradingError types before retry.

**Problem**:
- `is_retryable_error()` can't correctly identify retryable errors
- Status codes like 429 (rate limit) aren't mapped to `RateLimitExceeded`
- 5xx errors aren't mapped to `NetworkError`

**Location**: All API clients

**Fix**: Use `map_http_status_to_error()` helper function before error handling.

**Example Current Code**:
```rust
if !response.status().is_success() {
    let status = response.status();
    let error_text = response.text().await.unwrap_or_default();
    return Err(TradingError::ApiError(format!("HTTP {}: {}", status, error_text)));
}
```

**Improved Code**:
```rust
if !response.status().is_success() {
    let status = response.status().as_u16();
    let error_text = response.text().await.unwrap_or_default();
    return Err(map_http_status_to_error(status, error_text));
}
```

### 4. **Detect and Map Reqwest Timeout Errors**

**Current Issue**: Reqwest timeout errors aren't detected and mapped to `TimeoutError`.

**Problem**:
- Timeout errors might not be properly retried
- Error type doesn't reflect the actual issue (timeout vs network)

**Location**: All API clients using reqwest

**Fix**: Catch reqwest timeout errors specifically and map to `TradingError::TimeoutError`.

**Example Improved Code**:
```rust
client.get(&url).send().await
    .map_err(|e| {
        let error_str = e.to_string();
        if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
            TradingError::TimeoutError(format!("Request timeout: {}", e))
        } else if error_str.contains("dns") || error_str.contains("connection") {
            TradingError::NetworkError(format!("Network error: {}", e))
        } else {
            TradingError::ApiError(format!("Request failed: {}", e))
        }
    })?;
```

### 5. **Integrate Circuit Breaker with Retry Logic**

**Current Issue**: Circuit breaker and retry logic are separate - circuit breaker checks happen before retry.

**Problem**:
- Could retry even when circuit breaker is open
- Circuit breaker doesn't see individual retry attempts as failures
- Inefficient error handling flow

**Fix**: Use circuit breaker `.call()` method which wraps the entire retry operation.

**Example Improved Code**:
```rust
if let Some(ref cb) = self.circuit_breaker {
    let cb_clone = cb.clone();
    cb_clone.lock().await.call(async move {
        retry_with_backoff_retryable(
            || {
                Box::pin(async move {
                    client.get(&url).send().await
                        .map_err(|e| map_reqwest_error(e))
                })
            },
            RetryConfig::default(),
            "API call",
        ).await
    }).await?;
}
```

## 🟡 Important Improvements

### 6. **Service-Specific Retry Configurations**

**Current Issue**: All services use `RetryConfig::default()`.

**Problem**:
- Critical services (trading execution) get same retry config as non-critical (price feeds)
- No differentiation based on operation importance

**Fix**: Use service-specific retry configs:
- `RetryConfig::aggressive()` for critical operations (trades, withdrawals)
- `RetryConfig::default()` for normal operations (price feeds, analytics)
- `RetryConfig::conservative()` for non-critical operations (logging, stats)

**Example**:
```rust
// For critical trading operations
let config = RetryConfig::aggressive(); // 5 attempts, faster backoff

// For price feeds
let config = RetryConfig::default(); // 3 attempts

// For analytics
let config = RetryConfig::conservative(); // 2 attempts, slower backoff
```

### 7. **Enhanced Error Context**

**Current Issue**: Errors don't always include enough context for debugging.

**Fix**: Add URL, status code, timestamp, and operation name to error messages.

**Example**:
```rust
TradingError::ApiError(format!(
    "{} failed: HTTP {} at {} - {}",
    operation_name, status, url, error_text
))
```

### 8. **Remove Duplicate Error Checking Logic**

**Current Issue**: Code has duplicate error checking (lines 119-123 in error_handling.rs).

**Problem**: Redundant code that checks if attempt >= max_attempts twice.

**Fix**: Remove duplicate check since it's already handled by the `if attempt >= config.max_attempts` guard.

## 🟢 Nice-to-Have Improvements

### 9. **Structured Logging**

**Issue**: Some log messages could be more structured for better parsing.

**Fix**: Use structured logging with fields (operation, attempt, error_type, etc.).

### 10. **Metrics/Monitoring Integration**

**Issue**: No metrics collection for retry attempts, circuit breaker state changes.

**Fix**: Add metrics for:
- Retry attempt counts
- Circuit breaker state transitions
- Error rates by type
- API call latencies

## Implementation Priority

1. **🔴 CRITICAL**: Use circuit breaker `.call()` method (Issue #1)
2. **🔴 CRITICAL**: Map HTTP status codes properly (Issue #3)
3. **🔴 CRITICAL**: Use `retry_with_backoff_retryable` (Issue #2)
4. **🟡 IMPORTANT**: Detect timeout errors (Issue #4)
5. **🟡 IMPORTANT**: Service-specific retry configs (Issue #6)
6. **🟢 NICE-TO-HAVE**: Enhanced error context (Issue #7)
7. **🟢 NICE-TO-HAVE**: Remove duplicate code (Issue #8)

