use anyhow::{anyhow, Result};
use chrono::Utc;
use host::verify_email;
use log::{error, info, warn};
use tokio::fs;

use crate::config::Config;

pub struct EmailHandler {
    config: Config,
}

impl EmailHandler {
    pub async fn new(config: Config) -> Result<Self> {
        fs::create_dir_all(&config.storage.email_dir).await?;
        fs::create_dir_all(&config.storage.proof_dir).await?;

        Ok(Self { config })
    }

    pub async fn handle_email(&self, raw_email: &str, from: &str, to: &[String]) -> Result<()> {
        info!("Received email from: {} to: {:?}", from, to);

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%f");
        let sanitized_from = from.replace(['@', '.', '<', '>'], "_");
        let filename = format!("{}_{}.eml", timestamp, sanitized_from);
        let email_path = self.config.storage.email_dir.join(&filename);

        fs::write(&email_path, raw_email).await?;
        info!("Saved email to: {}", email_path.display());

        if self.config.processing.auto_verify {
            let from_domain = self.extract_domain(from)?;
            info!("Extracted domain: {}", from_domain);

            if !self.is_domain_allowed(&from_domain) {
                warn!(
                    "Domain {} not in allowed list, skipping verification",
                    from_domain
                );
                return Ok(());
            }

            match self
                .verify_email_async(&from_domain, raw_email, &filename)
                .await
            {
                Ok(_) => info!("Email verification completed successfully for {}", filename),
                Err(e) => error!("Email verification failed for {}: {}", filename, e),
            }
        }

        Ok(())
    }

    async fn verify_email_async(
        &self,
        from_domain: &str,
        raw_email: &str,
        filename: &str,
    ) -> Result<()> {
        info!("Starting DKIM verification for domain: {}", from_domain);

        let output = verify_email(from_domain, raw_email, None).await?;

        info!("DKIM verification result: {:?}", output);

        let proof_filename = format!("{}.json", filename.trim_end_matches(".eml"));
        let proof_path = self.config.storage.proof_dir.join(proof_filename);

        let proof_data = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "from_domain": from_domain,
            "filename": filename,
            "output": {
                "from_domain_hash": hex::encode(&output.from_domain_hash),
                "public_key_hash": hex::encode(&output.public_key_hash),
                "verified": output.verified,
                "hash_found": output.hash_found,
            }
        });

        fs::write(&proof_path, serde_json::to_string_pretty(&proof_data)?).await?;
        info!("Saved proof data to: {}", proof_path.display());

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
