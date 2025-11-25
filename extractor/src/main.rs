use anyhow::{anyhow, Result};
use log::info;
use risc0_zkvm::sha::Digestible;
use sha2::{Digest, Sha256};
use std::env;
use zkemail_core::PaymentReceipt;

// Selector for the Stellar verifier contract
const SELECTOR: &str = "73c457ba";

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();

    let receipt_path = if args.len() >= 2 {
        &args[1]
    } else {
        "groth16_receipt.bin"
    };

    extract_proof_from_file(receipt_path)?;

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

fn extract_groth16_proof_data(receipt: &risc0_zkvm::Receipt) -> Result<()> {
    info!("Extracting Groth16 proof data for on-chain verification...");

    let groth16_receipt = match &receipt.inner {
        risc0_zkvm::InnerReceipt::Groth16(g16) => g16,
        _ => return Err(anyhow!("Receipt is not in Groth16 format")),
    };

    // Get the seal (Groth16 proof) - this should be 256 bytes (proof only)
    // proof = a (64 bytes) + b (128 bytes) + c (64 bytes)
    let seal_bytes = groth16_receipt.seal.clone();

    // Verify seal size (should be 256 bytes without selector)
    if seal_bytes.len() != 256 {
        return Err(anyhow!(
            "Invalid seal size: expected 256 bytes, got {} bytes. The seal may be incorrectly serialized.",
            seal_bytes.len()
        ));
    }

    // Prepend the selector to create the full seal (260 bytes total)
    let selector_bytes =
        hex::decode(SELECTOR).map_err(|e| anyhow!("Failed to decode selector: {}", e))?;
    let mut full_seal = Vec::with_capacity(260);
    full_seal.extend_from_slice(&selector_bytes);
    full_seal.extend_from_slice(&seal_bytes);

    // Get the image_id from the claim
    let claim = receipt.claim()?;
    let claim_value = claim.as_value()?;
    let image_id = claim_value.pre.digest();

    let output: PaymentReceipt = receipt.journal.decode()?;
    println!("\n=== Payment Receipt ===");
    println!("Sender (Stellar): {}", output.sender);
    println!("Amount: {}", output.amount);
    println!("Nonce: {}", output.nonce);
    println!("Passkey: {} bytes", output.receiver_passkey.len());
    println!("Verified: {}", output.verified);

    // Compute SHA-256 digest of the journal (required by Stellar verifier)
    let mut hasher = Sha256::new();
    hasher.update(&receipt.journal.bytes);
    let journal_digest = hasher.finalize();
    let journal_digest_hex = hex::encode(journal_digest);

    use serde_json::json;

    let proof_json = json!({
        "seal": hex::encode(&full_seal),
        "journal_hex": hex::encode(&receipt.journal.bytes),
        "journal_digest": journal_digest_hex,
        "image_id": image_id.to_string(),
        "claim_digest": claim.digest().to_string(),
    });

    std::fs::write(
        "groth16_proof.json",
        serde_json::to_string_pretty(&proof_json)?,
    )?;
    Ok(())
}
