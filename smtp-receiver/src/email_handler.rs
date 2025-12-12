use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use log::{error, info, warn};
use tokio::{fs, time::sleep};

use crate::config::{Config, RunpodConfig};
use crate::email_store::{EmailStore, MatchResult, PendingEmail};

#[cfg(feature = "verify")]
use host::verify_email_pair;

pub struct EmailHandler {
    config: Config,
    email_store: EmailStore,
}

impl EmailHandler {
    pub async fn new(config: Config) -> Result<Self> {
        fs::create_dir_all(&config.storage.email_dir).await?;
        fs::create_dir_all(&config.storage.proof_dir).await?;

        let email_store = EmailStore::new(
            config.storage.email_dir.clone(),
            config.processing.validate_email_format,
        );

        Ok(Self {
            config,
            email_store,
        })
    }

    pub async fn handle_email(&self, raw_email: &str, from: &str, to: &[String]) -> Result<()> {
        info!("Received email from: {} to: {:?}", from, to);

        let from_domain = self.extract_domain(from)?;
        info!("Extracted domain: {}", from_domain);

        if !self.is_domain_allowed(&from_domain) {
            warn!("Domain {} not in allowed list, skipping", from_domain);
            return Ok(());
        }

        // Process email through the store to match pairs
        // Email is saved to disk only if it results in Stored or Matched
        let match_result = self.email_store.process_email(raw_email, from, to).await;

        match match_result {
            MatchResult::Stored {
                message_id,
                file_path,
            } => {
                info!(
                    "Email stored as pending (sender's email). Message-ID: {}. Path: {}. Waiting for reply...",
                    message_id,
                    file_path.display()
                );
            }
            MatchResult::Matched {
                sender_email,
                receiver_email,
            } => {
                info!(
                    "Email pair matched! Sender: {}, Receiver: {}",
                    sender_email.from, receiver_email.from
                );
                match self
                    .verify_email_pair_async(&sender_email, &receiver_email)
                    .await
                {
                    Ok(_) => info!("Email pair verification completed successfully"),
                    Err(e) => error!("Email pair verification failed: {}", e),
                }
            }
            MatchResult::NoMessageId => {
                warn!("Email has no Message-ID, cannot track for pairing. Skipping.");
            }
            MatchResult::InvalidFormat => {
                warn!(
                    "Email does not match expected format (no payment data or passkey). Skipping."
                );
            }
        }

        Ok(())
    }

    #[cfg(feature = "verify")]
    async fn verify_email_pair_async(
        &self,
        sender_email: &PendingEmail,
        receiver_email: &PendingEmail,
    ) -> Result<()> {
        let sender_domain = self.extract_domain(&sender_email.from)?;
        let receiver_domain = self.extract_domain(&receiver_email.from)?;

        info!(
            "Starting DKIM verification for email pair. Sender domain: {}, Receiver domain: {}",
            sender_domain, receiver_domain
        );

        let receipt = verify_email_pair(
            &sender_domain,
            &sender_email.raw_email,
            &receiver_domain,
            &receiver_email.raw_email,
        )
        .await?;

        let receipt_bytes = bincode::serialize(&receipt)
            .map_err(|e| anyhow!("Failed to serialize receipt: {}", e))?;

        self.save_receipt(&sender_email.message_id, &receipt_bytes)
            .await
    }

    #[cfg(not(feature = "verify"))]
    async fn verify_email_pair_async(
        &self,
        sender_email: &PendingEmail,
        receiver_email: &PendingEmail,
    ) -> Result<()> {
        let runpod_config = self
            .config
            .runpod
            .as_ref()
            .ok_or_else(|| anyhow!("Runpod configuration missing for verification"))?;

        let sender_domain = self.extract_domain(&sender_email.from)?;
        let receiver_domain = self.extract_domain(&receiver_email.from)?;

        info!(
            "Starting Runpod verification for email pair. Sender domain: {}, Receiver domain: {}",
            sender_domain, receiver_domain
        );

        let receipt = self
            .verify_via_runpod(
                runpod_config,
                &sender_domain,
                &sender_email.raw_email,
                &receiver_domain,
                &receiver_email.raw_email,
            )
            .await?;

        self.save_receipt(&sender_email.message_id, &receipt).await
    }

    #[cfg(not(feature = "verify"))]
    async fn verify_via_runpod(
        &self,
        runpod_config: &RunpodConfig,
        sender_domain: &str,
        sender_raw: &str,
        receiver_domain: &str,
        receiver_raw: &str,
    ) -> Result<Vec<u8>> {
        let client = reqwest::Client::new();
        let mut attempt = 0;
        loop {
            attempt += 1;
            let response = client
                .post(&runpod_config.runpod_url)
                .bearer_auth(&runpod_config.runpod_key)
                .json(&serde_json::json!({
                    "sender_domain": sender_domain,
                    "sender_raw": sender_raw,
                    "receiver_domain": receiver_domain,
                    "receiver_raw": receiver_raw,
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let bytes = resp
                        .bytes()
                        .await
                        .context("Failed to read Runpod response body")?;
                    return Ok(bytes.to_vec());
                }
                Ok(resp) => {
                    warn!(
                        "Runpod request failed with status {} on attempt {}",
                        resp.status(),
                        attempt
                    );
                }
                Err(err) => {
                    warn!(
                        "Runpod request error on attempt {}: {}",
                        attempt,
                        err
                    );
                }
            }

            if attempt >= runpod_config.retry_attempts {
                return Err(anyhow!(
                    "Runpod verification failed after {} attempts",
                    attempt
                ));
            }

            sleep(std::time::Duration::from_millis(
                runpod_config.retry_delay_decs * 100,
            ))
            .await;
        }
    }

    async fn save_receipt(&self, message_id: &str, receipt_bytes: &[u8]) -> Result<()> {
        let receipt_filename = format!(
            "receipt_{}_{}.bin",
            Utc::now().format("%Y%m%d_%H%M%S"),
            message_id.replace(['@', '.', '<', '>'], "_")
        );
        let receipt_path = self.config.storage.proof_dir.join(&receipt_filename);

        fs::write(&receipt_path, receipt_bytes)
            .await
            .with_context(|| format!("Failed to write receipt file {}", receipt_path.display()))?;
        info!("Saved receipt to: {}", receipt_path.display());

        Ok(())
    }

    fn extract_domain(&self, email: &str) -> Result<String> {
        let email = email.trim_matches(|c| c == '<' || c == '>');
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid email format: {}", email));
        }
        Ok(parts[1].to_string())
    }

    fn is_domain_allowed(&self, domain: &str) -> bool {
        if self.config.smtp.allowed_domains.is_empty() {
            return true;
        }
        self.config
            .smtp
            .allowed_domains
            .iter()
            .any(|d| d.eq_ignore_ascii_case(domain))
    }
}
