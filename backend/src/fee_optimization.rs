use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;

/// Fee estimation based on network congestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    pub min_fee: u64,           // Minimum fee in lamports
    pub recommended_fee: u64,   // Recommended fee for normal priority
    pub priority_fee: u64,      // Fee for high priority
    pub max_fee: u64,          // Maximum reasonable fee
    pub confidence: f64,        // 0.0-1.0 confidence in estimate
}

/// Network congestion levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CongestionLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// Fee optimizer that tracks network conditions
pub struct FeeOptimizer {
    recent_fees: VecDeque<u64>,
    recent_confirmations: VecDeque<(u64, Duration)>, // (fee, confirmation_time)
    max_history: usize,
    base_fee: u64, // Base fee in lamports (5000 = 0.000005 SOL)
}

impl FeeOptimizer {
    pub fn new(base_fee: u64) -> Self {
        Self {
            recent_fees: VecDeque::with_capacity(100),
            recent_confirmations: VecDeque::with_capacity(100),
            max_history: 100,
            base_fee,
        }
    }

    /// Record a successful transaction with its fee and confirmation time
    pub fn record_transaction(&mut self, fee: u64, confirmation_time: Duration) {
        self.recent_fees.push_back(fee);
        self.recent_confirmations.push_back((fee, confirmation_time));

        if self.recent_fees.len() > self.max_history {
            self.recent_fees.pop_front();
        }

        if self.recent_confirmations.len() > self.max_history {
            self.recent_confirmations.pop_front();
        }

        log::debug!("Recorded transaction: fee={} lamports, confirmation={:?}", 
                   fee, confirmation_time);
    }

    /// Get current fee estimate based on recent transactions
    pub fn estimate_fee(&self, priority: FeePriority) -> FeeEstimate {
        if self.recent_fees.is_empty() {
            return self.default_estimate();
        }

        let avg_fee = self.calculate_average_fee();
        let congestion = self.detect_congestion();

        let multiplier = match (priority, congestion) {
            (FeePriority::Low, _) => 0.8,
            (FeePriority::Normal, CongestionLevel::Low) => 1.0,
            (FeePriority::Normal, CongestionLevel::Medium) => 1.2,
            (FeePriority::Normal, CongestionLevel::High) => 1.5,
            (FeePriority::Normal, CongestionLevel::Extreme) => 2.0,
            (FeePriority::High, CongestionLevel::Low) => 1.5,
            (FeePriority::High, CongestionLevel::Medium) => 2.0,
            (FeePriority::High, CongestionLevel::High) => 2.5,
            (FeePriority::High, CongestionLevel::Extreme) => 3.0,
        };

        let recommended = (avg_fee as f64 * multiplier) as u64;

        FeeEstimate {
            min_fee: self.base_fee,
            recommended_fee: recommended.max(self.base_fee),
            priority_fee: (recommended as f64 * 1.5) as u64,
            max_fee: self.base_fee * 10, // Cap at 10x base fee
            confidence: self.calculate_confidence(),
        }
    }

    /// Detect current network congestion level
    pub fn detect_congestion(&self) -> CongestionLevel {
        if self.recent_confirmations.is_empty() {
            return CongestionLevel::Low;
        }

        // Calculate average confirmation time
        let total_time: Duration = self.recent_confirmations
            .iter()
            .map(|(_, time)| *time)
            .sum();
        
        let avg_time = total_time / self.recent_confirmations.len() as u32;

        // Classify congestion based on confirmation times
        match avg_time.as_secs() {
            0..=5 => CongestionLevel::Low,
            6..=15 => CongestionLevel::Medium,
            16..=30 => CongestionLevel::High,
            _ => CongestionLevel::Extreme,
        }
    }

    /// Calculate average fee from recent transactions
    fn calculate_average_fee(&self) -> u64 {
        if self.recent_fees.is_empty() {
            return self.base_fee;
        }

        let sum: u64 = self.recent_fees.iter().sum();
        sum / self.recent_fees.len() as u64
    }

    /// Calculate confidence in fee estimate
    fn calculate_confidence(&self) -> f64 {
        // Confidence increases with more data points
        let sample_confidence = (self.recent_fees.len() as f64 / self.max_history as f64).min(1.0);
        
        // Reduce confidence if fees are highly variable
        let variance = self.calculate_fee_variance();
        let variance_penalty = if variance > 0.5 { 0.7 } else { 1.0 };

        (sample_confidence * variance_penalty).max(0.1)
    }

    /// Calculate coefficient of variation for fees
    fn calculate_fee_variance(&self) -> f64 {
        if self.recent_fees.len() < 2 {
            return 0.0;
        }

        let avg = self.calculate_average_fee() as f64;
        let variance: f64 = self.recent_fees
            .iter()
            .map(|&fee| {
                let diff = fee as f64 - avg;
                diff * diff
            })
            .sum::<f64>() / self.recent_fees.len() as f64;

        let std_dev = variance.sqrt();
        std_dev / avg // Coefficient of variation
    }

    /// Get default estimate when no historical data is available
    fn default_estimate(&self) -> FeeEstimate {
        FeeEstimate {
            min_fee: self.base_fee,
            recommended_fee: self.base_fee * 2,
            priority_fee: self.base_fee * 3,
            max_fee: self.base_fee * 10,
            confidence: 0.5,
        }
    }

    /// Get statistics about recent transactions
    pub fn get_stats(&self) -> FeeStats {
        let congestion = self.detect_congestion();
        let avg_time = self.calculate_avg_confirmation_time();
        
        FeeStats {
            total_transactions: self.recent_fees.len(),
            average_fee: self.calculate_average_fee(),
            min_fee: self.recent_fees.iter().min().copied().unwrap_or(self.base_fee),
            max_fee: self.recent_fees.iter().max().copied().unwrap_or(self.base_fee),
            congestion_level: format!("{:?}", congestion),
            avg_confirmation_time: format!("{:.2}s", avg_time.as_secs_f64()),
        }
    }

    fn calculate_avg_confirmation_time(&self) -> Duration {
        if self.recent_confirmations.is_empty() {
            return Duration::from_secs(0);
        }

        let total: Duration = self.recent_confirmations
            .iter()
            .map(|(_, time)| *time)
            .sum();
        
        total / self.recent_confirmations.len() as u32
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FeePriority {
    Low,      // Economy mode
    Normal,   // Standard priority
    High,     // Fast confirmation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStats {
    pub total_transactions: usize,
    pub average_fee: u64,
    pub min_fee: u64,
    pub max_fee: u64,
    pub congestion_level: String,
    pub avg_confirmation_time: String,
}

impl From<CongestionLevel> for String {
    fn from(level: CongestionLevel) -> Self {
        match level {
            CongestionLevel::Low => "Low".to_string(),
            CongestionLevel::Medium => "Medium".to_string(),
            CongestionLevel::High => "High".to_string(),
            CongestionLevel::Extreme => "Extreme".to_string(),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_optimizer_creation() {
        let optimizer = FeeOptimizer::new(5000);
        assert_eq!(optimizer.base_fee, 5000);
    }

    #[test]
    fn test_default_estimate() {
        let optimizer = FeeOptimizer::new(5000);
        let estimate = optimizer.estimate_fee(FeePriority::Normal);
        
        assert_eq!(estimate.min_fee, 5000);
        assert!(estimate.recommended_fee >= estimate.min_fee);
    }

    #[test]
    fn test_congestion_detection() {
        let mut optimizer = FeeOptimizer::new(5000);
        
        // Simulate fast confirmations
        optimizer.record_transaction(5000, Duration::from_secs(3));
        optimizer.record_transaction(5000, Duration::from_secs(4));
        
        assert_eq!(optimizer.detect_congestion(), CongestionLevel::Low);
    }

    #[test]
    fn test_priority_fee_calculation() {
        let mut optimizer = FeeOptimizer::new(5000);
        optimizer.record_transaction(10000, Duration::from_secs(5));

        let low = optimizer.estimate_fee(FeePriority::Low);
        let normal = optimizer.estimate_fee(FeePriority::Normal);
        let high = optimizer.estimate_fee(FeePriority::High);

        assert!(low.recommended_fee <= normal.recommended_fee);
        assert!(normal.recommended_fee <= high.recommended_fee);
    }
}
