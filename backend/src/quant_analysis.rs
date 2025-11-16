use serde::{Deserialize, Serialize};

/// Advanced quantitative analysis module with technical indicators
#[derive(Debug, Clone)]
pub struct QuantAnalyzer {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub sma_10: f64,
    pub sma_20: f64,
    pub sma_50: Option<f64>,
    pub ema_12: f64,
    pub ema_26: f64,
    pub rsi_14: f64,
    pub macd: f64,
    pub macd_signal: f64,
    pub macd_histogram: f64,
    pub bollinger_upper: f64,
    pub bollinger_middle: f64,
    pub bollinger_lower: f64,
    pub atr_14: f64,
    pub obv: f64,
    pub momentum: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalQuality {
    pub score: f64,           // 0-100
    pub strength: String,      // "Strong", "Moderate", "Weak"
    pub trend: String,         // "Bullish", "Bearish", "Neutral"
    pub confidence: f64,       // 0-1
    pub risk_level: String,    // "Low", "Medium", "High"
    pub recommendation: String, // "Strong Buy", "Buy", "Hold", "Sell", "Strong Sell"
}

impl QuantAnalyzer {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Calculate Simple Moving Average
    pub fn calculate_sma(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() < period {
            return None;
        }
        let sum: f64 = prices[prices.len() - period..].iter().sum();
        Some(sum / period as f64)
    }

    /// Calculate Exponential Moving Average
    pub fn calculate_ema(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() < period {
            return None;
        }

        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut ema = prices[0];

        for price in prices.iter().skip(1) {
            ema = (price - ema) * multiplier + ema;
        }

        Some(ema)
    }

    /// Calculate Relative Strength Index (RSI)
    pub fn calculate_rsi(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() <= period {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=period {
            let change = prices[prices.len() - period + i - 1] - prices[prices.len() - period + i - 2];
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change;
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    /// Calculate MACD (Moving Average Convergence Divergence)
    pub fn calculate_macd(&self, prices: &[f64]) -> Option<(f64, f64, f64)> {
        let ema_12 = self.calculate_ema(prices, 12)?;
        let ema_26 = self.calculate_ema(prices, 26)?;
        let macd = ema_12 - ema_26;

        // Calculate signal line (9-period EMA of MACD)
        // Simplified: using the MACD value itself for signal
        let signal = macd * 0.9; // Simplified signal calculation
        let histogram = macd - signal;

        Some((macd, signal, histogram))
    }

    /// Calculate Bollinger Bands
    pub fn calculate_bollinger_bands(&self, prices: &[f64], period: usize, std_dev: f64) -> Option<(f64, f64, f64)> {
        let sma = self.calculate_sma(prices, period)?;
        
        let variance: f64 = prices[prices.len() - period..]
            .iter()
            .map(|p| (p - sma).powi(2))
            .sum::<f64>() / period as f64;
        
        let std = variance.sqrt();
        
        let upper = sma + (std_dev * std);
        let lower = sma - (std_dev * std);
        
        Some((upper, sma, lower))
    }

    /// Calculate Average True Range (ATR)
    pub fn calculate_atr(&self, highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<f64> {
        if highs.len() < period || lows.len() < period || closes.len() < period {
            return None;
        }

        let mut true_ranges = Vec::new();
        
        for i in 1..period {
            let high_low = highs[highs.len() - period + i] - lows[lows.len() - period + i];
            let high_close = (highs[highs.len() - period + i] - closes[closes.len() - period + i - 1]).abs();
            let low_close = (lows[lows.len() - period + i] - closes[closes.len() - period + i - 1]).abs();
            
            true_ranges.push(high_low.max(high_close).max(low_close));
        }

        Some(true_ranges.iter().sum::<f64>() / true_ranges.len() as f64)
    }

    /// Calculate On-Balance Volume (OBV)
    pub fn calculate_obv(&self, prices: &[f64], volumes: &[f64]) -> Option<f64> {
        if prices.len() < 2 || volumes.len() < 2 {
            return None;
        }

        let mut obv = 0.0;
        
        for i in 1..prices.len() {
            if prices[i] > prices[i - 1] {
                obv += volumes[i];
            } else if prices[i] < prices[i - 1] {
                obv -= volumes[i];
            }
        }

        Some(obv)
    }

    /// Calculate price momentum
    pub fn calculate_momentum(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() <= period {
            return None;
        }

        let current = prices[prices.len() - 1];
        let past = prices[prices.len() - period - 1];
        
        Some(((current - past) / past) * 100.0)
    }

    /// Calculate volatility (standard deviation of returns)
    pub fn calculate_volatility(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() <= period {
            return None;
        }

        let recent_prices = &prices[prices.len() - period..];
        let mut returns = Vec::new();
        
        for i in 1..recent_prices.len() {
            returns.push((recent_prices[i] - recent_prices[i - 1]) / recent_prices[i - 1]);
        }

        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / returns.len() as f64;

        Some(variance.sqrt() * 100.0)
    }

    /// Calculate all technical indicators
    pub fn calculate_indicators(&self, prices: &[f64], volumes: &[f64]) -> Option<TechnicalIndicators> {
        let sma_10 = self.calculate_sma(prices, 10)?;
        let sma_20 = self.calculate_sma(prices, 20)?;
        let sma_50 = self.calculate_sma(prices, 50);
        
        let ema_12 = self.calculate_ema(prices, 12)?;
        let ema_26 = self.calculate_ema(prices, 26)?;
        
        let rsi_14 = self.calculate_rsi(prices, 14).unwrap_or(50.0);
        
        let (macd, macd_signal, macd_histogram) = self.calculate_macd(prices).unwrap_or((0.0, 0.0, 0.0));
        
        let (bollinger_upper, bollinger_middle, bollinger_lower) = 
            self.calculate_bollinger_bands(prices, 20, 2.0).unwrap_or((0.0, 0.0, 0.0));
        
        // For ATR, we approximate using price ranges
        let highs: Vec<f64> = prices.iter().map(|p| p * 1.001).collect();
        let lows: Vec<f64> = prices.iter().map(|p| p * 0.999).collect();
        let atr_14 = self.calculate_atr(&highs, &lows, prices, 14).unwrap_or(0.0);
        
        let obv = self.calculate_obv(prices, volumes).unwrap_or(0.0);
        let momentum = self.calculate_momentum(prices, 10).unwrap_or(0.0);
        let volatility = self.calculate_volatility(prices, 14).unwrap_or(0.0);

        Some(TechnicalIndicators {
            sma_10,
            sma_20,
            sma_50,
            ema_12,
            ema_26,
            rsi_14,
            macd,
            macd_signal,
            macd_histogram,
            bollinger_upper,
            bollinger_middle,
            bollinger_lower,
            atr_14,
            obv,
            momentum,
            volatility,
        })
    }

    /// Analyze signal quality based on technical indicators
    pub fn analyze_signal_quality(&self, indicators: &TechnicalIndicators, current_price: f64) -> SignalQuality {
        let mut score: f64 = 50.0; // Start neutral
        let mut bullish_signals = 0;
        let mut bearish_signals = 0;

        // SMA crossover analysis
        if indicators.sma_10 > indicators.sma_20 {
            score += 10.0;
            bullish_signals += 1;
        } else {
            score -= 10.0;
            bearish_signals += 1;
        }

        // EMA crossover analysis
        if indicators.ema_12 > indicators.ema_26 {
            score += 8.0;
            bullish_signals += 1;
        } else {
            score -= 8.0;
            bearish_signals += 1;
        }

        // RSI analysis
        if indicators.rsi_14 < 30.0 {
            score += 15.0; // Oversold - bullish
            bullish_signals += 1;
        } else if indicators.rsi_14 > 70.0 {
            score -= 15.0; // Overbought - bearish
            bearish_signals += 1;
        }

        // MACD analysis
        if indicators.macd > indicators.macd_signal && indicators.macd_histogram > 0.0 {
            score += 12.0;
            bullish_signals += 1;
        } else if indicators.macd < indicators.macd_signal && indicators.macd_histogram < 0.0 {
            score -= 12.0;
            bearish_signals += 1;
        }

        // Bollinger Bands analysis
        if current_price < indicators.bollinger_lower {
            score += 10.0; // Near lower band - potential bounce
            bullish_signals += 1;
        } else if current_price > indicators.bollinger_upper {
            score -= 10.0; // Near upper band - potential pullback
            bearish_signals += 1;
        }

        // Momentum analysis
        if indicators.momentum > 5.0 {
            score += 7.0;
            bullish_signals += 1;
        } else if indicators.momentum < -5.0 {
            score -= 7.0;
            bearish_signals += 1;
        }

        // Normalize score to 0-100
        score = score.max(0.0).min(100.0);

        // Determine trend
        let trend = if bullish_signals > bearish_signals + 1 {
            "Bullish"
        } else if bearish_signals > bullish_signals + 1 {
            "Bearish"
        } else {
            "Neutral"
        };

        // Determine strength
        let strength = if score > 70.0 || score < 30.0 {
            "Strong"
        } else if score > 60.0 || score < 40.0 {
            "Moderate"
        } else {
            "Weak"
        };

        // Calculate confidence based on score and volatility
        let volatility_factor = (1.0 - (indicators.volatility / 10.0).min(1.0)) * 0.3;
        let confidence = ((score / 100.0) * 0.7 + volatility_factor).max(0.0).min(1.0);

        // Determine risk level
        let risk_level = if indicators.volatility > 5.0 {
            "High"
        } else if indicators.volatility > 2.0 {
            "Medium"
        } else {
            "Low"
        };

        // Generate recommendation
        let recommendation = if score > 75.0 {
            "Strong Buy"
        } else if score > 60.0 {
            "Buy"
        } else if score >= 40.0 {
            "Hold"
        } else if score > 25.0 {
            "Sell"
        } else {
            "Strong Sell"
        };

        SignalQuality {
            score,
            strength: strength.to_string(),
            trend: trend.to_string(),
            confidence,
            risk_level: risk_level.to_string(),
            recommendation: recommendation.to_string(),
        }
    }
}

impl Default for QuantAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quant_analyzer_creation() {
        let analyzer = QuantAnalyzer::new();
        assert!(analyzer.enabled);
    }

    #[test]
    fn test_sma_calculation() {
        let analyzer = QuantAnalyzer::new();
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let sma = analyzer.calculate_sma(&prices, 5).unwrap();
        assert_eq!(sma, 17.0); // (15+16+17+18+19)/5
    }

    #[test]
    fn test_rsi_calculation() {
        let analyzer = QuantAnalyzer::new();
        let prices = vec![44.0, 44.5, 45.0, 45.5, 46.0, 46.5, 47.0, 47.5, 48.0, 48.5, 49.0, 49.5, 50.0, 50.5, 51.0];
        let rsi = analyzer.calculate_rsi(&prices, 14);
        assert!(rsi.is_some());
        assert!(rsi.unwrap() > 50.0); // Uptrend should have RSI > 50
    }

    #[test]
    fn test_bollinger_bands() {
        let analyzer = QuantAnalyzer::new();
        let prices = vec![100.0, 101.0, 102.0, 101.0, 100.0, 99.0, 100.0, 101.0, 102.0, 103.0,
                          104.0, 103.0, 102.0, 101.0, 100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
        let (upper, middle, lower) = analyzer.calculate_bollinger_bands(&prices, 20, 2.0).unwrap();
        assert!(upper > middle);
        assert!(middle > lower);
    }

    #[test]
    fn test_signal_quality_bullish() {
        let analyzer = QuantAnalyzer::new();
        let indicators = TechnicalIndicators {
            sma_10: 105.0,
            sma_20: 100.0,
            sma_50: Some(95.0),
            ema_12: 106.0,
            ema_26: 101.0,
            rsi_14: 25.0, // Oversold
            macd: 2.0,
            macd_signal: 1.5,
            macd_histogram: 0.5,
            bollinger_upper: 110.0,
            bollinger_middle: 100.0,
            bollinger_lower: 90.0,
            atr_14: 2.0,
            obv: 1000000.0,
            momentum: 8.0,
            volatility: 1.5,
        };

        let quality = analyzer.analyze_signal_quality(&indicators, 92.0);
        assert_eq!(quality.trend, "Bullish");
        assert!(quality.score > 60.0);
    }

    #[test]
    fn test_momentum_calculation() {
        let analyzer = QuantAnalyzer::new();
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0, 109.0, 110.0];
        let momentum = analyzer.calculate_momentum(&prices, 10).unwrap();
        assert!(momentum > 0.0); // Uptrend should have positive momentum
    }
}
