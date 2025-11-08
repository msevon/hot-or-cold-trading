use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    // Alpaca API Configuration
    pub alpaca_api_key: String,
    pub alpaca_secret_key: String,
    pub alpaca_base_url: String,
    
    // Trading Parameters
    pub symbol: String,
    pub inverse_symbol: String,
    pub position_size: f64,
    pub buy_threshold: f64,
    pub sell_threshold: f64,
    
    // Signal Weights
    pub temperature_weight: f64,
    pub inventory_weight: f64,
    pub storm_weight: f64,
    
    // Weather API Configuration
    pub weather_api_url: String,
    pub weather_regions: Vec<String>,
    
    // EIA API Configuration
    pub eia_api_key: String,
    pub eia_api_url: String,
    
    // NOAA API Configuration
    pub noaa_api_url: String,
    
    // Logging Configuration
    pub log_level: String,
    pub log_file: String,
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            alpaca_api_key: env::var("ALPACA_API_KEY").unwrap_or_default(),
            alpaca_secret_key: env::var("ALPACA_SECRET_KEY").unwrap_or_default(),
            alpaca_base_url: env::var("ALPACA_BASE_URL")
                .unwrap_or_else(|_| "https://paper-api.alpaca.markets".to_string()),
            symbol: env::var("SYMBOL").unwrap_or_else(|_| "BOIL".to_string()),
            inverse_symbol: env::var("INVERSE_SYMBOL").unwrap_or_else(|_| "KOLD".to_string()),
            position_size: env::var("POSITION_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000.0),
            buy_threshold: env::var("BUY_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.3),
            sell_threshold: env::var("SELL_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(-0.3),
            temperature_weight: env::var("TEMPERATURE_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.5),
            inventory_weight: env::var("INVENTORY_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.4),
            storm_weight: env::var("STORM_WEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.1),
            weather_api_url: "https://api.open-meteo.com/v1/forecast".to_string(),
            weather_regions: vec![
                "40.7128,-74.0060".to_string(), // New York
                "41.8781,-87.6298".to_string(), // Chicago
                "42.3601,-71.0589".to_string(), // Boston
                "39.9526,-75.1652".to_string(), // Philadelphia
                "42.3314,-83.0458".to_string(), // Detroit
            ],
            eia_api_key: env::var("EIA_API_KEY").unwrap_or_default(),
            eia_api_url: "https://api.eia.gov/v2/natural-gas/stor/wkly/data/".to_string(),
            noaa_api_url: "https://api.weather.gov/alerts".to_string(),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string()),
            log_file: env::var("LOG_FILE").unwrap_or_else(|_| "trading_bot.log".to_string()),
        }
    }
}

impl TradingConfig {
    pub fn from_env() -> Self {
        // Try to load config.env first (matches Python version), then fall back to .env
        dotenv::from_filename("config.env").ok();
        dotenv::dotenv().ok(); // Also try .env as fallback
        Self::default()
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.alpaca_api_key.is_empty() || self.alpaca_secret_key.is_empty() {
            return Err("Alpaca API credentials not found! Please set ALPACA_API_KEY and ALPACA_SECRET_KEY environment variables".to_string());
        }
        Ok(())
    }
}

