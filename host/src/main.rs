use anyhow::{anyhow, Result};
use host::verify_email;
use std::{env, fs::File, io::Read, path::PathBuf};

async fn verify(
    from_domain: &str,
    email_path: &PathBuf,
    target_hash: Option<String>,
) -> Result<()> {
    let raw_email = read_email_file(email_path)?;
    let output = verify_email(from_domain, &raw_email, target_hash).await?;
    println!("{:?}", output);
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

    if args.len() < 3 || args.len() > 4 {
        return Err(anyhow!(
            "Usage: {} <from_domain> <email_path> [target_hash]",
            args[0]
        ));
    }

    let from_domain = &args[1];
    let email_path = PathBuf::from(&args[2]);
    let target_hash = args.get(3).map(|s| s.to_string());

    verify(from_domain, &email_path, target_hash).await?;
    println!("Email verification and proof generation completed successfully");

    Ok(())
}
