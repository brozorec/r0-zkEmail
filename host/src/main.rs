use anyhow::{anyhow, Result};
use host::verify_email_pair;
use std::{env, fs::File, io::Read, path::PathBuf};

async fn verify(
    sender_domain: &str,
    sender_path: &PathBuf,
    receiver_domain: &str,
    receiver_path: &PathBuf,
) -> Result<()> {
    let sender_raw = read_email_file(sender_path)?;
    let receiver_raw = read_email_file(receiver_path)?;

    let output = verify_email_pair(sender_domain, &sender_raw, receiver_domain, &receiver_raw).await?;

    println!("\n=== Payment Receipt ===");
    println!("Sender (Stellar): {}", output.sender);
    println!("Amount: {}", output.amount);
    println!("Nonce: {}", output.nonce);
    println!("Passkey: {} bytes", output.receiver_passkey.len());
    println!("Verified: {}", output.verified);

    Ok(())
}

fn read_email_file(path: &PathBuf) -> Result<String> {
    let mut file = File::open(path).map_err(|e| anyhow!("Failed to open email file: {}", e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| anyhow!("Failed to read email contents: {}", e))?;
    Ok(contents)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        return Err(anyhow!(
            "Usage: {} <sender_domain> <sender.eml> <receiver_domain> <receiver.eml>",
            args[0]
        ));
    }

    let sender_domain = &args[1];
    let sender_path = PathBuf::from(&args[2]);
    let receiver_domain = &args[3];
    let receiver_path = PathBuf::from(&args[4]);

    verify(sender_domain, &sender_path, receiver_domain, &receiver_path).await?;
    println!("\nEmail verification and proof generation completed successfully");

    Ok(())
}
