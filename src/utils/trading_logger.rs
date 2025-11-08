use crate::config::TradingConfig;
use crate::signals::TradingSignal;
use chrono::Utc;
use log::{info, error};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;

pub struct TradingLogger {
    _config: TradingConfig,
}

impl TradingLogger {
    pub fn new(config: TradingConfig) -> Self {
        Self { _config: config }
    }
    
    pub fn log_signal(&self, signal: &TradingSignal) {
        let signal_data = serde_json::json!({
            "timestamp": signal.timestamp.to_rfc3339(),
            "temperature_signal": signal.temperature_signal,
            "inventory_signal": signal.inventory_signal,
            "storm_signal": signal.storm_signal,
            "total_signal": signal.total_signal,
            "action": signal.action,
            "confidence": signal.confidence,
        });
        
        info!("TRADING SIGNAL: {}", serde_json::to_string_pretty(&signal_data).unwrap());
        
        // Save to separate signal log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/signals.log")
        {
            if let Err(e) = writeln!(file, "{}", serde_json::to_string(&signal_data).unwrap()) {
                error!("Error writing to signals.log: {}", e);
            }
        }
    }
    
    pub fn log_trade(&self, trade_result: Option<&impl Serialize>) {
        if let Some(trade) = trade_result {
            let trade_data = serde_json::json!({
                "timestamp": Utc::now().to_rfc3339(),
                "trade": trade,
            });
            
            info!("TRADE EXECUTED: {}", serde_json::to_string_pretty(&trade_data).unwrap());
            
            // Save to separate trade log file
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/trades.log")
            {
                if let Err(e) = writeln!(file, "{}", serde_json::to_string(&trade_data).unwrap()) {
                    error!("Error writing to trades.log: {}", e);
                }
            }
        } else {
            info!("No trade executed");
        }
    }
    
    pub fn log_portfolio(&self, portfolio: &impl Serialize) {
        let portfolio_data = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "portfolio": portfolio,
        });
        
        info!("PORTFOLIO STATUS: {}", serde_json::to_string_pretty(&portfolio_data).unwrap());
        
        // Save to separate portfolio log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/portfolio.log")
        {
            if let Err(e) = writeln!(file, "{}", serde_json::to_string(&portfolio_data).unwrap()) {
                error!("Error writing to portfolio.log: {}", e);
            }
        }
    }
    
    #[allow(dead_code)]
    pub fn log_error(&self, err: &anyhow::Error, context: &str) {
        let error_data = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "error_type": err.to_string(),
            "error_message": err.to_string(),
            "context": context,
        });
        
        error!("ERROR: {}", serde_json::to_string_pretty(&error_data).unwrap());
        
        // Save to separate error log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/errors.log")
        {
            if let Err(e) = writeln!(file, "{}", serde_json::to_string(&error_data).unwrap()) {
                eprintln!("Error writing to errors.log: {}", e);
            }
        }
    }
}

