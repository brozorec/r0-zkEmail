use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub smtp: SmtpConfig,
    pub storage: StorageConfig,
    pub processing: ProcessingConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub auto_verify: bool,
    pub extract_domain_from_sender: bool,
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
                auto_verify: true,
                extract_domain_from_sender: true,
            },
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
