mod config;
mod data_sources;
mod signals;
mod trading;
mod utils;

use clap::{Parser, Subcommand};
use config::TradingConfig;
use data_sources::{WeatherDataFetcher, EIADataFetcher, NOAADataFetcher};
use signals::SignalProcessor;
use trading::AlpacaTrader;
use utils::TradingLogger;
use log::{info, error};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Parser)]
#[command(name = "algotrade")]
#[command(about = "Natural gas trading bot for BOIL/KOLD ETFs")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single trading cycle
    Once,
    /// Run continuously with specified interval in hours (default: 24)
    Continuous {
        #[arg(default_value = "24")]
        interval_hours: u64,
    },
}

struct NatGasTraderBot {
    _config: TradingConfig,
    logger: TradingLogger,
    weather_fetcher: WeatherDataFetcher,
    eia_fetcher: EIADataFetcher,
    noaa_fetcher: NOAADataFetcher,
    signal_processor: SignalProcessor,
    trader: AlpacaTrader,
}

impl NatGasTraderBot {
    async fn new(config: TradingConfig) -> anyhow::Result<Self> {
        let logger = TradingLogger::new(config.clone());
        let weather_fetcher = WeatherDataFetcher::new(config.clone());
        let eia_fetcher = EIADataFetcher::new(config.clone());
        let noaa_fetcher = NOAADataFetcher::new(config.clone());
        let signal_processor = SignalProcessor::new(config.clone());
        let trader = AlpacaTrader::new(config.clone())?;
        
        // Verify connection
        match trader.get_account_info().await {
            Ok(account) => {
                info!("Connected to Alpaca. Account status: {}", account.equity);
                info!("Buying power: ${:.2}", account.buying_power);
            }
            Err(e) => {
                error!("Failed to connect to Alpaca API: {}", e);
                return Err(e);
            }
        }
        
        info!("NATGAS TRADER Bot initialized");
        
        Ok(Self {
            _config: config,
            logger,
            weather_fetcher,
            eia_fetcher,
            noaa_fetcher,
            signal_processor,
            trader,
        })
    }
    
    async fn fetch_all_signals(&self) -> (f64, f64, f64) {
        info!("");
        info!(">>> Starting signal fetch process <<<");
        info!("");
        
        info!("[1/3] Fetching temperature signal from weather data...");
        let temp_signal = self.weather_fetcher.get_regional_hdd_signal().await;
        info!("[1/3] Temperature signal: {:.4}", temp_signal);
        
        info!("[2/3] Fetching inventory signal from EIA data...");
        let inventory_signal = self.eia_fetcher.calculate_inventory_signal().await;
        info!("[2/3] Inventory signal: {:.4}", inventory_signal);
        
        info!("[3/3] Fetching storm signal from NOAA data...");
        let storm_signal = self.noaa_fetcher.calculate_storm_signal().await;
        info!("[3/3] Storm signal: {:.4}", storm_signal);
        
        info!("");
        info!(">>> Signal fetch complete <<<");
        info!("  Temperature: {:.4}", temp_signal);
        info!("  Inventory: {:.4}", inventory_signal);
        info!("  Storm: {:.4}", storm_signal);
        info!("");
        
        (temp_signal, inventory_signal, storm_signal)
    }
    
    async fn run_trading_cycle(&self) -> bool {
        info!("");
        info!("{}", "=".repeat(60));
        info!("STARTING TRADING CYCLE");
        info!("Time: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        info!("{}", "=".repeat(60));
        
        match self.fetch_all_signals().await {
            (temp_signal, inventory_signal, storm_signal) => {
                info!("");
                info!(">>> Processing signals and generating trading signal <<<");
                let trading_signal = self.signal_processor.create_trading_signal(
                    temp_signal,
                    inventory_signal,
                    storm_signal,
                );
                
                info!("");
                info!(">>> Trading signal generated <<<");
                self.logger.log_signal(&trading_signal);
                
                info!("");
                info!(">>> Executing trade based on signal <<<");
                info!("  Action: {}", trading_signal.action);
                info!("  Symbol: {}", trading_signal.symbol);
                info!("  Confidence: {:.2}", trading_signal.confidence);
                let trade_result = self.trader.execute_trade(&trading_signal).await;
                self.logger.log_trade(trade_result.as_ref());
                
                info!("");
                info!(">>> Fetching portfolio summary <<<");
                match self.trader.get_portfolio_summary().await {
                    Ok(portfolio) => {
                        self.logger.log_portfolio(&portfolio);
                    }
                    Err(e) => {
                        error!("Error getting portfolio summary: {}", e);
                    }
                }
                
                info!("");
                info!("{}", "=".repeat(60));
                info!("TRADING CYCLE COMPLETED SUCCESSFULLY");
                info!("{}", "=".repeat(60));
                info!("");
                true
            }
        }
    }
    
    async fn run_continuous(&self, interval_hours: u64) {
        info!("Starting continuous trading with {}h intervals", interval_hours);
        
        loop {
            match self.run_trading_cycle().await {
                true => {
                    let sleep_seconds = interval_hours * 3600;
                    info!("Waiting {} hours until next cycle", interval_hours);
                    sleep(Duration::from_secs(sleep_seconds)).await;
                }
                false => {
                    info!("Trading cycle failed, waiting 5 minutes before retry");
                    sleep(Duration::from_secs(300)).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger with more verbose output
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_secs()
        .init();
    
    info!("{}", "=".repeat(60));
    info!("NATGAS TRADER BOT - Starting up");
    info!("{}", "=".repeat(60));
    
    // Load configuration
    info!("Loading configuration from environment...");
    let config = TradingConfig::from_env();
    info!("Configuration loaded successfully");
    info!("  Symbol: {}", config.symbol);
    info!("  Inverse Symbol: {}", config.inverse_symbol);
    info!("  Buy Threshold: {}", config.buy_threshold);
    info!("  Sell Threshold: {}", config.sell_threshold);
    
    // Validate configuration
    info!("Validating configuration...");
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        eprintln!("ERROR: {}", e);
        eprintln!("Please set ALPACA_API_KEY and ALPACA_SECRET_KEY environment variables");
        eprintln!("Or check your .env file");
        std::process::exit(1);
    }
    info!("Configuration validated successfully");
    
    // Create logs directory
    info!("Creating logs directory...");
    std::fs::create_dir_all("logs")?;
    info!("Logs directory ready");
    
    // Create and run bot
    info!("Initializing trading bot...");
    let bot = NatGasTraderBot::new(config).await?;
    info!("Trading bot initialized successfully");
    
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Once) => {
            info!("Running in ONCE mode - single trading cycle");
            bot.run_trading_cycle().await;
            info!("Program completed");
        }
        Some(Commands::Continuous { interval_hours }) => {
            info!("Starting continuous trading mode (every {} hours)", interval_hours);
            info!("Press Ctrl+C to stop the bot");
            println!("Starting continuous trading mode (every {} hours)", interval_hours);
            println!("Press Ctrl+C to stop the bot");
            bot.run_continuous(interval_hours).await;
        }
        None => {
            // Default: run continuously (once per day)
            info!("Starting continuous trading mode (once per day)");
            info!("Press Ctrl+C to stop the bot");
            println!("Starting continuous trading mode (once per day)");
            println!("Press Ctrl+C to stop the bot");
            bot.run_continuous(24).await;
        }
    }
    
    Ok(())
}

