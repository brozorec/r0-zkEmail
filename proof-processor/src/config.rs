use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub processor: ProcessorConfig,
    pub stellar: StellarConfig,
    pub relayer: RelayerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    #[serde(default = "default_proof_dir")]
    pub proof_dir: PathBuf,
    
    #[serde(default = "default_processed_dir")]
    pub processed_dir: PathBuf,
    
    #[serde(default = "default_failed_dir")]
    pub failed_dir: PathBuf,
    
    #[serde(default = "default_scan_interval")]
    pub scan_interval_secs: u64,
    
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StellarConfig {
    #[serde(default = "default_network")]
    pub network: String,
    
    pub contract_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    #[serde(default = "default_relayer_url")]
    pub api_url: String,
    
    pub api_key: String,
}

fn default_proof_dir() -> PathBuf {
    PathBuf::from("./proofs")
}

fn default_processed_dir() -> PathBuf {
    PathBuf::from("./proofs/processed")
}

fn default_failed_dir() -> PathBuf {
    PathBuf::from("./proofs/failed")
}

fn default_scan_interval() -> u64 {
    60
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    5
}

fn default_network() -> String {
    "testnet".to_string()
}

fn default_relayer_url() -> String {
    "https://channels.openzeppelin.com/testnet".to_string()
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;
        
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.stellar.contract_address.is_empty() {
            anyhow::bail!("stellar.contract_address must be set");
        }
        
        if self.relayer.api_key.is_empty() {
            anyhow::bail!("relayer.api_key must be set");
        }
        
        if !["testnet", "futurenet", "mainnet"].contains(&self.stellar.network.as_str()) {
            anyhow::bail!("stellar.network must be one of: testnet, futurenet, mainnet");
        }
        
        Ok(())
    }
    
    pub fn network_passphrase(&self) -> &str {
        match self.stellar.network.as_str() {
            "testnet" => "Test SDF Network ; September 2015",
            "futurenet" => "Test SDF Future Network ; October 2022",
            "mainnet" => "Public Global Stellar Network ; September 2015",
            _ => "Test SDF Network ; September 2015",
        }
    }
}
