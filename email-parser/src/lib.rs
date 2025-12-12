use mailparse::MailHeaderMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasskeyData {
    pub public_key: Vec<u8>,
    pub cred_id: Vec<u8>,
}

/// Extract the text body from a raw email
pub fn get_email_body(raw_email: &[u8]) -> String {
    let parsed = mailparse::parse_mail(raw_email).expect("Failed to parse email");

    // Try to get text/plain part from multipart email
    if !parsed.subparts.is_empty() {
        for part in &parsed.subparts {
            if let Some(content_type) = part.headers.get_first_value("Content-Type") {
                if content_type.contains("text/plain") {
                    return part.get_body().unwrap_or_default();
                }
            }
        }
        // Fallback to first part
        return parsed.subparts[0].get_body().unwrap_or_default();
    }

    parsed.get_body().unwrap_or_default()
}

/// Extract payment data from email body
/// Format:
/// ---BEGIN PAYMENT DATA---
/// SENDER: <stellar_address>
/// AMOUNT: <amount>
/// NONCE: <nonce>
/// ---END PAYMENT DATA---
pub fn extract_payment_data(body: &str) -> Option<(String, i128, i32)> {
    let start_marker = "---BEGIN PAYMENT DATA---";
    // Be flexible with end marker (allow 2 or 3 dashes, with or without trailing spaces)
    let start = body.find(start_marker)? + start_marker.len();
    let end = find_end_marker(body, "---END PAYMENT DATA")?.min(body.len());
    let data_block = &body[start..end];

    let mut sender: Option<String> = None;
    let mut amount: Option<i128> = None;
    let mut nonce: Option<i32> = None;

    for line in data_block.lines() {
        // Strip quote markers (>) and whitespace
        let line = line.trim().trim_start_matches('>').trim();

        if let Some(value) = line.strip_prefix("SENDER:") {
            sender = Some(clean_value(value));
        } else if let Some(value) = line.strip_prefix("AMOUNT:") {
            amount = clean_value(value).parse().ok();
        } else if let Some(value) = line.strip_prefix("NONCE:") {
            nonce = clean_value(value).parse().ok();
        }
    }

    Some((sender?, amount?, nonce?))
}

/// Extract passkey public key and credential ID from email body
/// Format:
/// ---BEGIN PASSKEY DATA---
/// PUBLIC_KEY: <hex_encoded_65_byte_key>
/// CREDENTIAL_ID: <hex_data>
/// ---END PASSKEY DATA---
/// Also supports multi-line format where the key is on the next line(s)
pub fn extract_passkey(body: &str) -> Option<PasskeyData> {
    let start_marker = "---BEGIN PASSKEY DATA---";
    // Be flexible with end marker (allow 2 or 3 dashes)
    let start = body.find(start_marker)? + start_marker.len();
    let end = find_end_marker(body, "---END PASSKEY DATA")?.min(body.len());
    let data_block = &body[start..end];

    enum CaptureTarget {
        PublicKey,
        CredentialId,
    }

    let mut current_target: Option<CaptureTarget> = None;
    let mut public_key_hex: Vec<String> = Vec::new();
    let mut credential_id_hex: Vec<String> = Vec::new();

    for line in data_block.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        if let Some(value) = line.strip_prefix("PUBLIC_KEY:") {
            let trimmed_value = clean_value(value);
            if !trimmed_value.is_empty() {
                public_key_hex.push(trimmed_value);
                current_target = None;
            } else {
                current_target = Some(CaptureTarget::PublicKey);
            }
            continue;
        }

        if let Some(value) = line.strip_prefix("CREDENTIAL_ID:") {
            let trimmed_value = clean_value(value);
            if !trimmed_value.is_empty() {
                credential_id_hex.push(trimmed_value);
                current_target = None;
            } else {
                current_target = Some(CaptureTarget::CredentialId);
            }
            continue;
        }

        match current_target {
            Some(CaptureTarget::PublicKey) => {
                let cleaned = clean_value(line);
                if !cleaned.is_empty() {
                    public_key_hex.push(cleaned);
                }
            }
            Some(CaptureTarget::CredentialId) => {
                let cleaned = clean_value(line);
                if !cleaned.is_empty() {
                    credential_id_hex.push(cleaned);
                }
            }
            None => {}
        }
    }

    if public_key_hex.is_empty() || credential_id_hex.is_empty() {
        return None;
    }

    let combined_public = public_key_hex.join("");
    let combined_credential = credential_id_hex.join("");

    Some(PasskeyData {
        public_key: hex_decode(&combined_public)?,
        cred_id: hex_decode(&combined_credential)?,
    })
}

/// Find end marker with flexible matching (2 or 3 trailing dashes)
fn find_end_marker(body: &str, prefix: &str) -> Option<usize> {
    // Try exact match first (3 dashes)
    if let Some(pos) = body.find(&format!("{}---", prefix)) {
        return Some(pos);
    }
    // Try 2 dashes
    if let Some(pos) = body.find(&format!("{}--", prefix)) {
        return Some(pos);
    }
    None
}

/// Clean a value by removing non-breaking spaces and other unicode whitespace
fn clean_value(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() || *c == ' ')
        .collect::<String>()
        .replace('\u{00A0}', "") // non-breaking space
        .trim()
        .to_string()
}

/// Decode hex string to bytes
pub fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }

    Some(bytes)
}

/// Get a header value from a parsed email
pub fn get_header(parsed: &mailparse::ParsedMail, name: &str) -> Option<String> {
    parsed.headers.get_first_value(name)
}

/// Extract email address from a From/To header value
/// e.g. "FirstName LastName <sender@gmail.com>" -> "sender@gmail.com"
pub fn extract_email_address(header_value: &str) -> String {
    if let Some(start) = header_value.find('<') {
        if let Some(end) = header_value.find('>') {
            return header_value[start + 1..end].to_string();
        }
    }
    // If no angle brackets, assume the whole thing is the email
    header_value.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_payment_data() {
        let body = r#"
Hi, how are you?

---BEGIN PAYMENT DATA---
SENDER: CBL23CO6R3YY7A2QVXJY7LS6TTTLGPR6T5HSV6PYC7NASBCBC4M67FCN
AMOUNT: 1000000000
NONCE: -1231323
---END PAYMENT DATA---

More free content ...... blah blah
"#;

        let result = extract_payment_data(body);
        assert!(result.is_some());
        let (sender, amount, nonce) = result.unwrap();
        assert_eq!(
            sender,
            "CBL23CO6R3YY7A2QVXJY7LS6TTTLGPR6T5HSV6PYC7NASBCBC4M67FCN"
        );
        assert_eq!(amount, 1000000000);
        assert_eq!(nonce, -1231323);
    }

    #[test]
    fn test_extract_passkey() {
        let body = r#"
Hi thanks

---BEGIN PASSKEY DATA---
PUBLIC_KEY: 04f947be0b3df0ae82c2fa88b371ee9eff17ca959d5c88932ca71d727619850f4542f8f11538910c3e5dfe116264adca5025a49fd60747b1ba820f90e2d96ba14b
CREDENTIAL_ID: deadbeefcafebabe
---END PASSKEY DATA---
"#;

        let result = extract_passkey(body);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.public_key.len(), 65);
        assert_eq!(data.public_key[0], 0x04); // Uncompressed key prefix
        assert_eq!(
            data.cred_id,
            vec![0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]
        );
    }

    #[test]
    fn test_extract_passkey_multiline() {
        // Test multi-line format where PUBLIC_KEY value is on separate lines
        let body = r#"
---BEGIN PASSKEY DATA---
PUBLIC_KEY:
04beb2f6bb9f9b9406c46292957836aa6bddde48131ea0fabb33c0aa39f3c9a0641d2be6caf248c91de35f5113382a046e8fc946598d028d820b056d209019f47c
CREDENTIAL_ID:
001122334455
---END PASSKEY DATA--
"#;

        let result = extract_passkey(body);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.public_key.len(), 65);
        assert_eq!(data.public_key[0], 0x04); // Uncompressed key prefix
        assert_eq!(data.cred_id, vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_extract_passkey_multiline_wrapped() {
        // Test multi-line format where hex is wrapped across multiple lines
        let body = r#"
---BEGIN PASSKEY DATA---
PUBLIC_KEY:
04beb2f6bb9f9b9406c46292957836aa6bddde48131ea0fabb33c0aa39f3c9a064
1d2be6caf248c91de35f5113382a046e8fc946598d028d820b056d209019f47c
CREDENTIAL_ID:
aa
bb
ccdd
---END PASSKEY DATA---
"#;

        let result = extract_passkey(body);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.public_key.len(), 65);
        assert_eq!(data.public_key[0], 0x04); // Uncompressed key prefix
        assert_eq!(data.cred_id, vec![0xaa, 0xbb, 0xcc, 0xdd]);
    }

    #[test]
    fn test_hex_decode() {
        assert_eq!(hex_decode("0102030405"), Some(vec![1, 2, 3, 4, 5]));
        assert_eq!(hex_decode("ff00"), Some(vec![255, 0]));
        assert_eq!(hex_decode("invalid"), None);
        assert_eq!(hex_decode("0"), None); // Odd length
    }

    #[test]
    fn test_extract_email_address() {
        assert_eq!(
            extract_email_address("My Name <sender@gmail.com>"),
            "sender@gmail.com"
        );
        assert_eq!(extract_email_address("server@ik.me"), "server@ik.me");
        assert_eq!(
            extract_email_address("My Name <sender@gmail.com>, server@ik.me"),
            "sender@gmail.com"
        );
    }

    #[test]
    fn test_extract_payment_data_missing_fields() {
        let body = "---BEGIN PAYMENT DATA---\nSENDER: ABC\n---END PAYMENT DATA---";
        assert!(extract_payment_data(body).is_none());
    }

    #[test]
    fn test_extract_passkey_missing() {
        let body = "No passkey here";
        assert!(extract_passkey(body).is_none());
    }

    #[test]
    fn test_extract_passkey_missing_credential() {
        let body = r#"
---BEGIN PASSKEY DATA---
PUBLIC_KEY: 04f947be0b3df0ae82c2fa88b371ee9eff17ca959d5c88932ca71d727619850f4542f8f11538910c3e5dfe116264adca5025a49fd60747b1ba820f90e2d96ba14b
---END PASSKEY DATA---
"#;
        assert!(extract_passkey(body).is_none());
    }
}
