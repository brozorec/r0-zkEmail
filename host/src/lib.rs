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
use trust_dns_resolver::TokioAsyncResolver;
use zkemail_core::{Email, EmailPair};

/// Verify a pair of emails and generate a ZK proof
/// - sender_email: Original email with PAYMENT DATA
/// - receiver_email: Reply email with PASSKEY DATA
pub async fn verify_email_pair(
    sender_domain: &str,
    sender_raw: &str,
    receiver_domain: &str,
    receiver_raw: &str,
) -> Result<risc0_zkvm::Receipt> {
    info!("Verifying sender email from domain: {}", sender_domain);
    let sender_email = prepare_email(sender_domain, sender_raw).await?;

    info!("Verifying receiver email from domain: {}", receiver_domain);
    let receiver_email = prepare_email(receiver_domain, receiver_raw).await?;

    let email_pair = EmailPair {
        sender_email,
        receiver_email,
    };

    generate_and_verify_proof(&email_pair)
}

/// Prepare a single email for proof generation by extracting DKIM public key
async fn prepare_email(from_domain: &str, raw_email: &str) -> Result<Email> {
    let logger = Logger::root(Discard, o!());
    let email = mailparse::parse_mail(raw_email.as_bytes())
        .map_err(|e| anyhow!("Failed to parse email: {}", e))?;

    debug!("Looking for DKIM signatures...");
    let dkim_headers = email.headers.get_all_headers("DKIM-Signature");
    if dkim_headers.is_empty() {
        warn!("No DKIM signatures found in email!");
        return Err(anyhow!("No DKIM signatures found"));
    }

    let tokio_resolver = TokioAsyncResolver::tokio_from_system_conf()
        .map_err(|e| anyhow!("Failed to initialize DNS resolver: {}", e))?;
    let resolver = from_tokio_resolver(tokio_resolver);

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

            Ok(Email {
                from_domain: from_domain.to_string(),
                raw_email: raw_email.as_bytes().to_vec(),
                public_key_type: key_type.ok_or_else(|| anyhow!("No key type found"))?,
                public_key: extracted_public_key
                    .ok_or_else(|| anyhow!("No public key extracted"))?,
            })
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

fn generate_and_verify_proof(email_pair: &EmailPair) -> Result<risc0_zkvm::Receipt> {
    debug!("Starting ZK proof generation");

    let prover = default_prover();

    let input = postcard::to_allocvec(&email_pair).unwrap();
    let env = ExecutorEnv::builder()
        .write_frame(&input)
        .build()
        .map_err(|e| anyhow!("Failed to build environment: {}", e))?;

    let prove_info = prover
        .prove(env, DKIM_VERIFY_ELF)
        .map_err(|e| anyhow!("Failed to generate proof: {}", e))?;

    let receipt = prove_info.receipt;

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
    convert_to_groth16(&prover, &succinct_receipt)
}

fn convert_to_groth16(
    prover: &dyn Prover,
    receipt: &risc0_zkvm::Receipt,
) -> Result<risc0_zkvm::Receipt> {
    info!("Converting receipt to Groth16 format...");

    if matches!(&receipt.inner, risc0_zkvm::InnerReceipt::Fake { .. }) {
        warn!("Cannot generate Groth16 from fake receipt (dev mode)");
        warn!("Run with RISC0_DEV_MODE=0 to generate real proofs");
        return Ok(receipt.clone());
    }

    info!("Compressing to Groth16...");
    let groth16_receipt = prover
        .compress(&risc0_zkvm::ProverOpts::groth16(), receipt)
        .map_err(|e| anyhow!("Failed to compress to Groth16: {}", e))?;

    info!("Groth16 receipt generated successfully!");

    Ok(groth16_receipt)
}
