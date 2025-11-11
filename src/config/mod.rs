use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub organization: String,
    pub audit_url: Option<String>,
    pub github_token: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            anyhow::bail!("Configuration not found. Run 'with-env init' first.");
        }

        let contents =
            std::fs::read_to_string(&config_path).context("Failed to read configuration file")?;

        toml::from_str(&contents).context("Failed to parse configuration file")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        std::fs::write(&config_path, contents).context("Failed to write configuration file")?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to determine config directory")?;

        Ok(config_dir.join("with-env").join("config.toml"))
    }

    pub fn get_github_token(&self) -> Result<String> {
        if let Some(token) = &self.github_token {
            return Ok(token.clone());
        }

        std::env::var("GITHUB_TOKEN")
            .context("GitHub token not found in config or GITHUB_TOKEN environment variable")
    }
}
