use anyhow::{anyhow, Result};
use log::{error, info, warn};
use tokio::fs;

#[cfg(feature = "verify")]
use chrono::Utc;

use crate::config::Config;
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

        let receipt_filename = format!(
            "receipt_{}_{}.bin",
            Utc::now().format("%Y%m%d_%H%M%S"),
            sender_email.message_id.replace(['@', '.', '<', '>'], "_")
        );
        let receipt_path = self.config.storage.proof_dir.join(&receipt_filename);

        let receipt_bytes = bincode::serialize(&receipt)
            .map_err(|e| anyhow!("Failed to serialize receipt: {}", e))?;
        fs::write(&receipt_path, receipt_bytes).await?;
        info!("Saved receipt to: {}", receipt_path.display());

        Ok(())
    }

    #[cfg(not(feature = "verify"))]
    async fn verify_email_pair_async(
        &self,
        sender_email: &PendingEmail,
        receiver_email: &PendingEmail,
    ) -> Result<()> {
        warn!(
            "Verification disabled (build without 'verify' feature). \
             Email pair from {} and {} saved but not verified.",
            sender_email.from, receiver_email.from
        );
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
