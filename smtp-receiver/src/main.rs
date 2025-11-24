mod config;
mod email_handler;
mod smtp_server;

use anyhow::Result;
use config::Config;
use smtp_server::SmtpServer;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();

    let config = if args.len() > 1 && args[1] == "--generate-config" {
        let config_path = args.get(2).map(|s| s.as_str()).unwrap_or("config.toml");
        Config::save_default(config_path)?;
        println!("Generated default config at: {}", config_path);
        return Ok(());
    } else if args.len() > 1 {
        Config::from_file(&args[1])?
    } else {
        println!("Using default configuration");
        println!("To generate a config file, run: {} --generate-config [path]", args[0]);
        Config::default()
    };

    println!("\nSMTP Receiver Configuration:");
    println!("  Bind: {}:{}", config.smtp.bind_address, config.smtp.port);
    println!("  Max message size: {} MB", config.smtp.max_message_size / (1024 * 1024));
    println!("  Email storage: {}", config.storage.email_dir.display());
    println!("  Proof storage: {}", config.storage.proof_dir.display());
    println!("  Auto-verify: {}", config.processing.auto_verify);
    if !config.smtp.allowed_domains.is_empty() {
        println!("  Allowed domains: {:?}", config.smtp.allowed_domains);
    } else {
        println!("  Allowed domains: ALL (no restrictions)");
    }
    println!();

    let server = SmtpServer::new(config).await?;
    server.run()?;

    Ok(())
}
