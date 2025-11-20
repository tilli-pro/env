use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub organization: String,
    pub audit_url: Option<String>,
    pub audit_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

        // Set secure permissions on config file
        crate::security::set_secure_permissions(&config_path)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to determine config directory")?;

        Ok(config_dir.join("with-env").join("config.toml"))
    }

    /// Retrieves GitHub token from multiple sources in priority order:
    /// 1. GITHUB_TOKEN environment variable (highest priority)
    /// 2. System keyring (may not work on unsigned binaries on macOS)
    /// 3. GitHub CLI via `gh auth token` (automatic fallback)
    /// 4. Config file (deprecated, for backwards compatibility)
    pub fn get_github_token(&self) -> Result<String> {
        // Try environment variable first (most reliable for CLI tools)
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            return Ok(token);
        }

        // Try keyring second (may not work on unsigned binaries on macOS)
        if let Ok(token) = Self::get_token_from_keyring() {
            return Ok(token);
        }

        // Try GitHub CLI third (automatic fallback)
        if let Ok(token) = Self::get_token_from_gh_cli() {
            return Ok(token);
        }

        // Fall back to config file (deprecated, but supported for backwards compatibility)
        if let Some(token) = &self.github_token {
            eprintln!("⚠️  WARNING: Storing tokens in config file is deprecated and insecure.");
            eprintln!(
                "   Please migrate to environment variable: export GITHUB_TOKEN=$(gh auth token)"
            );
            eprintln!();
            return Ok(token.clone());
        }

        anyhow::bail!(
            "GitHub token not found.\n\
            \n\
            Please provide your GitHub token using one of these methods:\n\
            \n\
            1. Authenticate with GitHub CLI (automatic):\n\
               gh auth login\n\
            \n\
            2. Environment variable:\n\
               export GITHUB_TOKEN=$(gh auth token)\n\
            \n\
            3. Keyring storage (requires code-signed binary on macOS):\n\
               with-env init --org {} --token YOUR_TOKEN\n\
            \n\
            The tool will automatically use your GitHub CLI authentication if available.",
            self.organization
        )
    }

    fn get_token_from_keyring() -> Result<String> {
        let entry = keyring::Entry::new("with-env", "github_token")?;
        entry.get_password().context("Token not found in keyring")
    }

    fn get_token_from_gh_cli() -> Result<String> {
        // Check if gh CLI is installed
        let check = std::process::Command::new("gh").arg("--version").output();

        if check.is_err() {
            anyhow::bail!("GitHub CLI not installed");
        }

        // Try to get the token
        let output = std::process::Command::new("gh")
            .arg("auth")
            .arg("token")
            .output()
            .context("Failed to execute 'gh auth token'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("GitHub CLI not authenticated: {}", stderr.trim());
        }

        let token = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in gh auth token output")?
            .trim()
            .to_string();

        if token.is_empty() {
            anyhow::bail!("GitHub CLI returned empty token");
        }

        Ok(token)
    }

    pub fn store_token_in_keyring(token: &str) -> Result<()> {
        let entry = keyring::Entry::new("with-env", "github_token")?;
        entry
            .set_password(token)
            .context("Failed to store token in keyring")?;
        Ok(())
    }

    pub fn get_audit_token(&self) -> Option<&str> {
        self.audit_token.as_deref()
    }
}
