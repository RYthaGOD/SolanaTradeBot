use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;

/// Trade record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: String,
    pub timestamp: i64,
    pub symbol: String,
    pub action: String,        // "BUY" or "SELL"
    pub price: f64,
    pub size: f64,
    pub total_value: f64,
    pub fee: f64,
    pub pnl: f64,
    pub confidence: f64,
    pub strategy: String,
}

/// Portfolio snapshot for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub timestamp: i64,
    pub total_value: f64,
    pub cash_balance: f64,
    pub positions: HashMap<String, f64>,
    pub daily_pnl: f64,
    pub total_pnl: f64,
}

/// Performance metrics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    pub timestamp: i64,
    pub total_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
}

/// In-memory database (for simplicity - can be replaced with SQL)
pub struct Database {
    trades: Vec<TradeRecord>,
    snapshots: Vec<PortfolioSnapshot>,
    performance: Vec<PerformanceRecord>,
    data_file: String,
}

impl Database {
    pub fn new(data_file: &str) -> Self {
        let mut db = Self {
            trades: Vec::new(),
            snapshots: Vec::new(),
            performance: Vec::new(),
            data_file: data_file.to_string(),
        };

        // Load existing data if available
        if let Err(e) = db.load_from_file() {
            log::warn!("Could not load database: {}. Starting fresh.", e);
        }

        db
    }

    /// Insert a new trade record
    pub fn insert_trade(&mut self, trade: TradeRecord) -> Result<(), String> {
        log::info!("📝 Recording trade: {} {} {} at ${}", 
                  trade.action, trade.size, trade.symbol, trade.price);
        
        self.trades.push(trade);
        self.save_to_file()?;
        Ok(())
    }

    /// Get all trades
    pub fn get_all_trades(&self) -> &[TradeRecord] {
        &self.trades
    }

    /// Get trades for a specific symbol
    pub fn get_trades_by_symbol(&self, symbol: &str) -> Vec<&TradeRecord> {
        self.trades
            .iter()
            .filter(|t| t.symbol == symbol)
            .collect()
    }

    /// Get trades within a time range
    pub fn get_trades_by_timerange(&self, start: i64, end: i64) -> Vec<&TradeRecord> {
        self.trades
            .iter()
            .filter(|t| t.timestamp >= start && t.timestamp <= end)
            .collect()
    }

    /// Get recent trades (last N)
    pub fn get_recent_trades(&self, count: usize) -> Vec<&TradeRecord> {
        self.trades
            .iter()
            .rev()
            .take(count)
            .collect()
    }

    /// Insert portfolio snapshot
    pub fn insert_snapshot(&mut self, snapshot: PortfolioSnapshot) -> Result<(), String> {
        self.snapshots.push(snapshot);
        self.save_to_file()?;
        Ok(())
    }

    /// Get all snapshots
    pub fn get_all_snapshots(&self) -> &[PortfolioSnapshot] {
        &self.snapshots
    }

    /// Get recent snapshots (last N)
    pub fn get_recent_snapshots(&self, count: usize) -> Vec<&PortfolioSnapshot> {
        self.snapshots
            .iter()
            .rev()
            .take(count)
            .collect()
    }

    /// Insert performance record
    pub fn insert_performance(&mut self, perf: PerformanceRecord) -> Result<(), String> {
        self.performance.push(perf);
        self.save_to_file()?;
        Ok(())
    }

    /// Get all performance records
    pub fn get_all_performance(&self) -> &[PerformanceRecord] {
        &self.performance
    }

    /// Calculate aggregate statistics
    pub fn get_statistics(&self) -> TradingStatistics {
        let total_trades = self.trades.len();
        let winning_trades = self.trades.iter().filter(|t| t.pnl > 0.0).count();
        let losing_trades = self.trades.iter().filter(|t| t.pnl < 0.0).count();

        let total_pnl: f64 = self.trades.iter().map(|t| t.pnl).sum();
        let total_volume: f64 = self.trades.iter().map(|t| t.total_value).sum();
        let total_fees: f64 = self.trades.iter().map(|t| t.fee).sum();

        let avg_win = if winning_trades > 0 {
            self.trades
                .iter()
                .filter(|t| t.pnl > 0.0)
                .map(|t| t.pnl)
                .sum::<f64>() / winning_trades as f64
        } else {
            0.0
        };

        let avg_loss = if losing_trades > 0 {
            self.trades
                .iter()
                .filter(|t| t.pnl < 0.0)
                .map(|t| t.pnl.abs())
                .sum::<f64>() / losing_trades as f64
        } else {
            0.0
        };

        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        TradingStatistics {
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            total_pnl,
            total_volume,
            total_fees,
            avg_win,
            avg_loss,
            profit_factor: if avg_loss > 0.0 { avg_win / avg_loss } else { 0.0 },
        }
    }

    /// Save database to file
    fn save_to_file(&self) -> Result<(), String> {
        let data = DatabaseData {
            trades: self.trades.clone(),
            snapshots: self.snapshots.clone(),
            performance: self.performance.clone(),
        };

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialize database: {}", e))?;

        std::fs::write(&self.data_file, json)
            .map_err(|e| format!("Failed to write database file: {}", e))?;

        log::debug!("💾 Database saved to {}", self.data_file);
        Ok(())
    }

    /// Load database from file
    fn load_from_file(&mut self) -> Result<(), String> {
        let path = Path::new(&self.data_file);
        
        if !path.exists() {
            return Ok(()); // No file yet, start fresh
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read database file: {}", e))?;

        let data: DatabaseData = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse database: {}", e))?;

        self.trades = data.trades;
        self.snapshots = data.snapshots;
        self.performance = data.performance;

        log::info!("✅ Loaded database: {} trades, {} snapshots", 
                  self.trades.len(), self.snapshots.len());
        Ok(())
    }

    /// Clear all data (use with caution!)
    pub fn clear_all(&mut self) -> Result<(), String> {
        self.trades.clear();
        self.snapshots.clear();
        self.performance.clear();
        self.save_to_file()?;
        log::warn!("🗑️ Database cleared!");
        Ok(())
    }

    /// Export data to CSV
    pub fn export_trades_csv(&self, path: &Path) -> Result<(), String> {
        let mut csv = String::from("timestamp,symbol,action,price,size,total_value,fee,pnl,confidence,strategy\n");

        for trade in &self.trades {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                trade.timestamp,
                trade.symbol,
                trade.action,
                trade.price,
                trade.size,
                trade.total_value,
                trade.fee,
                trade.pnl,
                trade.confidence,
                trade.strategy
            ));
        }

        std::fs::write(path, csv)
            .map_err(|e| format!("Failed to export CSV: {}", e))?;

        log::info!("📊 Exported {} trades to CSV: {:?}", self.trades.len(), path);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseData {
    trades: Vec<TradeRecord>,
    snapshots: Vec<PortfolioSnapshot>,
    performance: Vec<PerformanceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStatistics {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub total_volume: f64,
    pub total_fees: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_database_creation() {
        let db = Database::new("/tmp/test_db.json");
        assert_eq!(db.get_all_trades().len(), 0);
    }

    #[test]
    fn test_insert_and_retrieve_trade() {
        let mut db = Database::new("/tmp/test_trade_db.json");
        
        let trade = TradeRecord {
            id: "test_1".to_string(),
            timestamp: 1699876543,
            symbol: "SOL/USDC".to_string(),
            action: "BUY".to_string(),
            price: 100.0,
            size: 10.0,
            total_value: 1000.0,
            fee: 1.0,
            pnl: 0.0,
            confidence: 0.8,
            strategy: "SMA".to_string(),
        };

        db.insert_trade(trade.clone()).unwrap();
        assert_eq!(db.get_all_trades().len(), 1);
        assert_eq!(db.get_all_trades()[0].symbol, "SOL/USDC");

        // Cleanup
        let _ = fs::remove_file("/tmp/test_trade_db.json");
    }

    #[test]
    fn test_statistics_calculation() {
        let mut db = Database::new("/tmp/test_stats_db.json");
        
        // Add winning trade
        db.insert_trade(TradeRecord {
            id: "1".to_string(),
            timestamp: 1,
            symbol: "SOL/USDC".to_string(),
            action: "BUY".to_string(),
            price: 100.0,
            size: 1.0,
            total_value: 100.0,
            fee: 0.1,
            pnl: 10.0,
            confidence: 0.8,
            strategy: "test".to_string(),
        }).unwrap();

        // Add losing trade
        db.insert_trade(TradeRecord {
            id: "2".to_string(),
            timestamp: 2,
            symbol: "SOL/USDC".to_string(),
            action: "SELL".to_string(),
            price: 95.0,
            size: 1.0,
            total_value: 95.0,
            fee: 0.1,
            pnl: -5.0,
            confidence: 0.6,
            strategy: "test".to_string(),
        }).unwrap();

        let stats = db.get_statistics();
        assert_eq!(stats.total_trades, 2);
        assert_eq!(stats.winning_trades, 1);
        assert_eq!(stats.losing_trades, 1);
        assert_eq!(stats.win_rate, 50.0);

        // Cleanup
        let _ = fs::remove_file("/tmp/test_stats_db.json");
    }
}
