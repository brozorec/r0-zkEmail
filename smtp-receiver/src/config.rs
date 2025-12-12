use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub smtp: SmtpConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub processing: ProcessingConfig,
    #[serde(default)]
    pub runpod: Option<RunpodConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_message_size: usize,
    pub allowed_domains: Vec<String>,
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub email_dir: PathBuf,
    pub proof_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingConfig {
    /// Validate that emails contain expected payment/passkey data format before storing
    #[serde(default)]
    pub validate_email_format: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunpodConfig {
    pub runpod_url: String,
    pub runpod_key: String,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_delay_decs")]
    pub retry_delay_decs: u64,
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay_decs() -> u64 {
    10 // 1 second
}

impl Default for Config {
    fn default() -> Self {
        Self {
            smtp: SmtpConfig {
                bind_address: "0.0.0.0".to_string(),
                port: 2525,
                max_message_size: 50 * 1024 * 1024,
                allowed_domains: vec![],
                tls: TlsConfig {
                    cert_path: PathBuf::from("./certs/cert.pem"),
                    key_path: PathBuf::from("./certs/key.pem"),
                },
            },
            storage: StorageConfig {
                email_dir: PathBuf::from("./received_emails"),
                proof_dir: PathBuf::from("./proofs"),
            },
            processing: ProcessingConfig {
                validate_email_format: true,
            },
            runpod: None,
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_default(path: &str) -> anyhow::Result<()> {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}
