use anyhow::{Context, Result};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::sealedbox;

#[derive(Debug, Deserialize)]
pub struct RepositoryPublicKey {
    pub key_id: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Secret {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct SecretsResponse {
    secrets: Vec<Secret>,
}

#[derive(Debug, Deserialize)]
struct EnvironmentsResponse {
    environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
struct Environment {
    name: String,
}

pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

impl GitHubClient {
    pub fn new(token: &str) -> Result<Self> {
        let client = reqwest::Client::new();

        Ok(Self {
            client,
            token: token.to_string(),
        })
    }

    pub async fn list_environments(&self, owner: &str, repo: &str) -> Result<Vec<String>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments",
            owner, repo
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "with-env-cli")
            .send()
            .await
            .context("Failed to send request")?;

        let response: EnvironmentsResponse =
            response.json().await.context("Failed to parse response")?;

        let environments = response
            .environments
            .into_iter()
            .map(|env| env.name)
            .collect();

        Ok(environments)
    }

    pub async fn list_secrets(
        &self,
        owner: &str,
        repo: &str,
        environment: &str,
    ) -> Result<Vec<Secret>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments/{}/secrets",
            owner, repo, environment
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "with-env-cli")
            .send()
            .await
            .context("Failed to send request")?;

        let response: SecretsResponse =
            response.json().await.context("Failed to parse response")?;

        Ok(response.secrets)
    }

    pub async fn get_secret(
        &self,
        owner: &str,
        repo: &str,
        environment: &str,
        secret_name: &str,
    ) -> Result<Secret> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments/{}/secrets/{}",
            owner, repo, environment, secret_name
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "with-env-cli")
            .send()
            .await
            .context("Failed to send request")?;

        let secret: Secret = response.json().await.context("Failed to parse response")?;

        Ok(secret)
    }

    pub async fn set_secret(
        &self,
        owner: &str,
        repo: &str,
        environment: &str,
        secret_name: &str,
        secret_value: &str,
    ) -> Result<()> {
        // Get the repository public key
        let public_key = self.get_repository_public_key(owner, repo).await?;

        // Encrypt the secret value
        let encrypted_value = self.encrypt_secret(&public_key.key, secret_value)?;

        // Set the secret
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments/{}/secrets/{}",
            owner, repo, environment, secret_name
        );

        let payload = serde_json::json!({
            "encrypted_value": encrypted_value,
            "key_id": public_key.key_id,
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "with-env-cli")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to set secret: {} - {}", status, body);
        }

        Ok(())
    }

    pub async fn delete_secret(
        &self,
        owner: &str,
        repo: &str,
        environment: &str,
        secret_name: &str,
    ) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments/{}/secrets/{}",
            owner, repo, environment, secret_name
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "with-env-cli")
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete secret: {} - {}", status, body);
        }

        Ok(())
    }

    async fn get_repository_public_key(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<RepositoryPublicKey> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets/public-key",
            owner, repo
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "with-env-cli")
            .send()
            .await
            .context("Failed to send request")?;

        let public_key: RepositoryPublicKey =
            response.json().await.context("Failed to parse response")?;

        Ok(public_key)
    }

    fn encrypt_secret(&self, public_key: &str, secret_value: &str) -> Result<String> {
        sodiumoxide::init().map_err(|_| anyhow::anyhow!("Failed to initialize sodiumoxide"))?;

        let public_key_bytes = BASE64_STANDARD
            .decode(public_key)
            .context("Failed to decode public key")?;

        let public_key = sodiumoxide::crypto::box_::PublicKey::from_slice(&public_key_bytes)
            .context("Invalid public key")?;

        let encrypted = sealedbox::seal(secret_value.as_bytes(), &public_key);

        Ok(BASE64_STANDARD.encode(encrypted))
    }
}

pub fn get_repository_info() -> Result<(String, String)> {
    let repo = git2::Repository::discover(".")
        .context("Not a git repository. Please run this command from within a git repository.")?;

    let remote = repo
        .find_remote("origin")
        .context("No 'origin' remote found")?;

    let url = remote.url().context("Remote URL not found")?;

    // Parse GitHub URL (supports both HTTPS and SSH formats)
    let (owner, repo_name) = parse_github_url(url)?;

    Ok((owner, repo_name))
}

fn parse_github_url(url: &str) -> Result<(String, String)> {
    // Handle SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let parts: Vec<&str> = url
            .trim_start_matches("git@github.com:")
            .split('/')
            .collect();
        if parts.len() >= 2 {
            let owner = parts[0].to_string();
            let repo = parts[1].trim_end_matches(".git").to_string();
            return Ok((owner, repo));
        }
    }

    // Handle HTTPS format: https://github.com/owner/repo.git
    if url.starts_with("https://github.com/") {
        let parts: Vec<&str> = url
            .trim_start_matches("https://github.com/")
            .split('/')
            .collect();
        if parts.len() >= 2 {
            let owner = parts[0].to_string();
            let repo = parts[1].trim_end_matches(".git").to_string();
            return Ok((owner, repo));
        }
    }

    anyhow::bail!("Invalid GitHub URL format: {}", url)
}
