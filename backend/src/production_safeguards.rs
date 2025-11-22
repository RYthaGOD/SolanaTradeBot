//! Production Safeguards for Live Trading
//! Critical safety checks and limits to prevent catastrophic losses

use serde::Serialize;
use chrono::Utc;

/// Production safety configuration
#[derive(Debug, Clone)]
pub struct ProductionConfig {
    /// Maximum position size as percentage of capital
    pub max_position_size_pct: f64,
    /// Maximum total exposure as percentage of capital
    pub max_total_exposure_pct: f64,
    /// Maximum daily loss as percentage of capital
    pub max_daily_loss_pct: f64,
    /// Maximum drawdown before trading stops
    pub max_drawdown_pct: f64,
    /// Minimum confidence required for trades
    pub min_confidence: f64,
    /// Maximum trades per day
    pub max_trades_per_day: usize,
    /// Maximum trades per hour
    pub max_trades_per_hour: usize,
    /// Emergency stop enabled
    pub emergency_stop_enabled: bool,
    /// Require manual confirmation for large trades
    pub require_confirmation_above_pct: f64,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            max_position_size_pct: 0.05, // 5% max per position
            max_total_exposure_pct: 0.30, // 30% max total exposure
            max_daily_loss_pct: 0.05, // 5% max daily loss
            max_drawdown_pct: 0.20, // 20% max drawdown
            min_confidence: 0.75, // 75% minimum confidence
            max_trades_per_day: 50,
            max_trades_per_hour: 10,
            emergency_stop_enabled: true,
            require_confirmation_above_pct: 0.10, // Require confirmation for >10% positions
        }
    }
}

/// Production safety monitor
pub struct ProductionSafetyMonitor {
    config: ProductionConfig,
    daily_pnl: f64,
    daily_trades: usize,
    hourly_trades: usize,
    last_trade_time: i64,
    last_hour_reset: i64,
    last_day_reset: i64,
    emergency_stop: bool,
    initial_capital: f64,
    peak_capital: f64,
    current_capital: f64,
}

impl ProductionSafetyMonitor {
    /// Create a new production safety monitor
    pub fn new(config: ProductionConfig, initial_capital: f64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            config,
            daily_pnl: 0.0,
            daily_trades: 0,
            hourly_trades: 0,
            last_trade_time: now,
            last_hour_reset: now,
            last_day_reset: now,
            emergency_stop: false,
            initial_capital,
            peak_capital: initial_capital,
            current_capital: initial_capital,
        }
    }
    
    /// Check if a trade is safe to execute
    pub fn validate_trade(
        &mut self,
        _position_size: f64,
        position_value: f64,
        confidence: f64,
        current_capital: f64,
    ) -> Result<(), SafetyViolation> {
        let now = Utc::now().timestamp();
        
        // Reset counters if needed
        self.reset_counters_if_needed(now);
        
        // Update capital tracking
        self.current_capital = current_capital;
        if current_capital > self.peak_capital {
            self.peak_capital = current_capital;
        }
        
        // Check emergency stop
        if self.emergency_stop {
            return Err(SafetyViolation::EmergencyStop);
        }
        
        // Check max drawdown
        let drawdown = if self.peak_capital > 0.0 {
            (self.peak_capital - current_capital) / self.peak_capital
        } else {
            0.0
        };
        
        if drawdown > self.config.max_drawdown_pct {
            self.emergency_stop = true;
            return Err(SafetyViolation::MaxDrawdownExceeded {
                drawdown: drawdown * 100.0,
                limit: self.config.max_drawdown_pct * 100.0,
            });
        }
        
        // Check daily loss
        if self.daily_pnl < 0.0 {
            let daily_loss_pct = (-self.daily_pnl / self.initial_capital).abs();
            if daily_loss_pct > self.config.max_daily_loss_pct {
                return Err(SafetyViolation::MaxDailyLossExceeded {
                    loss: daily_loss_pct * 100.0,
                    limit: self.config.max_daily_loss_pct * 100.0,
                });
            }
        }
        
        // Check position size
        let position_pct = position_value / current_capital;
        if position_pct > self.config.max_position_size_pct {
            return Err(SafetyViolation::MaxPositionSizeExceeded {
                position_pct: position_pct * 100.0,
                limit: self.config.max_position_size_pct * 100.0,
            });
        }
        
        // Check confidence
        if confidence < self.config.min_confidence {
            return Err(SafetyViolation::InsufficientConfidence {
                confidence: confidence * 100.0,
                required: self.config.min_confidence * 100.0,
            });
        }
        
        // Check trade frequency
        if self.daily_trades >= self.config.max_trades_per_day {
            return Err(SafetyViolation::MaxTradesPerDayExceeded {
                trades: self.daily_trades,
                limit: self.config.max_trades_per_day,
            });
        }
        
        if self.hourly_trades >= self.config.max_trades_per_hour {
            return Err(SafetyViolation::MaxTradesPerHourExceeded {
                trades: self.hourly_trades,
                limit: self.config.max_trades_per_hour,
            });
        }
        
        // Check if confirmation required
        if position_pct > self.config.require_confirmation_above_pct {
            return Err(SafetyViolation::ConfirmationRequired {
                position_pct: position_pct * 100.0,
                threshold: self.config.require_confirmation_above_pct * 100.0,
            });
        }
        
        Ok(())
    }
    
    /// Record a trade execution
    pub fn record_trade(&mut self, pnl: f64) {
        let now = Utc::now().timestamp();
        self.reset_counters_if_needed(now);
        
        self.daily_trades += 1;
        self.hourly_trades += 1;
        self.daily_pnl += pnl;
        self.last_trade_time = now;
    }
    
    /// Reset counters if needed (hourly/daily)
    fn reset_counters_if_needed(&mut self, now: i64) {
        // Reset hourly counter
        if now - self.last_hour_reset >= 3600 {
            self.hourly_trades = 0;
            self.last_hour_reset = now;
        }
        
        // Reset daily counter
        if now - self.last_day_reset >= 86400 {
            self.daily_trades = 0;
            self.daily_pnl = 0.0;
            self.last_day_reset = now;
        }
    }
    
    /// Get current safety status
    pub fn get_status(&self) -> SafetyStatus {
        let drawdown = if self.peak_capital > 0.0 {
            (self.peak_capital - self.current_capital) / self.peak_capital
        } else {
            0.0
        };
        
        SafetyStatus {
            emergency_stop: self.emergency_stop,
            current_capital: self.current_capital,
            peak_capital: self.peak_capital,
            drawdown_pct: drawdown * 100.0,
            daily_pnl: self.daily_pnl,
            daily_trades: self.daily_trades,
            hourly_trades: self.hourly_trades,
            can_trade: !self.emergency_stop && drawdown < self.config.max_drawdown_pct,
        }
    }
    
    /// Manually trigger emergency stop
    pub fn trigger_emergency_stop(&mut self) {
        self.emergency_stop = true;
        log::error!("🛑 EMERGENCY STOP TRIGGERED - All trading halted");
    }
    
    /// Clear emergency stop (requires manual intervention)
    pub fn clear_emergency_stop(&mut self) {
        self.emergency_stop = false;
        log::warn!("✅ Emergency stop cleared - Trading can resume");
    }
}

/// Safety violation types
#[derive(Debug, Clone, Serialize)]
pub enum SafetyViolation {
    EmergencyStop,
    MaxDrawdownExceeded { drawdown: f64, limit: f64 },
    MaxDailyLossExceeded { loss: f64, limit: f64 },
    MaxPositionSizeExceeded { position_pct: f64, limit: f64 },
    InsufficientConfidence { confidence: f64, required: f64 },
    MaxTradesPerDayExceeded { trades: usize, limit: usize },
    MaxTradesPerHourExceeded { trades: usize, limit: usize },
    ConfirmationRequired { position_pct: f64, threshold: f64 },
}

impl std::fmt::Display for SafetyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SafetyViolation::EmergencyStop => write!(f, "Emergency stop is active"),
            SafetyViolation::MaxDrawdownExceeded { drawdown, limit } => {
                write!(f, "Max drawdown exceeded: {:.2}% (limit: {:.2}%)", drawdown, limit)
            }
            SafetyViolation::MaxDailyLossExceeded { loss, limit } => {
                write!(f, "Max daily loss exceeded: {:.2}% (limit: {:.2}%)", loss, limit)
            }
            SafetyViolation::MaxPositionSizeExceeded { position_pct, limit } => {
                write!(f, "Max position size exceeded: {:.2}% (limit: {:.2}%)", position_pct, limit)
            }
            SafetyViolation::InsufficientConfidence { confidence, required } => {
                write!(f, "Insufficient confidence: {:.2}% (required: {:.2}%)", confidence, required)
            }
            SafetyViolation::MaxTradesPerDayExceeded { trades, limit } => {
                write!(f, "Max trades per day exceeded: {} (limit: {})", trades, limit)
            }
            SafetyViolation::MaxTradesPerHourExceeded { trades, limit } => {
                write!(f, "Max trades per hour exceeded: {} (limit: {})", trades, limit)
            }
            SafetyViolation::ConfirmationRequired { position_pct, threshold } => {
                write!(f, "Confirmation required for position: {:.2}% (threshold: {:.2}%)", position_pct, threshold)
            }
        }
    }
}

/// Current safety status
#[derive(Debug, Clone, Serialize)]
pub struct SafetyStatus {
    pub emergency_stop: bool,
    pub current_capital: f64,
    pub peak_capital: f64,
    pub drawdown_pct: f64,
    pub daily_pnl: f64,
    pub daily_trades: usize,
    pub hourly_trades: usize,
    pub can_trade: bool,
}

