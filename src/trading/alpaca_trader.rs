use crate::config::TradingConfig;
use crate::signals::TradingSignal;
use anyhow::Result;
use log::{info, error, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AlpacaAccount {
    status: String,
    buying_power: String,
    equity: String,
    cash: String,
    portfolio_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AlpacaPosition {
    symbol: String,
    qty: String,
    market_value: String,
    avg_entry_price: String,
    unrealized_pl: String,
    unrealized_plpc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlpacaOrder {
    #[serde(default)]
    id: String,
    symbol: String,
    qty: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    status: String,
    #[serde(default)]
    filled_qty: Option<String>,
    #[serde(default)]
    filled_avg_price: Option<String>,
    #[serde(default)]
    submitted_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub qty: f64,
    pub market_value: f64,
    pub avg_entry_price: f64,
    pub unrealized_pl: f64,
    pub unrealized_plpc: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub equity: f64,
    pub buying_power: f64,
    pub cash: f64,
    pub portfolio_value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeResult {
    pub order_id: String,
    pub symbol: String,
    pub qty: i32,
    pub side: String,
    pub status: String,
    pub filled_qty: Option<i32>,
    pub filled_avg_price: Option<f64>,
    pub submitted_at: String,
}

pub struct AlpacaTrader {
    config: TradingConfig,
    client: reqwest::Client,
    base_url: String,
}

impl AlpacaTrader {
    pub fn new(config: TradingConfig) -> Result<Self> {
        let client = reqwest::Client::new();
        let base_url = config.alpaca_base_url.clone();
        
        let trader = Self {
            config,
            client,
            base_url,
        };
        
        Ok(trader)
    }
    
    pub async fn get_account_info(&self) -> Result<AccountInfo> {
        let url = format!("{}/v2/account", self.base_url);
        
        let request = self.client
            .get(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
        
        let response = request.send().await?;
        let account: AlpacaAccount = response.json().await?;
        
        Ok(AccountInfo {
            equity: account.equity.parse()?,
            buying_power: account.buying_power.parse()?,
            cash: account.cash.parse()?,
            portfolio_value: account.portfolio_value.parse()?,
        })
    }
    
    pub async fn get_current_position(&self, symbol: &str) -> Result<Option<Position>> {
        let url = format!("{}/v2/positions/{}", self.base_url, symbol);
        
        let request = self.client
            .get(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
        
        match request.send().await {
            Ok(response) => {
                if response.status() == 404 {
                    return Ok(None);
                }
                let position: AlpacaPosition = response.json().await?;
                Ok(Some(Position {
                    symbol: position.symbol,
                    qty: position.qty.parse()?,
                    market_value: position.market_value.parse()?,
                    avg_entry_price: position.avg_entry_price.parse()?,
                    unrealized_pl: position.unrealized_pl.parse()?,
                    unrealized_plpc: position.unrealized_plpc.parse()?,
                }))
            }
            Err(e) => {
                if e.to_string().contains("404") {
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("Error getting position: {}", e))
                }
            }
        }
    }
    
    pub async fn get_current_price(&self, symbol: &str) -> Result<f64> {
        // Try the latest bar endpoint first
        let url = format!("{}/v2/stocks/{}/bars/latest", self.base_url, symbol);
        
        let request = self.client
            .get(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
        
        let response = request.send().await?;
        let status = response.status();
        
        if status == 404 {
            // Try alternative endpoint - latest quote
            let quote_url = format!("{}/v2/stocks/{}/quotes/latest", self.base_url, symbol);
            let quote_request = self.client
                .get(&quote_url)
                .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
                .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
            
            let quote_response = quote_request.send().await?;
            let quote_status = quote_response.status();
            if !quote_status.is_success() {
                // Try getting price from position if we have one
                if let Ok(Some(position)) = self.get_current_position(symbol).await {
                    // Calculate price from market value and quantity
                    if position.qty != 0.0 {
                        let price = position.market_value / position.qty;
                        info!("Using position-based price for {}: ${:.2}", symbol, price);
                        return Ok(price);
                    }
                }
                return Err(anyhow::anyhow!("Alpaca API returned status: {} for both bars and quotes, and no position found", quote_status));
            }
            
            let text = quote_response.text().await?;
            let data: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| anyhow::anyhow!("Failed to parse quote JSON: {} - Response: {}", e, &text[..text.len().min(200)]))?;
            
            let quote = data.get("quote").ok_or_else(|| anyhow::anyhow!("No quote data in response"))?;
            let price = quote.get("bp")  // bid price
                .or_else(|| quote.get("ap"))  // ask price
                .or_else(|| quote.get("p"))  // price
                .and_then(|v| v.as_f64())
                .ok_or_else(|| anyhow::anyhow!("No price in quote data"))?;
            
            return Ok(price);
        }
        
        if !status.is_success() {
            return Err(anyhow::anyhow!("Alpaca API returned status: {}", status));
        }
        
        let text = response.text().await?;
        if text.is_empty() {
            return Err(anyhow::anyhow!("Empty response from Alpaca API"));
        }
        
        let data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {} - Response: {}", e, &text[..text.len().min(200)]))?;
        
        let bar = data.get("bar").ok_or_else(|| anyhow::anyhow!("No bar data in response"))?;
        let close = bar.get("c")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("No close price in bar data"))?;
        
        Ok(close)
    }
    
    async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<AlpacaOrder>> {
        let mut url = format!("{}/v2/orders?status=open", self.base_url);
        if let Some(sym) = symbol {
            url = format!("{}/v2/orders?status=open&symbols={}", self.base_url, sym);
        }
        
        let request = self.client
            .get(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
        
        let response = request.send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to get orders: {}", error_text));
        }
        
        let orders: Vec<AlpacaOrder> = response.json().await?;
        Ok(orders)
    }
    
    pub async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let url = format!("{}/v2/orders/{}", self.base_url, order_id);
        
        let request = self.client
            .delete(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
        
        let response = request.send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to cancel order {}: {}", order_id, error_text));
        }
        
        Ok(())
    }
    
    pub async fn cancel_opposite_orders(&self, symbol: &str, side: &str) -> Result<()> {
        info!("  Checking for existing orders on {}...", symbol);
        let orders = self.get_open_orders(Some(symbol)).await?;
        
        if orders.is_empty() {
            info!("  No open orders found for {}", symbol);
            return Ok(());
        }
        
        info!("  Found {} open order(s) for {}", orders.len(), symbol);
        let opposite_side = if side == "buy" { "sell" } else { "buy" };
        
        let mut cancelled_count = 0;
        for order in orders {
            if order.side.to_lowercase() == opposite_side {
                info!("  Cancelling opposite {} order: {} (ID: {})", 
                      order.side, order.symbol, order.id);
                match self.cancel_order(&order.id).await {
                    Ok(_) => {
                        info!("  Successfully cancelled order {}", order.id);
                        cancelled_count += 1;
                    }
                    Err(e) => {
                        warn!("  Failed to cancel order {}: {}", order.id, e);
                    }
                }
            } else {
                info!("  Keeping existing {} order: {} (ID: {})", 
                      order.side, order.symbol, order.id);
            }
        }
        
        if cancelled_count > 0 {
            info!("  Cancelled {} opposite order(s), waiting 1 second for cancellation to process...", cancelled_count);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        
        Ok(())
    }
    
    pub async fn place_market_order(&self, side: &str, qty: i32, symbol: &str) -> Result<TradeResult> {
        // Cancel any opposite-side orders first to avoid wash trade errors
        if let Err(e) = self.cancel_opposite_orders(symbol, side).await {
            warn!("  Warning: Could not cancel opposite orders: {}", e);
            // Continue anyway, might not have any orders
        }
        
        info!("Placing {} order for {} shares of {}", side, qty, symbol);
        
        let url = format!("{}/v2/orders", self.base_url);
        
        let order_data = serde_json::json!({
            "symbol": symbol,
            "qty": qty,
            "side": side,
            "type": "market",
            "time_in_force": "day"
        });
        
        let request = self.client
            .post(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key)
            .json(&order_data);
        
        let response = request.send().await?;
        let status_code = response.status();
        if !status_code.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            // If we get a wash trade error, try to cancel opposite orders and retry once
            if status_code.as_u16() == 403 && error_text.contains("wash trade") {
                warn!("  Wash trade detected, attempting to cancel all opposite orders and retry...");
                
                // Cancel all opposite orders for this symbol
                if let Err(e) = self.cancel_opposite_orders(symbol, side).await {
                    warn!("  Failed to cancel opposite orders: {}", e);
                }
                
                // Wait a bit for cancellations to process
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Retry the order
                info!("  Retrying {} order for {} shares of {}...", side, qty, symbol);
                let retry_request = self.client
                    .post(&url)
                    .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
                    .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key)
                    .json(&order_data);
                
                let retry_response = retry_request.send().await?;
                let retry_status = retry_response.status();
                if !retry_status.is_success() {
                    let retry_error = retry_response.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("Alpaca API error after retry ({}): {}", retry_status, &retry_error[..retry_error.len().min(200)]));
                }
                
                // Process the successful retry response
                let retry_text = retry_response.text().await?;
                let order: AlpacaOrder = serde_json::from_str(&retry_text)
                    .map_err(|e| anyhow::anyhow!("Failed to parse order response: {} - Response: {}", e, &retry_text[..retry_text.len().min(200)]))?;
                
                // Wait a bit for order to fill
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Get order status
                let order_status = if !order.id.is_empty() {
                    let status_url = format!("{}/v2/orders/{}", self.base_url, order.id);
                    let status_request = self.client
                        .get(&status_url)
                        .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
                        .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
                    
                    match status_request.send().await {
                        Ok(status_response) => {
                            if status_response.status().is_success() {
                                match status_response.json::<AlpacaOrder>().await {
                                    Ok(status) => status,
                                    Err(_) => order.clone(),
                                }
                            } else {
                                order
                            }
                        }
                        Err(_) => order,
                    }
                } else {
                    order
                };
                
                let result = TradeResult {
                    order_id: order_status.id.clone(),
                    symbol: order_status.symbol.clone(),
                    qty: order_status.qty.parse().unwrap_or(0),
                    side: order_status.side.clone(),
                    status: order_status.status.clone(),
                    filled_qty: order_status.filled_qty.as_ref().and_then(|q| q.parse().ok()),
                    filled_avg_price: order_status.filled_avg_price.as_ref().and_then(|p| p.parse().ok()),
                    submitted_at: order_status.submitted_at.clone(),
                };
                
                info!("Order placed successfully after retry: {:?}", result);
                return Ok(result);
            }
            
            return Err(anyhow::anyhow!("Alpaca API error ({}): {}", status_code, &error_text[..error_text.len().min(200)]));
        }
        
        let text = response.text().await?;
        let order: AlpacaOrder = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse order response: {} - Response: {}", e, &text[..text.len().min(200)]))?;
        
        // Wait a bit for order to fill
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Get order status if we have an ID
        let order_status = if !order.id.is_empty() {
            let status_url = format!("{}/v2/orders/{}", self.base_url, order.id);
            let status_request = self.client
                .get(&status_url)
                .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
                .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
            
            match status_request.send().await {
                Ok(status_response) => {
                    if status_response.status().is_success() {
                        match status_response.json::<AlpacaOrder>().await {
                            Ok(status) => status,
                            Err(_) => order.clone(),
                        }
                    } else {
                        order
                    }
                }
                Err(_) => order,
            }
        } else {
            order
        };
        
        let result = TradeResult {
            order_id: order_status.id.clone(),
            symbol: order_status.symbol.clone(),
            qty: order_status.qty.parse().unwrap_or(0),
            side: order_status.side.clone(),
            status: order_status.status.clone(),
            filled_qty: order_status.filled_qty.as_ref().and_then(|q| q.parse().ok()),
            filled_avg_price: order_status.filled_avg_price.as_ref().and_then(|p| p.parse().ok()),
            submitted_at: order_status.submitted_at.clone(),
        };
        
        info!("Order placed: {:?}", result);
        Ok(result)
    }
    
    pub async fn execute_trade(&self, signal: &TradingSignal) -> Option<TradeResult> {
        info!("");
        info!(">>> EXECUTING TRADE <<<");
        info!("  Signal action: {}", signal.action);
        info!("  Signal symbol: {}", signal.symbol);
        info!("  Signal confidence: {:.2}", signal.confidence);
        info!("  Total signal strength: {:.4}", signal.total_signal);
        
        // Simple strategy: mutual exclusivity
        // If buying BOIL, sell all KOLD first and vice versa
        
        if signal.action != "BUY" {
            info!("  Signal indicates {}, no trade executed", signal.action);
            info!(">>> TRADE EXECUTION SKIPPED <<<");
            return None;
        }
        
        info!("  Checking current positions...");
        let boil_position = self.get_current_position(&self.config.symbol).await.ok().flatten();
        let kold_position = self.get_current_position(&self.config.inverse_symbol).await.ok().flatten();
        
        info!("  Current BOIL position: {:?}", boil_position);
        info!("  Current KOLD position: {:?}", kold_position);
        
        if signal.symbol == self.config.symbol {
            info!("  Strategy: Buying BOIL (bullish natural gas)");
            // Buying BOIL
            // Sell all KOLD first
            if let Some(kold_pos) = kold_position {
                if kold_pos.qty > 0.0 {
                    info!("  Mutual exclusivity: Selling all KOLD positions before buying BOIL");
                    info!("  KOLD position qty: {:.2}", kold_pos.qty);
                    let qty = kold_pos.qty.abs() as i32;
                    if let Err(e) = self.place_market_order("sell", qty, &self.config.inverse_symbol).await {
                        error!("  Error selling KOLD: {}", e);
                    } else {
                        info!("  Successfully sold KOLD position");
                    }
                } else {
                    info!("  No KOLD position to close");
                }
            } else {
                info!("  No existing KOLD position");
            }
            
            // Close existing BOIL position
            if let Some(boil_pos) = boil_position {
                if boil_pos.qty > 0.0 {
                    info!("  Closing existing BOIL position before new purchase");
                    info!("  Existing BOIL qty: {:.2}", boil_pos.qty);
                    let qty = boil_pos.qty.abs() as i32;
                    // Check if position is available (not held for orders)
                    if boil_pos.qty > 0.0 && qty > 0 {
                        match self.place_market_order("sell", qty, &self.config.symbol).await {
                            Ok(_) => info!("  Successfully closed BOIL position"),
                            Err(e) => {
                                // If it's an insufficient qty error, position might already be closing
                                if e.to_string().contains("insufficient qty") {
                                    warn!("  BOIL position already held for orders, skipping close");
                                } else {
                                    error!("  Error closing BOIL: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    info!("  No existing BOIL position to close");
                }
            } else {
                info!("  No existing BOIL position");
            }
            
            // Buy BOIL
            info!("  Fetching current BOIL price...");
            match self.get_current_price(&self.config.symbol).await {
                Ok(price) => {
                    let qty = (self.config.position_size / price).max(1.0) as i32;
                    info!("  Current BOIL price: ${:.2}", price);
                    info!("  Position size: ${:.2}", self.config.position_size);
                    info!("  Calculated quantity: {} shares", qty);
                    info!("  Placing market order to buy {} shares of BOIL...", qty);
                    match self.place_market_order("buy", qty, &self.config.symbol).await {
                        Ok(result) => {
                            info!("  Order placed successfully: {:?}", result);
                            info!(">>> TRADE EXECUTION COMPLETE <<<");
                            Some(result)
                        }
                        Err(e) => {
                            error!("  Failed to place order: {}", e);
                            info!(">>> TRADE EXECUTION FAILED <<<");
                            None
                        }
                    }
                }
                Err(e) => {
                    error!("  Could not get current price for BOIL: {}", e);
                    warn!("  Skipping BOIL purchase due to price lookup failure");
                    info!(">>> TRADE EXECUTION FAILED <<<");
                    None
                }
            }
        } else if signal.symbol == self.config.inverse_symbol {
            info!("  Strategy: Buying KOLD (bearish natural gas)");
            // Buying KOLD
            // Sell all BOIL first
            if let Some(boil_pos) = boil_position {
                if boil_pos.qty > 0.0 {
                    info!("  Mutual exclusivity: Selling all BOIL positions before buying KOLD");
                    info!("  BOIL position qty: {:.2}", boil_pos.qty);
                    let qty = boil_pos.qty.abs() as i32;
                    if let Err(e) = self.place_market_order("sell", qty, &self.config.symbol).await {
                        error!("  Error selling BOIL: {}", e);
                    } else {
                        info!("  Successfully sold BOIL position");
                    }
                } else {
                    info!("  No BOIL position to close");
                }
            } else {
                info!("  No existing BOIL position");
            }
            
            // Close existing KOLD position
            if let Some(kold_pos) = kold_position {
                if kold_pos.qty > 0.0 {
                    info!("  Closing existing KOLD position before new purchase");
                    info!("  Existing KOLD qty: {:.2}", kold_pos.qty);
                    let qty = kold_pos.qty.abs() as i32;
                    if qty > 0 {
                        match self.place_market_order("sell", qty, &self.config.inverse_symbol).await {
                            Ok(_) => info!("  Successfully closed KOLD position"),
                            Err(e) => {
                                if e.to_string().contains("insufficient qty") {
                                    warn!("  KOLD position already held for orders, skipping close");
                                } else {
                                    error!("  Error closing KOLD: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    info!("  No existing KOLD position to close");
                }
            } else {
                info!("  No existing KOLD position");
            }
            
            // Buy KOLD
            info!("  Fetching current KOLD price...");
            match self.get_current_price(&self.config.inverse_symbol).await {
                Ok(price) => {
                    let qty = (self.config.position_size / price).max(1.0) as i32;
                    info!("  Current KOLD price: ${:.2}", price);
                    info!("  Position size: ${:.2}", self.config.position_size);
                    info!("  Calculated quantity: {} shares", qty);
                    info!("  Placing market order to buy {} shares of KOLD...", qty);
                    match self.place_market_order("buy", qty, &self.config.inverse_symbol).await {
                        Ok(result) => {
                            info!("  Order placed successfully: {:?}", result);
                            info!(">>> TRADE EXECUTION COMPLETE <<<");
                            Some(result)
                        }
                        Err(e) => {
                            error!("  Failed to place order: {}", e);
                            info!(">>> TRADE EXECUTION FAILED <<<");
                            None
                        }
                    }
                }
                Err(e) => {
                    error!("  Could not get current price for KOLD: {}", e);
                    warn!("  Skipping KOLD purchase due to price lookup failure");
                    info!(">>> TRADE EXECUTION FAILED <<<");
                    None
                }
            }
        } else {
            warn!("  Unsupported symbol: {}", signal.symbol);
            warn!("  Expected {} or {}", self.config.symbol, self.config.inverse_symbol);
            info!(">>> TRADE EXECUTION SKIPPED - UNSUPPORTED SYMBOL <<<");
            None
        }
    }
    
    pub async fn get_portfolio_summary(&self) -> Result<serde_json::Value> {
        info!("  Fetching portfolio positions from Alpaca...");
        let url = format!("{}/v2/positions", self.base_url);
        
        let request = self.client
            .get(&url)
            .header("APCA-API-KEY-ID", &self.config.alpaca_api_key)
            .header("APCA-API-SECRET-KEY", &self.config.alpaca_secret_key);
        
        let response = request.send().await?;
        info!("  Positions API response status: {}", response.status());
        let positions: Vec<AlpacaPosition> = response.json().await?;
        info!("  Found {} positions", positions.len());
        
        info!("  Fetching account information...");
        let account = self.get_account_info().await?;
        info!("  Account equity: ${:.2}", account.equity);
        info!("  Buying power: ${:.2}", account.buying_power);
        info!("  Cash: ${:.2}", account.cash);
        
        let mut portfolio_positions = Vec::new();
        for position in positions {
            let qty: f64 = position.qty.parse()?;
            let market_value: f64 = position.market_value.parse()?;
            let current_price = if qty != 0.0 { market_value / qty } else { 0.0 };
            
            info!("  Position: {} - Qty: {:.2}, Value: ${:.2}, Price: ${:.2}", 
                  position.symbol, qty, market_value, current_price);
            
            portfolio_positions.push(serde_json::json!({
                "symbol": position.symbol,
                "qty": qty,
                "current_price": current_price,
                "market_value": market_value,
                "unrealized_pl": position.unrealized_pl.parse::<f64>()?,
                "unrealized_plpc": position.unrealized_plpc.parse::<f64>()?,
            }));
        }
        
        let summary = serde_json::json!({
            "total_value": account.portfolio_value,
            "cash": account.cash,
            "buying_power": account.buying_power,
            "positions": portfolio_positions,
        });
        
        info!("  Portfolio summary generated");
        Ok(summary)
    }
}

