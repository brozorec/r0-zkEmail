use anyhow::{anyhow, Result};
use chrono::Utc;
use log::{error, info, warn};
use tokio::fs;

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

        let email_store = EmailStore::new(config.storage.email_dir.clone());

        Ok(Self {
            config,
            email_store,
        })
    }

    pub async fn handle_email(&self, raw_email: &str, from: &str, to: &[String]) -> Result<()> {
        info!("Received email from: {} to: {:?}", from, to);

        // Save email to disk
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%f");
        let sanitized_from = from.replace(['@', '.', '<', '>'], "_");
        let filename = format!("{}_{}.eml", timestamp, sanitized_from);
        let email_path = self.config.storage.email_dir.join(&filename);

        fs::write(&email_path, raw_email).await?;
        info!("Saved email to: {}", email_path.display());

        let from_domain = self.extract_domain(from)?;
        info!("Extracted domain: {}", from_domain);

        if !self.is_domain_allowed(&from_domain) {
            warn!(
                "Domain {} not in allowed list, skipping verification",
                from_domain
            );
            return Ok(());
        }

        // Process email through the store to match pairs
        let match_result = self
            .email_store
            .process_email(raw_email, from, to, email_path.clone())
            .await;

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
        }

        Ok(())
    }

    #[cfg(feature = "verify")]
    async fn verify_email_pair_async(
        &self,
        sender_email: &PendingEmail,
        receiver_email: &PendingEmail,
    ) -> Result<()> {
        info!(
            "Starting DKIM verification for email pair. Sender domain: {}, Receiver domain: {}",
            sender_email.from_domain, receiver_email.from_domain
        );

        let output = verify_email_pair(
            &sender_email.from_domain,
            &sender_email.raw_email,
            &receiver_email.from_domain,
            &receiver_email.raw_email,
        )
        .await?;

        info!("Email pair verification result: {:?}", output);

        let proof_filename = format!(
            "pair_{}_{}.json",
            Utc::now().format("%Y%m%d_%H%M%S"),
            sender_email.message_id.replace(['@', '.', '<', '>'], "_")
        );
        let proof_path = self.config.storage.proof_dir.join(&proof_filename);

        let proof_data = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "sender": {
                "from": sender_email.from,
                "domain": sender_email.from_domain,
                "message_id": sender_email.message_id,
                "received_at": sender_email.received_at.to_rfc3339(),
            },
            "receiver": {
                "from": receiver_email.from,
                "domain": receiver_email.from_domain,
                "message_id": receiver_email.message_id,
                "received_at": receiver_email.received_at.to_rfc3339(),
            },
            "output": {
                "sender": output.sender,
                "amount": output.amount,
                "nonce": output.nonce,
                "verified": output.verified,
            }
        });

        fs::write(&proof_path, serde_json::to_string_pretty(&proof_data)?).await?;
        info!("Saved proof data to: {}", proof_path.display());

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
