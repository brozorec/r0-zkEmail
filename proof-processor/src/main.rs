mod config;
mod extractor;
mod processor;
mod relayer;

use anyhow::{Context, Result};
use config::Config;
use processor::ReceiptProcessor;
use std::path::PathBuf;
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() >= 2 {
        &args[1]
    } else {
        "config.toml"
    };

    tracing::info!("Loading configuration from: {}", config_path);
    let config = Config::from_file(config_path)
        .context("Failed to load configuration")?;

    config.validate()
        .context("Configuration validation failed")?;

    tracing::info!("Configuration loaded successfully");
    tracing::info!("Network: {}", config.stellar.network);
    tracing::info!("Contract: {}", config.stellar.contract_address);
    tracing::info!("Proof directory: {}", config.processor.proof_dir.display());
    tracing::info!("Scan interval: {} seconds", config.processor.scan_interval_secs);

    // Create necessary directories
    tokio::fs::create_dir_all(&config.processor.proof_dir).await?;
    tokio::fs::create_dir_all(&config.processor.processed_dir).await?;
    tokio::fs::create_dir_all(&config.processor.failed_dir).await?;

    // Initialize processor
    let processor = ReceiptProcessor::new(config.clone());

    // Process any existing receipts on startup
    tracing::info!("Checking for existing receipts...");
    process_existing_receipts(&processor).await?;

    // Start periodic scanning
    tracing::info!("Starting periodic receipt scanner...");
    run_periodic_scanner(processor, config.processor.scan_interval_secs).await
}

/// Process all existing .bin files in the proof directory on startup
async fn process_existing_receipts(processor: &ReceiptProcessor) -> Result<()> {
    let receipts = processor.get_pending_receipts().await?;

    if receipts.is_empty() {
        tracing::info!("No existing receipts found");
        return Ok(());
    }

    tracing::info!("Found {} existing receipt(s) to process", receipts.len());

    for receipt_path in receipts {
        tracing::info!("Processing existing receipt: {}", receipt_path.display());
        processor.process_receipt_safe(&receipt_path).await;
    }

    Ok(())
}

/// Run the periodic scanner that checks for new receipts
async fn run_periodic_scanner(processor: ReceiptProcessor, interval_secs: u64) -> Result<()> {
    let mut ticker = interval(Duration::from_secs(interval_secs));

    loop {
        ticker.tick().await;

        tracing::debug!("Scanning for new receipts...");

        match processor.get_pending_receipts().await {
            Ok(receipts) => {
                if !receipts.is_empty() {
                    tracing::info!("Found {} receipt(s) to process", receipts.len());

                    for receipt_path in receipts {
                        processor.process_receipt_safe(&receipt_path).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to scan for receipts: {}", e);
            }
        }
    }
}
