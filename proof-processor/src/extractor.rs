use anyhow::{anyhow, Result};
use risc0_zkvm::sha::Digestible;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

// Selector for the Stellar verifier contract
const SELECTOR: &str = "73c457ba";

/// Payment receipt from zkVM journal (matches zkemail-core::PaymentReceipt)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub receiver_passkey: Vec<u8>,
    pub amount: i128,
    pub sender: String,
    pub nonce: i32,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub struct ProofData {
    pub seal: Vec<u8>,
    pub journal_bytes: Vec<u8>,
    pub journal_digest: Vec<u8>,
    pub image_id: String,
    pub payment_receipt: PaymentReceipt,
}

pub fn extract_proof_from_file(receipt_path: &Path) -> Result<ProofData> {
    tracing::info!("Reading receipt from: {}", receipt_path.display());

    let receipt_bytes = std::fs::read(receipt_path)
        .map_err(|e| anyhow!("Failed to read receipt file '{}': {}", receipt_path.display(), e))?;

    let receipt: risc0_zkvm::Receipt = bincode::deserialize(&receipt_bytes)
        .map_err(|e| anyhow!("Failed to deserialize receipt: {}", e))?;

    extract_groth16_proof_data(&receipt)
}

fn extract_groth16_proof_data(receipt: &risc0_zkvm::Receipt) -> Result<ProofData> {
    tracing::debug!("Extracting Groth16 proof data for on-chain verification");

    let groth16_receipt = match &receipt.inner {
        risc0_zkvm::InnerReceipt::Groth16(g16) => g16,
        risc0_zkvm::InnerReceipt::Fake { .. } => {
            tracing::warn!("Receipt is in Fake/dev mode format");
            return Err(anyhow!("Cannot process fake receipt (dev mode). Run with RISC0_DEV_MODE=0"));
        }
        _ => return Err(anyhow!("Receipt is not in Groth16 format")),
    };

    let seal_bytes = groth16_receipt.seal.clone();

    if seal_bytes.len() != 256 {
        return Err(anyhow!(
            "Invalid seal size: expected 256 bytes, got {} bytes",
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

    // Decode the payment receipt from journal
    let payment_receipt: PaymentReceipt = receipt.journal.decode()?;
    
    tracing::info!("Payment Receipt - Sender: {}, Amount: {}, Nonce: {}, Verified: {}", 
        payment_receipt.sender, 
        payment_receipt.amount, 
        payment_receipt.nonce,
        payment_receipt.verified
    );

    // Compute SHA-256 digest of the journal (required by Stellar verifier)
    let mut hasher = Sha256::new();
    hasher.update(&receipt.journal.bytes);
    let journal_digest = hasher.finalize().to_vec();

    Ok(ProofData {
        seal: full_seal,
        journal_bytes: receipt.journal.bytes.clone(),
        journal_digest,
        image_id: image_id.to_string(),
        payment_receipt,
    })
}
