//! Twitter Sentiment Analysis Client
//! Integrates with Python ML-based sentiment analysis service
//! Uses DistilRoBERTa fine-tuned for financial news sentiment

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::error_handling::{TradingError, retry_with_backoff_retryable, RetryConfig, map_http_status_to_error};
use crate::http_client::SharedHttpClient;
use reqwest::Client;

/// Twitter sentiment data from ML analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterSentimentData {
    pub symbol: String,
    /// Weighted polarity score from -1.0 (negative) to 1.0 (positive)
    pub weighted_polarity: f64,
    /// Overall sentiment classification
    pub sentiment: String, // "Positive", "Negative", "Neutral"
    /// Tweet volume growth percentage
    pub volume_growth: f64,
    /// Community growth metrics
    pub community_growth: CommunityMetrics,
    /// Engagement metrics
    pub engagement_metrics: EngagementMetrics,
    /// Analysis timestamp
    pub timestamp: String,
}

/// Community growth metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMetrics {
    pub avg_followers: f64,
    pub total_followers: i64,
    pub top_accounts: Vec<AccountData>,
}

/// Twitter account data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub username: String,
    pub followers: i64,
    pub tweet_count: i64,
    #[serde(default)]
    pub listed_count: i64,
}

/// Engagement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementMetrics {
    pub avg_likes: f64,
    pub avg_retweets: f64,
    pub total_tweets: usize,
}

/// Request payload for sentiment analysis
#[derive(Debug, Clone, Serialize)]
struct SentimentRequest {
    symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    community_accounts: Option<Vec<String>>,
}

/// Twitter Sentiment Analysis Client
/// Connects to Python FastAPI service for ML-based sentiment analysis
pub struct TwitterSentimentClient {
    service_url: String,
    client: Arc<Client>,
}

impl TwitterSentimentClient {
    /// Create a new Twitter sentiment client
    /// 
    /// # Arguments
    /// * `service_url` - Base URL of the Python sentiment service (e.g., "http://localhost:8000")
    pub fn new(service_url: String) -> Self {
        Self {
            service_url: service_url.trim_end_matches('/').to_string(),
            client: SharedHttpClient::shared(),
        }
    }

    /// Get sentiment analysis for a symbol
    /// 
    /// # Arguments
    /// * `symbol` - Token symbol to analyze (e.g., "DOGE", "SHIB")
    /// * `search_term` - Optional custom Twitter search query
    /// * `community_accounts` - Optional list of Twitter usernames to track
    /// 
    /// # Returns
    /// * `Ok(TwitterSentimentData)` - Sentiment analysis results
    /// * `Err(TradingError)` - Error if analysis fails
    pub async fn get_sentiment(
        &self,
        symbol: &str,
        search_term: Option<String>,
        community_accounts: Option<Vec<String>>,
    ) -> Result<TwitterSentimentData, TradingError> {
        let url = format!("{}/api/sentiment", self.service_url);
        
        let request = SentimentRequest {
            symbol: symbol.to_string(),
            search_term,
            community_accounts,
        };

        let retry_config = RetryConfig::conservative();
        
        let result = retry_with_backoff_retryable(
            || {
                let client = self.client.clone();
                let url = url.clone();
                let request = request.clone();
                
                Box::pin(async move {
                    let response = client
                        .post(&url)
                        .json(&request)
                        .send()
                        .await
                        .map_err(|e| {
                            let error_str = e.to_string();
                            if e.is_timeout() || error_str.contains("timeout") || error_str.contains("timed out") {
                                TradingError::TimeoutError(format!("Twitter sentiment service timeout: {}", e))
                            } else if error_str.contains("dns") || error_str.contains("connection") || error_str.contains("No such host") {
                                TradingError::NetworkError(format!("Twitter sentiment service connection error: {}", e))
                            } else {
                                TradingError::NetworkError(format!("Twitter sentiment API error: {}", e))
                            }
                        })?;

                    if !response.status().is_success() {
                        let status = response.status().as_u16();
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let mapped_error = map_http_status_to_error(status, error_text);
                        return Err(mapped_error);
                    }

                    let data: TwitterSentimentData = response
                        .json()
                        .await
                        .map_err(|e| TradingError::ApiError(format!("Failed to parse sentiment response: {}", e)))?;

                    Ok(data)
                })
            },
            retry_config,
            "Twitter sentiment analysis",
        ).await;

        result
    }

    /// Check if the sentiment service is healthy
    /// 
    /// # Returns
    /// * `Ok(true)` - Service is healthy
    /// * `Ok(false)` - Service is not responding
    /// * `Err(TradingError)` - Network error
    pub async fn health_check(&self) -> Result<bool, TradingError> {
        let url = format!("{}/health", self.service_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                log::warn!("Twitter sentiment service health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get the service URL
    pub fn service_url(&self) -> &str {
        &self.service_url
    }
}

impl Clone for TwitterSentimentClient {
    fn clone(&self) -> Self {
        Self {
            service_url: self.service_url.clone(),
            client: self.client.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running Python service
    async fn test_health_check() {
        let client = TwitterSentimentClient::new("http://localhost:8000".to_string());
        let healthy = client.health_check().await;
        assert!(healthy.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires running Python service and Twitter API credentials
    async fn test_get_sentiment() {
        let client = TwitterSentimentClient::new("http://localhost:8000".to_string());
        let result = client.get_sentiment("DOGE", None, None).await;
        // This will fail if service is not running, which is expected
        if let Ok(data) = result {
            assert!(!data.symbol.is_empty());
            assert!(data.weighted_polarity >= -1.0 && data.weighted_polarity <= 1.0);
        }
    }
}

