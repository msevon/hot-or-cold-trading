use crate::config::TradingConfig;
use anyhow::Result;
use log::{info, warn, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct NOAAResponse {
    features: Vec<Feature>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Feature {
    properties: Properties,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Properties {
    event: Option<String>,
    severity: Option<String>,
    urgency: Option<String>,
    description: Option<String>,
    effective: Option<String>,
    expires: Option<String>,
    #[serde(rename = "areaDesc")]
    area_desc: Option<String>,
    state: Option<String>,
}

pub struct NOAADataFetcher {
    config: TradingConfig,
}

impl NOAADataFetcher {
    pub fn new(config: TradingConfig) -> Self {
        Self { config }
    }
    
    pub async fn fetch_weather_alerts(&self) -> Result<Vec<Properties>> {
        info!("Fetching weather alerts from NOAA API...");
        info!("  URL: {}", self.config.noaa_api_url);
        
        let client = reqwest::Client::builder()
            .user_agent("algotrade/1.0 (contact: your-email@example.com)")
            .build()?;
        let url = &self.config.noaa_api_url;
        
        let params = [
            ("active", "true"),
            ("status", "actual"),
            ("message_type", "alert"),
        ];
        
        info!("  Parameters: active=true, status=actual, message_type=alert");
        
        match client.get(url).query(&params).send().await {
            Ok(response) => {
                let status = response.status();
                info!("  Response status: {}", status);
                if !status.is_success() {
                    error!("  NOAA API returned error status: {}", status);
                    return Err(anyhow::anyhow!("NOAA API returned status: {}", status));
                }
                let text = response.text().await?;
                info!("  Response received, size: {} bytes", text.len());
                if text.trim().is_empty() {
                    warn!("  NOAA API returned empty response");
                    return Ok(Vec::new());
                }
                match serde_json::from_str::<NOAAResponse>(&text) {
                    Ok(data) => {
                        info!("  Successfully parsed NOAA response");
                        info!("  Total features in response: {}", data.features.len());
                        let mut alerts = Vec::new();
                        
                        for feature in data.features {
                            let event_type = feature.properties.event.as_ref()
                                .map(|s| s.to_lowercase())
                                .unwrap_or_default();
                            
                            if event_type.contains("storm")
                                || event_type.contains("winter")
                                || event_type.contains("blizzard")
                                || event_type.contains("ice")
                                || event_type.contains("freeze")
                                || event_type.contains("hurricane")
                                || event_type.contains("tornado")
                                || event_type.contains("severe")
                            {
                                info!("  Found relevant alert: {}", feature.properties.event.as_deref().unwrap_or("Unknown"));
                                alerts.push(feature.properties);
                            }
                        }
                        
                        info!("  Total relevant alerts found: {}", alerts.len());
                        Ok(alerts)
                    }
                    Err(e) => {
                        error!("  Error parsing NOAA response: {}", e);
                        Err(anyhow::anyhow!("Error parsing NOAA response: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(anyhow::anyhow!("Error fetching NOAA alerts: {}", e))
            }
        }
    }
    
    pub async fn calculate_storm_signal(&self) -> f64 {
        info!("Calculating storm signal from NOAA alerts...");
        match self.fetch_weather_alerts().await {
            Ok(alerts) => {
                if alerts.is_empty() {
                    info!("No relevant weather alerts found - storm signal: 0.0");
                    return 0.0;
                }
                
                info!("Processing {} weather alerts...", alerts.len());
                let mut storm_signal: f64 = 0.0;
                
                for alert in alerts {
                    let event = alert.event.as_ref()
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();
                    let severity = alert.severity.as_ref()
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();
                    
                    // Base signal strength based on event type
                    let base_signal = if event.contains("winter") || event.contains("blizzard") {
                        0.3
                    } else if event.contains("storm") {
                        0.2
                    } else if event.contains("severe") {
                        0.15
                    } else {
                        0.1
                    };
                    
                    // Adjust based on severity
                    let multiplier = if severity == "extreme" {
                        1.5
                    } else if severity == "severe" {
                        1.2
                    } else if severity == "moderate" {
                        1.0
                    } else {
                        0.8
                    };
                    
                    storm_signal += base_signal * multiplier;
                    
                    info!(
                        "Alert: {} ({}) - Signal: {:.3}",
                        alert.event.as_deref().unwrap_or("Unknown"),
                        severity,
                        base_signal * multiplier
                    );
                }
                
                // Cap the signal at 1.0
                storm_signal = storm_signal.min(1.0);
                
                info!("Total storm signal: {:.3}", storm_signal);
                
                storm_signal
            }
            Err(e) => {
                error!("Error calculating storm signal: {}", e);
                // Return 0.0 instead of mock data when API fails
                0.0
            }
        }
    }
}

