use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use stellar_xdr::curr::{HostFunction, SorobanAuthorizationEntry};
use tokio::time::{sleep, Duration};

use crate::config::Config;
use crate::extractor::{extract_proof_from_file, ProofData};
use crate::relayer::{build_verify_and_pay_args, send_to_relayer, RelayerResponse};

/// Trait for notification handlers (extensibility point for future alerts)
pub trait NotificationHandler: Send + Sync {
    fn on_success(&self, receipt_path: &Path, tx_data: &RelayerResponse);
    fn on_failure(&self, receipt_path: &Path, error: &anyhow::Error);
}

/// Default no-op notification handler
pub struct NoOpNotificationHandler;

impl NotificationHandler for NoOpNotificationHandler {
    fn on_success(&self, _receipt_path: &Path, _tx_data: &RelayerResponse) {}
    fn on_failure(&self, _receipt_path: &Path, _error: &anyhow::Error) {}
}

pub struct ReceiptProcessor {
    config: Config,
    notification_handler: Box<dyn NotificationHandler>,
}

impl ReceiptProcessor {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            notification_handler: Box::new(NoOpNotificationHandler),
        }
    }

    pub fn with_notification_handler(mut self, handler: Box<dyn NotificationHandler>) -> Self {
        self.notification_handler = handler;
        self
    }

    /// Process a single receipt file
    pub async fn process_receipt(&self, receipt_path: &Path) -> Result<()> {
        tracing::info!("Processing receipt: {}", receipt_path.display());

        // Extract proof data
        let proof_data = extract_proof_from_file(receipt_path)
            .context("Failed to extract proof data from receipt")?;

        // Validate the payment receipt
        if !proof_data.payment_receipt.verified {
            anyhow::bail!("Payment receipt verification failed - emails not verified");
        }

        // Submit to blockchain with retries
        let response = self.submit_with_retry(&proof_data).await?;

        // Move to processed directory
        self.move_to_processed(receipt_path)
            .await
            .context("Failed to move receipt to processed directory")?;

        // Notify success
        self.notification_handler
            .on_success(receipt_path, &response);

        tracing::info!("Successfully processed receipt: {}", receipt_path.display());
        Ok(())
    }

    /// Process receipt with error handling and moving to failed directory
    pub async fn process_receipt_safe(&self, receipt_path: &Path) {
        match self.process_receipt(receipt_path).await {
            Ok(_) => {
                tracing::info!("Receipt processed successfully: {}", receipt_path.display());
            }
            Err(e) => {
                tracing::error!(
                    "Failed to process receipt {}: {}",
                    receipt_path.display(),
                    e
                );

                // Notify failure
                self.notification_handler.on_failure(receipt_path, &e);

                // Move to failed directory
                if let Err(move_err) = self.move_to_failed(receipt_path).await {
                    tracing::error!("Failed to move receipt to failed directory: {}", move_err);
                }
            }
        }
    }

    async fn submit_with_retry(&self, proof_data: &ProofData) -> Result<RelayerResponse> {
        let mut last_error = None;

        for attempt in 1..=self.config.processor.retry_attempts {
            match self.submit_to_blockchain(proof_data).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::warn!(
                        "Submission attempt {}/{} failed: {}",
                        attempt,
                        self.config.processor.retry_attempts,
                        e
                    );
                    last_error = Some(e);

                    if attempt < self.config.processor.retry_attempts {
                        sleep(Duration::from_secs(self.config.processor.retry_delay_secs)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Submission failed")))
    }

    async fn submit_to_blockchain(&self, proof_data: &ProofData) -> Result<RelayerResponse> {
        tracing::info!("Submitting proof to blockchain");
        tracing::debug!(
            "Contract: {}, Sender: {}, Amount: {}, Nonce: {}",
            self.config.stellar.contract_address,
            proof_data.payment_receipt.sender,
            proof_data.payment_receipt.amount,
            proof_data.payment_receipt.nonce
        );

        // Parse image_id from string to bytes (32 bytes)
        let image_id_bytes =
            hex::decode(&proof_data.image_id).context("Failed to decode image_id")?;

        if image_id_bytes.len() != 32 {
            anyhow::bail!(
                "Invalid image_id length: expected 32 bytes, got {}",
                image_id_bytes.len()
            );
        }

        // Build the contract invocation arguments
        let invoke_args = build_verify_and_pay_args(
            &self.config.stellar.contract_address,
            proof_data.seal.clone(),
            image_id_bytes,
            proof_data.journal_bytes.clone(),
        )?;

        // For now, we're using empty auth entries since the relayer manages authorization
        // In a production setup, you might need to build proper auth entries
        let auth_entries: Vec<SorobanAuthorizationEntry> = vec![];

        // Submit to relayer
        let response = send_to_relayer(
            &self.config.relayer.api_url,
            &self.config.relayer.api_key,
            &HostFunction::InvokeContract(invoke_args),
            auth_entries,
        )
        .await?;

        Ok(response)
    }

    async fn move_to_processed(&self, receipt_path: &Path) -> Result<()> {
        let processed_dir = &self.config.processor.processed_dir;
        tokio::fs::create_dir_all(processed_dir).await?;

        let file_name = receipt_path.file_name().context("Invalid receipt path")?;
        let dest_path = processed_dir.join(file_name);

        tokio::fs::rename(receipt_path, &dest_path).await?;
        tracing::debug!("Moved receipt to: {}", dest_path.display());

        Ok(())
    }

    async fn move_to_failed(&self, receipt_path: &Path) -> Result<()> {
        let failed_dir = &self.config.processor.failed_dir;
        tokio::fs::create_dir_all(failed_dir).await?;

        let file_name = receipt_path.file_name().context("Invalid receipt path")?;
        let dest_path = failed_dir.join(file_name);

        tokio::fs::rename(receipt_path, &dest_path).await?;
        tracing::debug!("Moved receipt to failed directory: {}", dest_path.display());

        Ok(())
    }

    /// Get list of unprocessed receipt files
    pub async fn get_pending_receipts(&self) -> Result<Vec<PathBuf>> {
        let proof_dir = &self.config.processor.proof_dir;

        if !proof_dir.exists() {
            tracing::warn!("Proof directory does not exist: {}", proof_dir.display());
            return Ok(vec![]);
        }

        let mut receipts = Vec::new();
        let mut entries = tokio::fs::read_dir(proof_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("bin") {
                receipts.push(path);
            }
        }

        receipts.sort();
        Ok(receipts)
    }
}
