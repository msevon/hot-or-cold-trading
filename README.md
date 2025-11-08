# NATGAS TRADER

A Rust CLI trading bot for natural gas 2x leverage ETFs (BOIL/KOLD) using weather data (HDD), EIA storage data, and NOAA storm alerts.

## Features

- Weather analysis (heating degree days), EIA storage data, NOAA storm alerts
- Weighted signal processing with configurable thresholds
- Paper trading via Alpaca API
- Comprehensive logging (signals, trades, portfolio)

## Prerequisites

- Rust 1.70+
- Alpaca API credentials (paper trading)
- Optional: EIA API key

## Setup

1. Create `.env` file:
   ```bash
   ALPACA_API_KEY=your_api_key_here
   ALPACA_SECRET_KEY=your_secret_key_here
   ALPACA_BASE_URL=https://paper-api.alpaca.markets
   EIA_API_KEY=your_eia_key_here  # Optional
   SYMBOL=BOIL
   INVERSE_SYMBOL=KOLD
   POSITION_SIZE=1000.0
   BUY_THRESHOLD=0.3
   SELL_THRESHOLD=-0.3
   TEMPERATURE_WEIGHT=0.5
   INVENTORY_WEIGHT=0.4
   STORM_WEIGHT=0.1
   ```

2. Build and run:
   ```bash
   cargo build --release
   cargo run --release -- once              # Single run
   cargo run --release -- continuous [24]   # Continuous (default: 24h)
   ```

## Configuration

Environment variables (see `src/config.rs` for defaults):
- `ALPACA_API_KEY`, `ALPACA_SECRET_KEY` (required)
- `EIA_API_KEY` (optional)
- `SYMBOL`, `INVERSE_SYMBOL`, `POSITION_SIZE`
- `BUY_THRESHOLD`, `SELL_THRESHOLD`
- `TEMPERATURE_WEIGHT`, `INVENTORY_WEIGHT`, `STORM_WEIGHT`

## Signal logic

- **Temperature**: Colder → bullish, warmer → bearish (via HDD)
- **Inventory**: Lower storage → bullish, higher → bearish
- **Storm**: Severe weather → bullish (supply disruption)
- **Total**: Weighted combination; above/below thresholds → buy/sell, else hold

## Logging

Logs in `logs/`: `signals.log`, `trades.log`, `portfolio.log`, `errors.log`

## Safety

Paper trading only, error handling, configurable position limits, mutual exclusivity (one position: BOIL or KOLD)

## Building

```bash
cargo build           # Debug
cargo build --release # Release
cargo test            # Tests
```