use anyhow::Result;
use clap::Parser;
use rtl_sdr_gnss::{config::Config, receiver::GnssReceiver, ui::UI};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// RTL-SDR device index
    #[arg(short, long, default_value_t = 0)]
    device: u32,

    /// Enable debug logging
    #[arg(short, long)]
    verbose: bool,

    /// Disable UI (output only to stdout)
    #[arg(long)]
    no_ui: bool,

    /// NMEA output file (or stdout if not specified)
    #[arg(long)]
    nmea_output: Option<String>,

    /// JSON output file
    #[arg(long)]
    json_output: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.verbose { "debug" } else { "info" };

    fmt()
        .with_env_filter(EnvFilter::new(log_level))
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("RTL-SDR GNSS Receiver starting...");

    // Load configuration
    let config = Config::load(&args.config).unwrap_or_else(|_| {
        tracing::warn!("Could not load config file, using defaults");
        Config::default()
    });

    // Create GNSS receiver
    let mut receiver = GnssReceiver::new(args.device, config)?;

    // Start the receiver
    receiver.start().await?;

    if args.no_ui {
        // Run without UI, just output to files/stdout
        receiver.run_headless().await?;
    } else {
        // Run with TUI
        let mut ui = UI::new()?;
        ui.run(&mut receiver).await?;
    }

    Ok(())
}
