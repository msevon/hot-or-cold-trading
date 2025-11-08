use crate::config::TradingConfig;
use anyhow::Result;
use chrono::{DateTime, Utc, Duration, Datelike};
use log::{info, warn, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct EIAResponse {
    response: ResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseData {
    data: Vec<StorageDataPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StorageDataPoint {
    period: String,
    value: serde_json::Value,
}

pub struct EIADataFetcher {
    config: TradingConfig,
}

impl EIADataFetcher {
    pub fn new(config: TradingConfig) -> Self {
        Self { config }
    }
    
    pub async fn fetch_storage_data(&self) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if self.config.eia_api_key.is_empty() {
            return Err(anyhow::anyhow!("EIA API key not provided"));
        }
        
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(365);
        
        let client = reqwest::Client::new();
        let url = &self.config.eia_api_url;
        
        let params = [
            ("api_key", self.config.eia_api_key.as_str()),
            ("data[]", "value"),
            ("start", &start_date.format("%Y-%m-%d").to_string()),
            ("end", &end_date.format("%Y-%m-%d").to_string()),
            ("length", "1000"),
        ];
        
        info!(
            "Fetching EIA data from {} to {}",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );
        
        info!("  Sending request to EIA API...");
        match client.get(url).query(&params).send().await {
            Ok(response) => {
                info!("  Response status: {}", response.status());
                match response.json::<EIAResponse>().await {
                    Ok(data) => {
                        info!("  Successfully parsed EIA response");
                        info!("  Raw data points received: {}", data.response.data.len());
                        let mut storage_data = Vec::new();
                        
                        for point in data.response.data {
                            let value_f64 = match &point.value {
                                serde_json::Value::Number(n) => n.as_f64(),
                                serde_json::Value::String(s) => s.parse().ok(),
                                _ => None,
                            };
                            if let Some(v) = value_f64 {
                                if let Ok(period) = DateTime::parse_from_rfc3339(&point.period) {
                                    let period_utc = period.with_timezone(&Utc);
                                    if period_utc >= start_date {
                                        storage_data.push((period_utc, v));
                                    }
                                }
                            }
                        }
                        
                        storage_data.sort_by_key(|(date, _)| *date);
                        info!("Successfully fetched {} data points from EIA API", storage_data.len());
                        Ok(storage_data)
                    }
                    Err(e) => {
                        Err(anyhow::anyhow!("Error parsing EIA API response: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(anyhow::anyhow!("EIA API failed: {}", e))
            }
        }
    }
    
    pub async fn calculate_inventory_signal(&self) -> f64 {
        match self.fetch_storage_data().await {
            Ok(storage_data) => {
                if storage_data.len() < 2 {
                    warn!("Insufficient storage data");
                    return 0.0;
                }
                
                let current_storage = storage_data.last().unwrap().1;
                let historical_avg: f64 = storage_data.iter().map(|(_, v)| v).sum::<f64>() / storage_data.len() as f64;
                
                // Calculate signal: positive if below average (bullish for prices)
                let inventory_signal = (historical_avg - current_storage) / historical_avg;
                
                info!("Current storage: {:.0} Bcf", current_storage);
                info!("Historical avg: {:.0} Bcf", historical_avg);
                info!("Inventory signal: {:.3}", inventory_signal);
                
                inventory_signal
            }
            Err(e) => {
                error!("Error calculating inventory signal: {}", e);
                // Return 0.0 instead of mock data when API fails
                0.0
            }
        }
    }
    
    #[allow(dead_code)]
    fn get_mock_storage_data(&self) -> Vec<(DateTime<Utc>, f64)> {
        info!("Using mock storage data (EIA API unavailable)");
        
        let mut storage_data = Vec::new();
        let base_storage = 3500.0;
        let mut current_date = Utc::now() - Duration::weeks(52);
        let end_date = Utc::now();
        
        let mut week = 0;
        while current_date <= end_date {
            let month = current_date.month();
            let seasonal_factor = if month == 12 || month == 1 || month == 2 {
                1.1
            } else if month == 6 || month == 7 || month == 8 {
                0.9
            } else {
                1.0
            };
            
            let weekly_factor = 1.0 + ((week % 4) as f64 - 1.5) * 0.05;
            let storage_value = base_storage * seasonal_factor * weekly_factor;
            
            storage_data.push((current_date, storage_value));
            current_date += Duration::weeks(1);
            week += 1;
        }
        
        info!("Generated {} mock storage data points", storage_data.len());
        storage_data
    }
}

