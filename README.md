# NATGAS TRADER (Rust Version)

A Rust command-line trading bot that trades natural gas ETFs (BOIL/KOLD) using multiple data sources including weather APIs (for heating degree days), EIA (for US natural gas storage data), and NOAA (for storm alerts).

## Features

- **Weather analysis**: Fetches daily weather forecasts and calculates heating degree days (HDD) for key US regions
- **Storage data**: Retrieves weekly US natural gas storage data from EIA API
- **Storm alerts**: Monitors NOAA weather alerts for supply disruption signals
- **Signal processing**: Combines all signals with configurable weights
- **Paper trading**: Executes trades through Alpaca API in paper trading mode
- **Comprehensive logging**: Logs all signals, trades, and portfolio status
- **Command-line interface**: Simple CLI with subcommands for different modes

## Prerequisites

- Rust 1.70+ (with Cargo)
- Alpaca API credentials (paper trading account)
- Optional: EIA API key for storage data

## Setup

1. **Clone and navigate to the Rust version**:
   ```bash
   cd rust_version
   ```

2. **Create a `.env` file** (or set environment variables):
   
   You can copy the `.env` file from the Python version (`../python_version/config/config.env`) 
   or create a new one with:
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
   LOG_LEVEL=INFO
   LOG_FILE=trading_bot.log
   ```

3. **Build the project**:
   ```bash
   cargo build --release
   ```

4. **Run the bot**:
   ```bash
   # Run once
   cargo run --release -- once
   
   # Run continuously (default: every 24 hours)
   cargo run --release -- continuous 24
   
   # Run continuously with custom interval (e.g., every 12 hours)
   cargo run --release -- continuous 12
   ```

## Usage

The bot supports two main modes:

### Single Run Mode
```bash
cargo run --release -- once
```
Runs a single trading cycle and exits.

### Continuous Mode
```bash
cargo run --release -- continuous [interval_hours]
```
Runs continuously with the specified interval between cycles (default: 24 hours).

If no command is specified, the bot defaults to continuous mode with 24-hour intervals.

## Configuration

The bot can be configured through environment variables or by modifying `src/config.rs`:

- `ALPACA_API_KEY`: Your Alpaca API key (required)
- `ALPACA_SECRET_KEY`: Your Alpaca secret key (required)
- `ALPACA_BASE_URL`: Alpaca API base URL (default: `https://paper-api.alpaca.markets`)
- `EIA_API_KEY`: EIA API key for storage data (optional)
- `SYMBOL`: Bullish ETF symbol to trade (default: BOIL)
- `INVERSE_SYMBOL`: Bearish ETF symbol to trade (default: KOLD)
- `POSITION_SIZE`: Dollar amount per trade (default: $1000)
- `BUY_THRESHOLD`: Signal threshold to buy (default: 0.3)
- `SELL_THRESHOLD`: Signal threshold to sell (default: -0.3)
- Signal weights: `TEMPERATURE_WEIGHT`, `INVENTORY_WEIGHT`, `STORM_WEIGHT`

## Signal Logic

1. **Temperature signal**: Based on heating degree days (HDD)
   - Colder than average → Bullish signal
   - Warmer than average → Bearish signal

2. **Inventory signal**: Based on natural gas storage levels
   - Lower than average → Bullish signal
   - Higher than average → Bearish signal

3. **Storm signal**: Based on weather alerts
   - Severe weather events → Bullish signal (supply disruption)

4. **Total signal**: Weighted combination of all signals
   - Above threshold → Buy ETF
   - Below threshold → Sell ETF
   - Between thresholds → Hold

## Logging

The bot creates comprehensive logs in the `logs/` directory:
- `trading_bot.log`: Main log file (via env_logger)
- `signals.log`: All trading signals
- `trades.log`: All trade executions
- `portfolio.log`: Portfolio status updates
- `errors.log`: Error logging

## Safety Features

- **Paper trading only**: Uses Alpaca paper trading API
- **Error handling**: Comprehensive error handling for API failures
- **Position limits**: Configurable position sizes
- **Signal validation**: Validates signals before trading
- **Mutual exclusivity**: Only holds one position at a time (BOIL or KOLD)

## Project Structure

```
rust_version/
├── Cargo.toml          # Project dependencies
├── src/
│   ├── main.rs        # Main entry point and CLI
│   ├── config.rs      # Configuration management
│   ├── data_sources/  # Weather, EIA, NOAA data fetchers
│   ├── signals/       # Signal processing
│   ├── trading/       # Alpaca trading integration
│   └── utils/         # Logging utilities
└── README.md          # This file
```

## Differences from Python Version

- **No dashboard**: This is a pure CLI application
- **Async/await**: Uses Tokio for async operations
- **Type safety**: Full Rust type safety and error handling
- **Performance**: Compiled binary for better performance
- **Simpler strategy**: Uses basic mutual exclusivity strategy (no 2-day confirmation or stop losses)

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check for issues
cargo check
```

## Disclaimer

Trading involves risk, past performance does not guarantee future results. Always test algorithmic trading bots thoroughly in paper trading mode before considering live trading.

