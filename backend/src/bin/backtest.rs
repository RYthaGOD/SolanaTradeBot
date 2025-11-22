//! Backtesting CLI Tool
//! Run backtests on historical data to validate strategies

// Note: This binary is part of the agentburn-backend package
// Import from parent module
#[path = "../backtesting.rs"]
mod backtesting;

use backtesting::{BacktestEngine, BacktestConfig, generate_sample_data};
use chrono::Utc;
use std::env;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 BACKTESTING ENGINE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let initial_balance = args.get(1)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(10000.0);
    
    let days = args.get(2)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30);
    
    // Create backtest configuration
    let config = BacktestConfig {
        initial_balance,
        start_date: Utc::now() - chrono::Duration::days(days),
        end_date: Utc::now(),
        max_drawdown: 0.20,
        commission_rate: 0.001, // 0.1%
        slippage: 0.0005, // 0.05%
        min_confidence: 0.6,
        max_position_size_pct: 0.1,
    };
    
    println!("📊 Configuration:");
    println!("   Initial Balance: ${:.2}", config.initial_balance);
    println!("   Period: {} days", days);
    println!("   Max Drawdown: {:.1}%", config.max_drawdown * 100.0);
    println!("   Commission: {:.3}%", config.commission_rate * 100.0);
    println!("   Slippage: {:.3}%", config.slippage * 100.0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Generate sample historical data
    println!("📈 Generating historical data...");
    let historical_data = generate_sample_data(
        "SOL/USD".to_string(),
        config.start_date,
        config.end_date,
        100.0, // Initial price
    );
    
    println!("   Generated {} data points", historical_data.len());
    
    // Run backtest
    println!("🚀 Running backtest...");
    let mut engine = BacktestEngine::new(config.clone());
    let results = engine.run(historical_data).await;
    
    // Print results
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 BACKTEST RESULTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💰 Performance:");
    println!("   Initial Balance: ${:.2}", results.initial_balance);
    println!("   Final Balance: ${:.2}", results.final_balance);
    println!("   Total Return: ${:.2} ({:.2}%)", results.total_return, results.total_return_pct);
    println!("   Max Drawdown: {:.2}%", results.max_drawdown_pct);
    println!("\n📈 Trading Statistics:");
    println!("   Total Trades: {}", results.total_trades);
    println!("   Winning Trades: {} ({:.1}%)", results.winning_trades, results.win_rate * 100.0);
    println!("   Losing Trades: {}", results.losing_trades);
    println!("   Avg Win: ${:.2}", results.avg_win);
    println!("   Avg Loss: ${:.2}", results.avg_loss);
    println!("   Profit Factor: {:.2}", results.profit_factor);
    println!("\n📊 Risk Metrics:");
    println!("   Sharpe Ratio: {:.2}", results.sharpe_ratio);
    println!("   Sortino Ratio: {:.2}", results.sortino_ratio);
    println!("   Total Fees: ${:.2}", results.total_fees);
    println!("   Total Slippage: ${:.2}", results.total_slippage);
    
    if !results.symbol_performance.is_empty() {
        println!("\n🎯 Symbol Performance:");
        for (symbol, perf) in &results.symbol_performance {
            println!("   {}:", symbol);
            println!("      Trades: {} (Win Rate: {:.1}%)", perf.total_trades, perf.win_rate * 100.0);
            println!("      Total P&L: ${:.2} (Avg: ${:.2})", perf.total_pnl, perf.avg_pnl);
            println!("      Best Trade: ${:.2}", perf.best_trade);
            println!("      Worst Trade: ${:.2}", perf.worst_trade);
        }
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Export results to JSON
    let json = serde_json::to_string_pretty(&results).unwrap();
    let filename = format!("backtest_results_{}.json", Utc::now().timestamp());
    std::fs::write(&filename, json).unwrap();
    println!("💾 Results saved to: {}", filename);
}

