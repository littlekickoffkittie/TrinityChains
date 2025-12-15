//! Configuration management for TrinityChain

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub database: DatabaseConfig,
    pub miner: MinerConfig,
    #[serde(default)]
    pub ai_validation: AIValidationConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub p2p_port: u16,
    pub api_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct MinerConfig {
    pub threads: usize,
    pub beneficiary_address: String,
}

#[derive(Debug, Deserialize)]
pub struct AIValidationConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub enable_transaction_validation: bool,
    #[serde(default = "default_enabled")]
    pub enable_for_all_clients: bool,
}

impl Default for AIValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: "claude-3-5-haiku-20241022".to_string(),
            provider: "anthropic".to_string(),
            timeout_secs: 30,
            enable_transaction_validation: true,
            enable_for_all_clients: true,
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_model() -> String {
    "claude-3-5-haiku-20241022".to_string()
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_timeout() -> u64 {
    30
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}
