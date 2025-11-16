use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Historical price data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceDataPoint {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Historical dataset for a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataset {
    pub symbol: String,
    pub data: VecDeque<PriceDataPoint>,
    pub max_size: usize,
}

impl HistoricalDataset {
    pub fn new(symbol: String, max_size: usize) -> Self {
        Self {
            symbol,
            data: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Add new price data point
    pub fn add_data_point(&mut self, point: PriceDataPoint) {
        self.data.push_back(point);

        // Keep only max_size most recent points
        while self.data.len() > self.max_size {
            self.data.pop_front();
        }
    }

    /// Get recent N data points
    pub fn get_recent(&self, count: usize) -> Vec<PriceDataPoint> {
        self.data.iter().rev().take(count).cloned().collect()
    }

    /// Calculate price changes over different timeframes
    pub fn calculate_price_changes(&self) -> PriceChanges {
        if self.data.is_empty() {
            return PriceChanges::default();
        }

        let current_price = self.data.back().unwrap().close;

        let change_5m = self.calculate_change(current_price, 5);
        let change_15m = self.calculate_change(current_price, 15);
        let change_1h = self.calculate_change(current_price, 60);
        let change_4h = self.calculate_change(current_price, 240);
        let change_24h = self.calculate_change(current_price, 1440);

        PriceChanges {
            change_5m,
            change_15m,
            change_1h,
            change_4h,
            change_24h,
        }
    }

    fn calculate_change(&self, current_price: f64, minutes_ago: usize) -> f64 {
        if self.data.len() < minutes_ago {
            return 0.0;
        }

        let past_price = self.data[self.data.len() - minutes_ago].close;
        if past_price > 0.0 {
            ((current_price - past_price) / past_price) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate moving averages
    pub fn calculate_moving_averages(&self) -> MovingAverages {
        MovingAverages {
            sma_10: self.calculate_sma(10),
            sma_20: self.calculate_sma(20),
            sma_50: self.calculate_sma(50),
            sma_200: self.calculate_sma(200),
            ema_10: self.calculate_ema(10),
            ema_20: self.calculate_ema(20),
            ema_50: self.calculate_ema(50),
        }
    }

    fn calculate_sma(&self, period: usize) -> Option<f64> {
        if self.data.len() < period {
            return None;
        }

        let sum: f64 = self.data.iter().rev().take(period).map(|p| p.close).sum();

        Some(sum / period as f64)
    }

    fn calculate_ema(&self, period: usize) -> Option<f64> {
        if self.data.len() < period {
            return None;
        }

        let multiplier = 2.0 / (period as f64 + 1.0);
        let prices: Vec<f64> = self
            .data
            .iter()
            .rev()
            .take(period)
            .map(|p| p.close)
            .collect();

        let mut ema = prices[0];
        for price in prices.iter().skip(1) {
            ema = (price * multiplier) + (ema * (1.0 - multiplier));
        }

        Some(ema)
    }

    /// Calculate volatility (standard deviation of returns)
    pub fn calculate_volatility(&self, period: usize) -> f64 {
        if self.data.len() < period + 1 {
            return 0.0;
        }

        let returns: Vec<f64> = self
            .data
            .iter()
            .rev()
            .take(period + 1)
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| {
                let price_change = (w[0].close - w[1].close) / w[1].close;
                price_change
            })
            .collect();

        if returns.is_empty() {
            return 0.0;
        }

        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;

        variance.sqrt() * 100.0 // Convert to percentage
    }

    /// Calculate RSI (Relative Strength Index)
    pub fn calculate_rsi(&self, period: usize) -> Option<f64> {
        if self.data.len() < period + 1 {
            return None;
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        let recent_data: Vec<_> = self.data.iter().rev().take(period + 1).collect();

        for i in 0..period {
            let change = recent_data[i].close - recent_data[i + 1].close;
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(change.abs());
            }
        }

        let avg_gain = gains.iter().sum::<f64>() / period as f64;
        let avg_loss = losses.iter().sum::<f64>() / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        let rsi = 100.0 - (100.0 / (1.0 + rs));

        Some(rsi)
    }

    /// Generate features for ML/RL models
    pub fn generate_features(&self) -> HistoricalFeatures {
        let price_changes = self.calculate_price_changes();
        let moving_averages = self.calculate_moving_averages();
        let volatility = self.calculate_volatility(20);
        let rsi = self.calculate_rsi(14);

        let current_price = self.data.back().map(|p| p.close).unwrap_or(0.0);
        let volume = self.data.back().map(|p| p.volume).unwrap_or(0.0);

        // Calculate volume trend (recent vs historical average)
        let avg_volume = if !self.data.is_empty() {
            self.data.iter().map(|p| p.volume).sum::<f64>() / self.data.len() as f64
        } else {
            0.0
        };

        let volume_ratio = if avg_volume > 0.0 {
            volume / avg_volume
        } else {
            1.0
        };

        // Trend strength (compare short vs long MA)
        let trend_strength = if let (Some(ema_10), Some(ema_50)) =
            (moving_averages.ema_10, moving_averages.ema_50)
        {
            ((ema_10 - ema_50) / ema_50) * 100.0
        } else {
            0.0
        };

        HistoricalFeatures {
            symbol: self.symbol.clone(),
            current_price,
            volume,
            volume_ratio,
            price_changes,
            moving_averages,
            volatility,
            rsi,
            trend_strength,
            data_points: self.data.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PriceChanges {
    pub change_5m: f64,
    pub change_15m: f64,
    pub change_1h: f64,
    pub change_4h: f64,
    pub change_24h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovingAverages {
    pub sma_10: Option<f64>,
    pub sma_20: Option<f64>,
    pub sma_50: Option<f64>,
    pub sma_200: Option<f64>,
    pub ema_10: Option<f64>,
    pub ema_20: Option<f64>,
    pub ema_50: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalFeatures {
    pub symbol: String,
    pub current_price: f64,
    pub volume: f64,
    pub volume_ratio: f64,
    pub price_changes: PriceChanges,
    pub moving_averages: MovingAverages,
    pub volatility: f64,
    pub rsi: Option<f64>,
    pub trend_strength: f64,
    pub data_points: usize,
}

/// Historical data manager for multiple symbols
pub struct HistoricalDataManager {
    datasets: std::collections::HashMap<String, HistoricalDataset>,
    max_size_per_symbol: usize,
}

impl HistoricalDataManager {
    pub fn new(max_size_per_symbol: usize) -> Self {
        Self {
            datasets: std::collections::HashMap::new(),
            max_size_per_symbol,
        }
    }

    /// Add price data for a symbol
    pub fn add_price_data(&mut self, symbol: String, point: PriceDataPoint) {
        let dataset = self
            .datasets
            .entry(symbol.clone())
            .or_insert_with(|| HistoricalDataset::new(symbol, self.max_size_per_symbol));

        dataset.add_data_point(point);
    }

    /// Get historical features for a symbol
    pub fn get_features(&self, symbol: &str) -> Option<HistoricalFeatures> {
        self.datasets.get(symbol).map(|ds| ds.generate_features())
    }

    /// Get dataset for a symbol
    pub fn get_dataset(&self, symbol: &str) -> Option<&HistoricalDataset> {
        self.datasets.get(symbol)
    }

    /// Get all symbols with data
    pub fn get_symbols(&self) -> Vec<String> {
        self.datasets.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_historical_dataset_creation() {
        let dataset = HistoricalDataset::new("SOL/USD".to_string(), 100);
        assert_eq!(dataset.symbol, "SOL/USD");
        assert_eq!(dataset.data.len(), 0);
    }

    #[test]
    fn test_add_and_retrieve_data() {
        let mut dataset = HistoricalDataset::new("SOL/USD".to_string(), 5);

        for i in 0..10 {
            dataset.add_data_point(PriceDataPoint {
                timestamp: 1000 + i,
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 100.0 + i as f64,
                volume: 1000.0,
            });
        }

        // Should only keep last 5
        assert_eq!(dataset.data.len(), 5);

        let recent = dataset.get_recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].close, 109.0); // Most recent
    }

    #[test]
    fn test_calculate_moving_averages() {
        let mut dataset = HistoricalDataset::new("SOL/USD".to_string(), 100);

        for i in 0..50 {
            dataset.add_data_point(PriceDataPoint {
                timestamp: 1000 + i,
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 100.0 + i as f64,
                volume: 1000.0,
            });
        }

        let mas = dataset.calculate_moving_averages();
        assert!(mas.sma_10.is_some());
        assert!(mas.sma_20.is_some());
        assert!(mas.ema_10.is_some());
    }

    #[test]
    fn test_calculate_volatility() {
        let mut dataset = HistoricalDataset::new("SOL/USD".to_string(), 100);

        for i in 0..30 {
            let price = if i % 2 == 0 { 100.0 } else { 110.0 };
            dataset.add_data_point(PriceDataPoint {
                timestamp: 1000 + i,
                open: price,
                high: price + 5.0,
                low: price - 5.0,
                close: price,
                volume: 1000.0,
            });
        }

        let volatility = dataset.calculate_volatility(20);
        assert!(volatility > 0.0); // Should detect volatility
    }

    #[test]
    fn test_calculate_rsi() {
        let mut dataset = HistoricalDataset::new("SOL/USD".to_string(), 100);

        // Trending up
        for i in 0..20 {
            dataset.add_data_point(PriceDataPoint {
                timestamp: 1000 + i,
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 100.0 + i as f64,
                volume: 1000.0,
            });
        }

        let rsi = dataset.calculate_rsi(14);
        assert!(rsi.is_some());
        assert!(rsi.unwrap() > 50.0); // Should be > 50 in uptrend
    }

    #[test]
    fn test_generate_features() {
        let mut dataset = HistoricalDataset::new("SOL/USD".to_string(), 100);

        for i in 0..60 {
            dataset.add_data_point(PriceDataPoint {
                timestamp: 1000 + i,
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 100.0 + (i as f64 * 0.5),
                volume: 1000.0 * (1.0 + (i as f64 * 0.01)),
            });
        }

        let features = dataset.generate_features();
        assert_eq!(features.symbol, "SOL/USD");
        assert!(features.current_price > 0.0);
        assert!(features.volatility >= 0.0);
        assert!(features.data_points > 0);
    }
}
