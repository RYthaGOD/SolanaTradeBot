//! DeepSeek AI client for trading analysis
//! Request/response structs used internally by AI orchestrator

use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeepSeekRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeepSeekResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDecision {
    pub action: String,  // "BUY", "SELL", "HOLD"
    pub confidence: f64, // 0.0 - 1.0
    pub reasoning: String,
    pub risk_assessment: String, // "LOW", "MEDIUM", "HIGH"
    pub suggested_size: f64,     // Percentage of capital (0-100)
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

pub struct DeepSeekClient {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    model: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_url: "https://api.deepseek.com/v1/chat/completions".to_string(),
            client: reqwest::Client::new(),
            model: "deepseek-chat".to_string(), // Free tier model
        }
    }

    /// Analyze market data and generate trading decision
    pub async fn analyze_trade(
        &self,
        symbol: &str,
        current_price: f64,
        price_history: &[f64],
        volume_history: &[f64],
        portfolio_value: f64,
        position_size: f64,
    ) -> Result<TradingDecision, Box<dyn Error>> {
        // Calculate technical indicators
        let sma_10 = if price_history.len() >= 10 {
            price_history[price_history.len() - 10..]
                .iter()
                .sum::<f64>()
                / 10.0
        } else {
            current_price
        };

        let sma_20 = if price_history.len() >= 20 {
            price_history[price_history.len() - 20..]
                .iter()
                .sum::<f64>()
                / 20.0
        } else {
            current_price
        };

        let price_change = if price_history.len() >= 2 {
            ((current_price - price_history[price_history.len() - 2])
                / price_history[price_history.len() - 2])
                * 100.0
        } else {
            0.0
        };

        let avg_volume = if volume_history.len() >= 10 {
            volume_history[volume_history.len() - 10..]
                .iter()
                .sum::<f64>()
                / 10.0
        } else {
            volume_history.last().copied().unwrap_or(0.0)
        };

        // Build context for AI
        let trend_signal = if current_price > sma_20 {
            "Bullish"
        } else {
            "Bearish"
        };
        let volume_signal = if volume_history.last().copied().unwrap_or(0.0) > avg_volume {
            "Above Average (Strong)"
        } else {
            "Below Average (Weak)"
        };
        let ma_signal = if sma_10 > sma_20 {
            "Bullish Crossover"
        } else if sma_10 < sma_20 {
            "Bearish Crossover"
        } else {
            "Neutral"
        };

        let prompt = format!(
            r#"You are an expert cryptocurrency trading AI analyzing {}.

Current Market Data:
- Current Price: ${:.2}
- Price Change (24h): {:.2}%
- SMA-10: ${:.2}
- SMA-20: ${:.2}
- Current Volume: {:.0}
- Average Volume (10 periods): {:.0}

Portfolio Status:
- Total Portfolio Value: ${:.2}
- Current Position Size: {:.4} {}
- Available Capital: ${:.2}

Technical Analysis Context:
- Trend: {}
- Volume Profile: {}
- Moving Average Signal: {}

Task: Provide a trading decision with the following JSON format:
{{
  "action": "BUY" | "SELL" | "HOLD",
  "confidence": 0.0-1.0,
  "reasoning": "Brief explanation of the decision",
  "risk_assessment": "LOW" | "MEDIUM" | "HIGH",
  "suggested_size": 0-100 (percentage of capital),
  "stop_loss": null or price level,
  "take_profit": null or price level
}}

Consider:
1. Risk management (never risk more than 10% of portfolio)
2. Market momentum and trend strength
3. Volume confirmation
4. Support/resistance levels
5. Position sizing based on confidence

Respond ONLY with valid JSON, no additional text."#,
            symbol,
            current_price,
            price_change,
            sma_10,
            sma_20,
            volume_history.last().copied().unwrap_or(0.0),
            avg_volume,
            portfolio_value,
            position_size,
            symbol,
            portfolio_value - (position_size * current_price),
            trend_signal,
            volume_signal,
            ma_signal
        );

        let request = DeepSeekRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a professional cryptocurrency trading AI. Provide precise, data-driven trading decisions in JSON format only.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            temperature: 0.3, // Lower temperature for more deterministic decisions
            max_tokens: 500,
        };

        log::debug!("Sending request to DeepSeek API for {} analysis", symbol);

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("DeepSeek API error: {}", error_text).into());
        }

        let deepseek_response: DeepSeekResponse = response.json().await?;

        if deepseek_response.choices.is_empty() {
            return Err("No choices returned from DeepSeek API".into());
        }

        let content = &deepseek_response.choices[0].message.content;

        log::info!(
            "DeepSeek AI response for {}: {} tokens used",
            symbol,
            deepseek_response.usage.total_tokens
        );
        log::debug!("AI Decision: {}", content);

        // Parse JSON response
        let decision: TradingDecision = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse AI decision: {}. Response: {}", e, content))?;

        // Validate decision
        if !["BUY", "SELL", "HOLD"].contains(&decision.action.as_str()) {
            return Err(format!("Invalid action: {}", decision.action).into());
        }

        if decision.confidence < 0.0 || decision.confidence > 1.0 {
            return Err(format!("Invalid confidence: {}", decision.confidence).into());
        }

        if decision.suggested_size < 0.0 || decision.suggested_size > 100.0 {
            return Err(format!("Invalid suggested size: {}", decision.suggested_size).into());
        }

        Ok(decision)
    }

    /// Quick risk assessment for a potential trade
    pub async fn assess_risk(
        &self,
        symbol: &str,
        action: &str,
        price: f64,
        size: f64,
        portfolio_value: f64,
    ) -> Result<String, Box<dyn Error>> {
        let risk_percentage = (size * price / portfolio_value) * 100.0;

        let prompt = format!(
            r#"Assess the risk of this trade:
Symbol: {}
Action: {}
Price: ${:.2}
Size: {:.4}
Portfolio Value: ${:.2}
Risk Percentage: {:.2}%

Provide a brief risk assessment (LOW, MEDIUM, or HIGH) with reasoning in 1-2 sentences."#,
            symbol, action, price, size, portfolio_value, risk_percentage
        );

        let request = DeepSeekRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.2,
            max_tokens: 150,
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("DeepSeek API error: {}", error_text).into());
        }

        let deepseek_response: DeepSeekResponse = response.json().await?;

        Ok(deepseek_response.choices[0].message.content.clone())
    }
}

impl Default for DeepSeekClient {
    fn default() -> Self {
        Self::new(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_client_creation() {
        let client = DeepSeekClient::new("test_api_key".to_string());
        assert_eq!(client.model, "deepseek-chat");
        assert_eq!(
            client.api_url,
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_trading_decision_validation() {
        let decision = TradingDecision {
            action: "BUY".to_string(),
            confidence: 0.75,
            reasoning: "Strong uptrend with high volume".to_string(),
            risk_assessment: "MEDIUM".to_string(),
            suggested_size: 5.0,
            stop_loss: Some(95.0),
            take_profit: Some(110.0),
        };

        assert_eq!(decision.action, "BUY");
        assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
        assert!(decision.suggested_size >= 0.0 && decision.suggested_size <= 100.0);
    }
}
