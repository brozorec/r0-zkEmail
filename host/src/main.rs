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

    // Convert to Groth16 format
    let _groth16_receipt = convert_to_groth16(prover, &succinct_receipt)?;

    Ok(())
}

fn convert_to_groth16(
    prover: &dyn Prover,
    receipt: &risc0_zkvm::Receipt,
) -> Result<risc0_zkvm::Receipt> {
    info!("Converting receipt to Groth16 format...");

    // Check if already in dev mode
    if matches!(&receipt.inner, risc0_zkvm::InnerReceipt::Fake { .. }) {
        warn!("Cannot generate Groth16 from fake receipt (dev mode)");
        warn!("Run with RISC0_DEV_MODE=0 to generate real proofs");
        return Ok(receipt.clone());
    }

    // Use the Prover's compress method with Groth16 options
    // This handles the entire STARK-to-SNARK conversion automatically
    info!("Compressing to Groth16 using Docker...");
    let groth16_receipt = prover
        .compress(&risc0_zkvm::ProverOpts::groth16(), receipt)
        .map_err(|e| anyhow!("Failed to compress to Groth16: {}", e))?;

    info!("Groth16 receipt generated successfully!");

    // Save the Groth16 receipt
    std::fs::write("groth16_receipt.bin", bincode::serialize(&groth16_receipt)?)?;
    info!("Groth16 receipt saved to groth16_receipt.bin");

    // Extract and save the proof data for on-chain verification
    extract_groth16_proof_data(&groth16_receipt)?;

    Ok(groth16_receipt)
}

fn extract_groth16_proof_data(receipt: &risc0_zkvm::Receipt) -> Result<()> {
    info!("Extracting Groth16 proof data for on-chain verification...");

    let groth16_receipt = match &receipt.inner {
        risc0_zkvm::InnerReceipt::Groth16(g16) => g16,
        _ => return Err(anyhow!("Receipt is not in Groth16 format")),
    };

    // Get the seal (Groth16 proof)
    let seal = &groth16_receipt.seal;

    // Convert seal to JSON format for easy viewing
    use serde_json::json;

    let proof_json = json!({
        "seal": seal,
        "journal": hex::encode(&receipt.journal.bytes),
        "claim": receipt.claim()?.digest().to_string(),
    });

    std::fs::write(
        "groth16_proof.json",
        serde_json::to_string_pretty(&proof_json)?,
    )?;
    info!("Groth16 proof data saved to groth16_proof.json");

    // Also save the seal bytes separately for contract submission
    let seal_bytes = bincode::serialize(seal)?;
    std::fs::write("groth16_seal.bin", &seal_bytes)?;
    info!("Groth16 seal bytes saved to groth16_seal.bin");

    info!("\n=== Groth16 Proof Data for Stellar Contract ===");
    info!("Journal (hex): {}", hex::encode(&receipt.journal.bytes));
    info!("Claim digest: {}", receipt.claim()?.digest());
    info!("Seal size: {} bytes", seal_bytes.len());
    info!("\nFiles generated:");
    info!("  - groth16_receipt.bin: Full receipt (for verification)");
    info!("  - groth16_proof.json: Human-readable proof data");
    info!("  - groth16_seal.bin: Seal bytes for contract submission");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();

    // Check if first argument is "extract-proof"
    if args.len() >= 2 && args[1] == "extract-proof" {
        let receipt_path = if args.len() >= 3 {
            &args[2]
        } else {
            "groth16_receipt.bin"
        };
        return extract_proof_from_file(receipt_path);
    }

    // Original email verification flow
    if args.len() < 3 || args.len() > 4 {
        return Err(anyhow!(
            "Usage:\n  {} <from_domain> <email_path> [target_hash]\n  {} extract-proof [receipt_path]",
            args[0], args[0]
        ));
    }

    let from_domain = &args[1];
    let email_path = PathBuf::from(&args[2]);
    let target_hash = args.get(3).map(|s| s.to_string());

    verify_email(from_domain, &email_path, target_hash).await?;
    println!("Email verification and proof generation completed successfully");

    Ok(())
}

fn extract_proof_from_file(receipt_path: &str) -> Result<()> {
    info!("Reading Groth16 receipt from: {}", receipt_path);

    // Read the receipt file
    let receipt_bytes = std::fs::read(receipt_path)
        .map_err(|e| anyhow!("Failed to read receipt file '{}': {}", receipt_path, e))?;

    // Deserialize the receipt
    let receipt: risc0_zkvm::Receipt = bincode::deserialize(&receipt_bytes)
        .map_err(|e| anyhow!("Failed to deserialize receipt: {}", e))?;

    // Extract the proof data
    extract_groth16_proof_data(&receipt)?;

    println!("\nProof data extraction completed successfully!");
    println!("Check the generated files for contract submission data.");

    Ok(())
}
