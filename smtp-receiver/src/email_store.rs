use chrono::Utc;
use email_parser::{extract_passkey, extract_payment_data, get_email_body};
use log::{debug, warn};
use std::path::PathBuf;
use tokio::fs;

/// Type of email based on its content
#[derive(Debug, Clone, PartialEq)]
pub enum EmailType {
    /// Sender's email with payment data
    PaymentRequest,
    /// Receiver's reply with passkey data
    PasskeyResponse,
    /// Unknown format
    Unknown,
}

/// Represents an email in a pair
#[derive(Debug, Clone)]
pub struct PendingEmail {
    pub message_id: String,
    pub from: String,
    pub to: Vec<String>,
    pub raw_email: String,
    /// Path where this email is stored on disk
    pub file_path: PathBuf,
    /// Type of email based on content
    pub email_type: EmailType,
}

/// Result of attempting to match an incoming email
#[derive(Debug)]
pub enum MatchResult {
    /// First email in the pair (sender's original email)
    Stored {
        message_id: String,
        file_path: PathBuf,
    },
    /// Second email matched with first (receiver's reply)
    Matched {
        sender_email: PendingEmail,
        receiver_email: PendingEmail,
    },
    /// Could not determine Message-ID or In-Reply-To
    NoMessageId,
    /// Email does not match expected format (no payment data or passkey)
    InvalidFormat,
}

/// File-based store for pending emails awaiting their pair
#[derive(Clone)]
pub struct EmailStore {
    /// Directory where emails are stored
    email_dir: PathBuf,
    /// Whether to validate email format before storing
    validate_format: bool,
}

impl EmailStore {
    pub fn new(email_dir: PathBuf, validate_format: bool) -> Self {
        Self {
            email_dir,
            validate_format,
        }
    }

    /// Determine email type by checking for payment data or passkey
    fn get_email_type(raw_email: &str) -> EmailType {
        let body = get_email_body(raw_email.as_bytes());

        if extract_payment_data(&body).is_some() {
            EmailType::PaymentRequest
        } else if extract_passkey(&body).is_some() {
            EmailType::PasskeyResponse
        } else {
            EmailType::Unknown
        }
    }

    /// Generate a filename for storing an email
    fn generate_filename(from: &str) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%f");
        let sanitized_from = from.replace(['@', '.', '<', '>'], "_");
        format!("{}_{}.eml", timestamp, sanitized_from)
    }

    /// Save email to disk
    async fn save_email(&self, raw_email: &str, from: &str) -> Result<PathBuf, std::io::Error> {
        let filename = Self::generate_filename(from);
        let path = self.email_dir.join(&filename);
        fs::write(&path, raw_email).await?;
        Ok(path)
    }

    /// Process an incoming email and attempt to match it with a pending pair
    /// Saves email to disk only if it results in Stored or Matched
    pub async fn process_email(
        &self,
        raw_email: &str,
        from: &str,
        to: &[String],
    ) -> MatchResult {
        let parsed = match mailparse::parse_mail(raw_email.as_bytes()) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to parse email for Message-ID extraction: {}", e);
                return MatchResult::NoMessageId;
            }
        };

        // Detect email type
        let email_type = Self::get_email_type(raw_email);
        debug!("Detected email type: {:?}", email_type);

        // Validate format if enabled
        if self.validate_format && email_type == EmailType::Unknown {
            warn!("Email does not contain valid payment data or passkey. Rejecting.");
            return MatchResult::InvalidFormat;
        }

        let message_id_raw = Self::extract_header(&parsed, "Message-ID");
        let in_reply_to = Self::extract_header(&parsed, "In-Reply-To");
        let references = Self::extract_header(&parsed, "References");

        debug!(
            "Email headers - Message-ID: {:?}, In-Reply-To: {:?}, References: {:?}",
            message_id_raw, in_reply_to, references
        );

        // Check if we have a Message-ID before proceeding
        let message_id = match &message_id_raw {
            Some(id) => Self::normalize_message_id(id),
            None => {
                warn!("Email has no Message-ID header, cannot track for pairing");
                return MatchResult::NoMessageId;
            }
        };

        // Try to find a reference to a pending email (this is a reply)
        let reference_id = in_reply_to.or_else(|| {
            // References header contains space-separated Message-IDs, first one is the original
            references.and_then(|refs| refs.split_whitespace().next().map(|s| s.to_string()))
        });

        if let Some(ref_id) = reference_id {
            let normalized_ref = Self::normalize_message_id(&ref_id);

            // Try to find the original email on disk
            if let Some(sender_email) = self.find_pending_email(&normalized_ref).await {
                // Save the receiver email to disk
                let saved_path = match self.save_email(raw_email, from).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Failed to save receiver email: {}", e);
                        return MatchResult::NoMessageId;
                    }
                };

                let receiver_email = PendingEmail {
                    message_id: message_id.clone(),
                    from: from.to_string(),
                    to: to.to_vec(),
                    raw_email: raw_email.to_string(),
                    file_path: saved_path,
                    email_type,
                };

                return MatchResult::Matched {
                    sender_email,
                    receiver_email,
                };
            }
        }

        // No match found - store as pending (first email in pair)
        // Save the sender email to disk
        let saved_path = match self.save_email(raw_email, from).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to save sender email: {}", e);
                return MatchResult::NoMessageId;
            }
        };

        MatchResult::Stored {
            message_id,
            file_path: saved_path,
        }
    }

    /// Find a pending email by scanning the email directory for matching Message-ID
    async fn find_pending_email(&self, message_id: &str) -> Option<PendingEmail> {
        let mut entries = match fs::read_dir(&self.email_dir).await {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read email directory: {}", e);
                return None;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map(|e| e == "eml").unwrap_or(false) {
                if let Some(email) = self.check_email_file(&path, message_id).await {
                    return Some(email);
                }
            }
        }

        None
    }

    /// Check if an email file has the specified Message-ID
    async fn check_email_file(
        &self,
        path: &PathBuf,
        target_message_id: &str,
    ) -> Option<PendingEmail> {
        let contents = match fs::read_to_string(path).await {
            Ok(c) => c,
            Err(_) => return None,
        };

        let parsed = match mailparse::parse_mail(contents.as_bytes()) {
            Ok(p) => p,
            Err(_) => return None,
        };

        let message_id = Self::extract_header(&parsed, "Message-ID")?;
        let normalized = Self::normalize_message_id(&message_id);

        if normalized != target_message_id {
            return None;
        }

        // Extract other fields
        let from = Self::extract_header(&parsed, "From").unwrap_or_default();
        let email_type = Self::get_email_type(&contents);

        Some(PendingEmail {
            message_id: normalized,
            from,
            to: vec![],
            raw_email: contents,
            file_path: path.clone(),
            email_type,
        })
    }

    /// Extract a header value from parsed email
    fn extract_header(parsed: &mailparse::ParsedMail, name: &str) -> Option<String> {
        parsed
            .headers
            .iter()
            .find(|h| h.get_key().eq_ignore_ascii_case(name))
            .map(|h| h.get_value())
    }

    /// Normalize Message-ID by removing angle brackets and whitespace
    fn normalize_message_id(id: &str) -> String {
        id.trim()
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string()
    }
}
