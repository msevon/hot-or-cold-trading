use crate::config::TradingConfig;
use anyhow::Result;
use log::{info, warn, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WeatherResponse {
    daily: DailyData,
}

#[derive(Debug, Serialize, Deserialize)]
struct DailyData {
    #[serde(rename = "temperature_2m_max")]
    temperature_2m_max: Vec<f64>,
    #[serde(rename = "temperature_2m_min")]
    temperature_2m_min: Vec<f64>,
}

pub struct WeatherDataFetcher {
    config: TradingConfig,
}

impl WeatherDataFetcher {
    pub fn new(config: TradingConfig) -> Self {
        Self { config }
    }
    
    pub async fn fetch_weather_forecast(&self, region: &str, days: i32) -> Result<WeatherResponse> {
        info!("  Fetching weather forecast for region: {} ({} days)", region, days);
        let parts: Vec<&str> = region.split(',').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid region format"));
        }
        
        let lat = parts[0];
        let lon = parts[1];
        
        let client = reqwest::Client::new();
        let url = &self.config.weather_api_url;
        
        let params = [
            ("latitude", lat),
            ("longitude", lon),
            ("daily", "temperature_2m_max,temperature_2m_min"),
            ("timezone", "America/New_York"),
            ("forecast_days", &days.to_string()),
        ];
        
        info!("    URL: {}", url);
        info!("    Coordinates: lat={}, lon={}", lat, lon);
        
        let response = client
            .get(url)
            .query(&params)
            .send()
            .await?;
        
        info!("    Response status: {}", response.status());
        let weather_data: WeatherResponse = response.json().await?;
        info!("    Successfully fetched weather data");
        Ok(weather_data)
    }
    
    pub fn calculate_hdd(&self, temp_max: f64, temp_min: f64, base_temp: f64) -> f64 {
        let avg_temp = (temp_max + temp_min) / 2.0;
        (base_temp - avg_temp).max(0.0)
    }
    
    pub async fn get_regional_hdd_signal(&self) -> f64 {
        info!("Calculating regional HDD signal from {} regions...", self.config.weather_regions.len());
        let mut total_hdd = 0.0;
        let mut valid_regions = 0;
        
        for (idx, region) in self.config.weather_regions.iter().enumerate() {
            info!("  Processing region {}/{}: {}", idx + 1, self.config.weather_regions.len(), region);
            match self.fetch_weather_forecast(region, 7).await {
                Ok(weather_data) => {
                    let daily_data = &weather_data.daily;
                    let temps_max = &daily_data.temperature_2m_max;
                    let temps_min = &daily_data.temperature_2m_min;
                    
                    let mut region_hdd = 0.0;
                    for (temp_max, temp_min) in temps_max.iter().zip(temps_min.iter()) {
                        region_hdd += self.calculate_hdd(*temp_max, *temp_min, 65.0);
                    }
                    
                    total_hdd += region_hdd;
                    valid_regions += 1;
                    
                    info!("Region {}: HDD = {:.2}", region, region_hdd);
                }
                Err(e) => {
                    error!("Error fetching weather data for {}: {}", region, e);
                }
            }
        }
        
        if valid_regions == 0 {
            warn!("No valid weather data received");
            return 0.0;
        }
        
        let avg_hdd = total_hdd / valid_regions as f64;
        
        // Historical average HDD for comparison
        let historical_avg_hdd = 25.0;
        
        // Calculate signal: positive if colder than average
        let hdd_signal = (avg_hdd - historical_avg_hdd) / historical_avg_hdd;
        
        info!("Average HDD: {:.2}, Signal: {:.3}", avg_hdd, hdd_signal);
        
        hdd_signal
    }
}

