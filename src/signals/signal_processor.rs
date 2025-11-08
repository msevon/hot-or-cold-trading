use crate::config::TradingConfig;
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub timestamp: DateTime<Utc>,
    pub temperature_signal: f64,
    pub inventory_signal: f64,
    pub storm_signal: f64,
    pub total_signal: f64,
    pub action: String, // "BUY", "SELL", "HOLD"
    pub symbol: String, // "BOIL" or "KOLD"
    pub confidence: f64,
}

pub struct SignalProcessor {
    config: TradingConfig,
}

impl SignalProcessor {
    pub fn new(config: TradingConfig) -> Self {
        Self { config }
    }
    
    pub fn calculate_total_signal(
        &self,
        temp_signal: f64,
        inventory_signal: f64,
        storm_signal: f64,
    ) -> f64 {
        let total_signal = temp_signal * self.config.temperature_weight
            + inventory_signal * self.config.inventory_weight
            + storm_signal * self.config.storm_weight;
        
        info!("Signal components:");
        info!(
            "  Temperature: {:.3} (weight: {})",
            temp_signal, self.config.temperature_weight
        );
        info!(
            "  Inventory: {:.3} (weight: {})",
            inventory_signal, self.config.inventory_weight
        );
        info!(
            "  Storm: {:.3} (weight: {})",
            storm_signal, self.config.storm_weight
        );
        info!("  Total signal: {:.3}", total_signal);
        
        total_signal
    }
    
    pub fn determine_action(&self, total_signal: f64) -> (String, String, f64) {
        info!("Determining trading action...");
        info!("  Total signal: {:.4}", total_signal);
        info!("  Buy threshold: {}", self.config.buy_threshold);
        info!("  Sell threshold: {}", self.config.sell_threshold);
        
        if total_signal > self.config.buy_threshold {
            let action = "BUY".to_string();
            let symbol = self.config.symbol.clone(); // BOIL for bullish natural gas
            let confidence = (total_signal / self.config.buy_threshold).min(2.0);
            info!("  Decision: {} {} (confidence: {:.2})", action, symbol, confidence);
            (action, symbol, confidence)
        } else if total_signal < self.config.sell_threshold {
            let action = "BUY".to_string();
            let symbol = self.config.inverse_symbol.clone(); // KOLD for bearish natural gas
            let confidence = (total_signal.abs() / self.config.sell_threshold.abs()).min(2.0);
            info!("  Decision: {} {} (confidence: {:.2})", action, symbol, confidence);
            (action, symbol, confidence)
        } else {
            let action = "HOLD".to_string();
            let symbol = String::new();
            let confidence = 0.0;
            info!("  Decision: {} (signal between thresholds)", action);
            (action, symbol, confidence)
        }
    }
    
    pub fn create_trading_signal(
        &self,
        temp_signal: f64,
        inventory_signal: f64,
        storm_signal: f64,
    ) -> TradingSignal {
        let total_signal = self.calculate_total_signal(temp_signal, inventory_signal, storm_signal);
        let (action, symbol, confidence) = self.determine_action(total_signal);
        
        TradingSignal {
            timestamp: Utc::now(),
            temperature_signal: temp_signal,
            inventory_signal,
            storm_signal,
            total_signal,
            action,
            symbol,
            confidence,
        }
    }
}

