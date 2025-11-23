use anyhow::{anyhow, Result};
use cfdkim::{
    dns::from_tokio_resolver, public_key::retrieve_public_key, validate_header,
    verify_email_with_resolver,
};
use log::{debug, error, info, warn};
use mailparse::MailHeaderMap;
use methods::{DKIM_VERIFY_ELF, DKIM_VERIFY_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, Prover};
use slog::{o, Discard, Logger};
use std::{env, fs::File, io::Read, path::PathBuf};
use trust_dns_resolver::TokioAsyncResolver;
use zkemail_core::{DKIMOutput, Email};

async fn verify_email(
    from_domain: &str,
    email_path: &PathBuf,
    target_hash: Option<String>,
) -> Result<()> {
    let logger = Logger::root(Discard, o!());
    let raw_email = read_email_file(email_path)?;
    let email = mailparse::parse_mail(raw_email.as_bytes())
        .map_err(|e| anyhow!("Failed to parse email: {}", e))?;

    debug!("Looking for DKIM signatures...");
    let dkim_headers = email.headers.get_all_headers("DKIM-Signature");
    if dkim_headers.is_empty() {
        warn!("No DKIM signatures found in email!");
        return Err(anyhow!("No DKIM signatures found"));
    }

    let resolver = TokioAsyncResolver::tokio_from_system_conf()
        .map_err(|e| anyhow!("Failed to initialize DNS resolver: {}", e))?;
    let resolver = from_tokio_resolver(resolver);

    let prover = default_prover();

    let mut extracted_public_key = None;
    let mut key_type = None;

    for header in dkim_headers.iter() {
        let header_value = String::from_utf8_lossy(header.get_value_raw());

        let dkim_header = match validate_header(&header_value) {
            Ok(h) => h,
            Err(e) => {
                debug!("Invalid DKIM header: {}", e);
                continue;
            }
        };

        if dkim_header.get_required_tag("d").to_lowercase() != from_domain.to_lowercase() {
            continue;
        }

        let algo = dkim_header.get_required_tag("a");
        let current_key_type = if algo.starts_with("rsa-") {
            "rsa"
        } else if algo.starts_with("ed25519-") {
            "ed25519"
        } else {
            debug!("Unsupported algorithm: {}", algo);
            continue;
        };

        let selector = dkim_header.get_required_tag("s");
        match retrieve_public_key(&logger, resolver.clone(), from_domain.to_string(), selector)
            .await
        {
            Ok(pk) => {
                extracted_public_key = Some(pk.to_vec());
                key_type = Some(current_key_type.to_string());
                break;
            }
            Err(e) => {
                debug!("Failed to retrieve public key: {}", e);
                continue;
            }
        }
    }

    let result = verify_email_with_resolver(&logger, from_domain, &email, resolver)
        .await
        .map_err(|e| anyhow!("Failed to verify email: {}", e))?;

    match result {
        result if result.with_detail().starts_with("pass") => {
            info!("DKIM verification passed: {}", result.with_detail());

            let email_proof = Email {
                from_domain: from_domain.to_string(),
                raw_email: raw_email.as_bytes().to_vec(),
                public_key_type: key_type.ok_or_else(|| anyhow!("No key type found"))?,
                public_key: extracted_public_key
                    .ok_or_else(|| anyhow!("No public key extracted"))?,
                target_hash,
            };

            generate_and_verify_proof(prover.as_ref(), email_proof)?;
            Ok(())
        }
        result => {
            error!("DKIM verification failed: {}", result.with_detail());
            Err(anyhow!(
                "DKIM verification failed: {}",
                result.with_detail()
            ))
        }
    }
}

fn read_email_file(path: &PathBuf) -> Result<String> {
    let mut file = File::open(path).map_err(|e| anyhow!("Failed to open email file: {}", e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| anyhow!("Failed to read email contents: {}", e))?;
    Ok(contents)
}

fn generate_and_verify_proof(prover: &dyn Prover, email: Email) -> Result<()> {
    debug!("Starting ZK proof generation");

    let input = postcard::to_allocvec(&email).unwrap();
    let env = ExecutorEnv::builder()
        .write_frame(&input)
        .build()
        .map_err(|e| anyhow!("Failed to build environment: {}", e))?;

    let prove_info = prover
        .prove(env, DKIM_VERIFY_ELF)
        .map_err(|e| anyhow!("Failed to generate proof: {}", e))?;

    let receipt = prove_info.receipt;
    let output: DKIMOutput = receipt.journal.decode()?;
    println!("{:?}", output);

    receipt
        .verify(DKIM_VERIFY_ID)
        .map_err(|e| anyhow!("Failed to verify proof: {}", e))?;

    info!("ZK proof generated and verified successfully");

    // Compress the receipt to Succinct format for Groth16 conversion
    info!("Compressing receipt to Succinct format...");
    let succinct_receipt = prover
        .compress(&risc0_zkvm::ProverOpts::succinct(), &receipt)
        .map_err(|e| anyhow!("Failed to compress receipt: {}", e))?;

    info!("Receipt compressed successfully");

    // Generate input.json for Groth16 proving
    generate_groth16_input(&succinct_receipt)?;

    Ok(())
}

fn generate_groth16_input(receipt: &risc0_zkvm::Receipt) -> Result<()> {
    info!("Generating input.json for Groth16 proving...");

    // Extract the seal bytes from the receipt
    let seal_bytes = match &receipt.inner {
        risc0_zkvm::InnerReceipt::Fake { .. } => {
            warn!("Cannot generate Groth16 input from fake receipt (dev mode)");
            warn!("Run with RISC0_DEV_MODE=0 to generate real proofs");
            return Ok(());
        }
        risc0_zkvm::InnerReceipt::Succinct(succinct) => {
            info!("Receipt is Succinct - ready for Groth16 conversion");
            &succinct.seal
        }
        _ => {
            error!("Receipt must be Succinct for Groth16 conversion");
            error!("Current receipt type is not Succinct");
            return Err(anyhow!(
                "Receipt is not in Succinct format. You may need to compress it first."
            ));
        }
    };

    // Convert seal to the proper JSON format using risc0-groth16's seal_to_json
    // This creates the correct format with pi_a, pi_b, pi_c fields as hex strings
    let input_json_str = risc0_groth16::prove::seal_to_json(seal_bytes)
        .map_err(|e| anyhow!("Failed to convert seal to JSON: {}", e))?;

    // Write to input.json
    std::fs::write("input.json", &input_json_str)?;
    info!("Generated input.json for Groth16 proving");
    info!("Seal contains {} elements", seal_bytes.len());

    // Also save the binary receipt for reference
    std::fs::write("receipt.bin", bincode::serialize(receipt)?)?;
    info!("Receipt also saved to receipt.bin");

    info!("\nTo convert to Groth16:");
    info!("  1. On an x86_64 machine with Docker:");
    info!("  2. git clone https://github.com/risc0/risc0 && cd risc0/groth16_proof");
    info!("  3. docker build -f docker/prover.Dockerfile . -t risc0-groth16-prover");
    info!("  4. Copy your input.json to that directory");
    info!("  5. docker run --rm -v $(pwd):/mnt risc0-groth16-prover");
    info!("  6. Output will be in proof.json");

    Ok(())
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

    verify_email(from_domain, &email_path, target_hash).await?;
    println!("Email verification and proof generation completed successfully");

    Ok(())
}
