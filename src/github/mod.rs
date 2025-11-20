use anyhow::{Context, Result};
use base64::prelude::*;
use crypto_box::{aead::Aead, PublicKey, SalsaBox, SecretKey};
use serde::{Deserialize, Serialize};
use urlencoding::encode as url_encode;

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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()?;

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
            owner,
            repo,
            url_encode(environment)
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
            owner,
            repo,
            url_encode(environment),
            secret_name
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
        // Get the environment-specific public key (not the repository-level key)
        let public_key = self
            .get_environment_public_key(owner, repo, environment)
            .await?;

        // Encrypt the secret value
        let encrypted_value = self.encrypt_secret(&public_key.key, secret_value)?;

        // Set the secret
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments/{}/secrets/{}",
            owner,
            repo,
            url_encode(environment),
            secret_name
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
            // Log full error for debugging but don't expose to user
            eprintln!(
                "DEBUG: GitHub API error response: {}",
                crate::security::redact_error_message(&body)
            );
            anyhow::bail!("Failed to set secret: HTTP {}", status.as_u16());
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
            owner,
            repo,
            url_encode(environment),
            secret_name
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
            // Log full error for debugging but don't expose to user
            eprintln!(
                "DEBUG: GitHub API error response: {}",
                crate::security::redact_error_message(&body)
            );
            anyhow::bail!("Failed to delete secret: HTTP {}", status.as_u16());
        }

        Ok(())
    }

    #[allow(dead_code)]
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

    async fn get_environment_public_key(
        &self,
        owner: &str,
        repo: &str,
        environment: &str,
    ) -> Result<RepositoryPublicKey> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/environments/{}/secrets/public-key",
            owner,
            repo,
            url_encode(environment)
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
        use blake2::{Blake2b512, Digest};

        let public_key_bytes = BASE64_STANDARD
            .decode(public_key)
            .context("Failed to decode public key")?;

        let recipient_pk = PublicKey::from(
            <[u8; 32]>::try_from(public_key_bytes.as_slice())
                .map_err(|_| anyhow::anyhow!("Invalid public key length"))?,
        );

        // Implement sealed box encryption (libsodium compatible)
        // 1. Generate ephemeral keypair
        let mut rng = rand::thread_rng();
        let ephemeral_sk = SecretKey::generate(&mut rng);
        let ephemeral_pk = ephemeral_sk.public_key();

        // 2. Compute nonce as BLAKE2b(ephemeral_pk || recipient_pk)[0..24]
        let mut hasher = Blake2b512::new();
        hasher.update(ephemeral_pk.as_bytes());
        hasher.update(recipient_pk.as_bytes());
        let hash = hasher.finalize();
        let nonce = crypto_box::Nonce::from_slice(&hash[0..24]);

        // 3. Encrypt using crypto_box
        let salsa_box = SalsaBox::new(&recipient_pk, &ephemeral_sk);
        let ciphertext = salsa_box
            .encrypt(nonce, secret_value.as_bytes())
            .map_err(|_| anyhow::anyhow!("Encryption failed"))?;

        // 4. Return ephemeral_pk || ciphertext
        let mut sealed = Vec::with_capacity(32 + ciphertext.len());
        sealed.extend_from_slice(ephemeral_pk.as_bytes());
        sealed.extend_from_slice(&ciphertext);

        Ok(BASE64_STANDARD.encode(sealed))
    }
}

/// Get repository information from git remote (current directory)
pub fn get_repository_info() -> Result<(String, String)> {
    let repo = git2::Repository::discover(".")
        .context("Not a git repository. Please run this command from within a git repository, or use --repo to specify a repository.")?;

    let remote = repo
        .find_remote("origin")
        .context("No 'origin' remote found")?;

    let url = remote.url().context("Remote URL not found")?;

    // Parse GitHub URL (supports both HTTPS and SSH formats)
    let (owner, repo_name) = parse_github_url(url)?;

    Ok((owner, repo_name))
}

/// Resolve repository information from either the --repo flag or git auto-detection
///
/// If repo_flag is provided, it should be in format:
/// - "owner/repo" - explicit owner and repo name
/// - "repo" - just repo name, owner will be taken from config
///
/// If repo_flag is None, auto-detects from current directory's git remote
pub fn resolve_repository_info(repo_flag: Option<String>) -> Result<(String, String)> {
    if let Some(repo_spec) = repo_flag {
        parse_repo_spec(&repo_spec)
    } else {
        get_repository_info()
    }
}

/// Parse repository specification from --repo flag
/// Formats supported:
/// - "owner/repo" - returns (owner, repo)
/// - "repo" - returns (org from config, repo)
fn parse_repo_spec(spec: &str) -> Result<(String, String)> {
    // Validate the repo spec format
    if spec.is_empty() {
        anyhow::bail!("Repository specification cannot be empty");
    }

    if spec.contains(char::is_whitespace) {
        anyhow::bail!(
            "Repository specification cannot contain whitespace: {}",
            spec
        );
    }

    // Check if it's in "owner/repo" format
    if let Some(slash_pos) = spec.find('/') {
        let owner = spec[..slash_pos].trim();
        let repo = spec[slash_pos + 1..].trim();

        if owner.is_empty() || repo.is_empty() {
            anyhow::bail!(
                "Invalid repository format '{}'. Use 'owner/repo' or just 'repo'",
                spec
            );
        }

        // Validate owner and repo names
        validate_github_name(owner, "owner")?;
        validate_github_name(repo, "repository")?;

        Ok((owner.to_string(), repo.to_string()))
    } else {
        // Just repo name provided, need to get owner from config
        validate_github_name(spec, "repository")?;

        let config = crate::config::Config::load().context(
            "Failed to load config. Run 'with-env init' first or use 'owner/repo' format.",
        )?;

        Ok((config.organization, spec.to_string()))
    }
}

/// Validate GitHub owner/repo name
/// GitHub allows alphanumeric characters, hyphens, underscores, and dots
fn validate_github_name(name: &str, label: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("{} name cannot be empty", label);
    }

    if name.len() > 100 {
        anyhow::bail!("{} name too long (max 100 characters): {}", label, name);
    }

    // GitHub allows alphanumeric, hyphens, underscores, and dots
    // but cannot start with a dot or hyphen
    if name.starts_with('.') || name.starts_with('-') {
        anyhow::bail!("{} name cannot start with '.' or '-': {}", label, name);
    }

    for ch in name.chars() {
        if !ch.is_alphanumeric() && ch != '-' && ch != '_' && ch != '.' {
            anyhow::bail!(
                "{} name '{}' contains invalid character '{}'. Only alphanumeric, hyphens, underscores, and dots are allowed.",
                label, name, ch
            );
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // parse_github_url() tests
    // ========================================================================

    #[test]
    fn test_parse_github_url_ssh_with_git() {
        let url = "git@github.com:octocat/Hello-World.git";
        let result = parse_github_url(url);
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "Hello-World");
    }

    #[test]
    fn test_parse_github_url_ssh_without_git() {
        let url = "git@github.com:octocat/Hello-World";
        let result = parse_github_url(url);
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "Hello-World");
    }

    #[test]
    fn test_parse_github_url_https_with_git() {
        let url = "https://github.com/octocat/Hello-World.git";
        let result = parse_github_url(url);
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "Hello-World");
    }

    #[test]
    fn test_parse_github_url_https_without_git() {
        let url = "https://github.com/octocat/Hello-World";
        let result = parse_github_url(url);
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "Hello-World");
    }

    #[test]
    fn test_parse_github_url_invalid_no_protocol() {
        let url = "github.com/octocat/Hello-World";
        let result = parse_github_url(url);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid GitHub URL format"));
    }

    #[test]
    fn test_parse_github_url_invalid_wrong_protocol() {
        let url = "http://github.com/octocat/Hello-World";
        let result = parse_github_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_url_ssh_missing_owner() {
        let url = "git@github.com:";
        let result = parse_github_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_url_https_missing_repo() {
        let url = "https://github.com/octocat";
        let result = parse_github_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_url_empty() {
        let result = parse_github_url("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_url_malformed() {
        let url = "not-a-valid-url";
        let result = parse_github_url(url);
        assert!(result.is_err());
    }

    // ========================================================================
    // validate_github_name() tests
    // ========================================================================

    #[test]
    fn test_validate_github_name_valid_simple() {
        assert!(validate_github_name("myrepo", "repository").is_ok());
        assert!(validate_github_name("owner123", "owner").is_ok());
    }

    #[test]
    fn test_validate_github_name_valid_with_hyphens() {
        assert!(validate_github_name("my-repo", "repository").is_ok());
        assert!(validate_github_name("hello-world-test", "repository").is_ok());
    }

    #[test]
    fn test_validate_github_name_valid_with_underscores() {
        assert!(validate_github_name("my_repo", "repository").is_ok());
        assert!(validate_github_name("hello_world_test", "repository").is_ok());
    }

    #[test]
    fn test_validate_github_name_valid_with_dots() {
        assert!(validate_github_name("my.repo", "repository").is_ok());
        assert!(validate_github_name("hello.world.test", "repository").is_ok());
    }

    #[test]
    fn test_validate_github_name_valid_complex() {
        assert!(validate_github_name("My_Repo-123.test", "repository").is_ok());
        assert!(validate_github_name("owner-123_test.name", "owner").is_ok());
    }

    #[test]
    fn test_validate_github_name_empty() {
        let result = validate_github_name("", "repository");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_github_name_too_long() {
        let long_name = "a".repeat(101);
        let result = validate_github_name(&long_name, "repository");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_validate_github_name_starts_with_dot() {
        let result = validate_github_name(".myrepo", "repository");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot start with"));
    }

    #[test]
    fn test_validate_github_name_starts_with_hyphen() {
        let result = validate_github_name("-myrepo", "repository");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot start with"));
    }

    #[test]
    fn test_validate_github_name_invalid_at_symbol() {
        let result = validate_github_name("my@repo", "repository");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("invalid character"));
        assert!(error_msg.contains("@"));
    }

    #[test]
    fn test_validate_github_name_invalid_slash() {
        let result = validate_github_name("my/repo", "repository");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("invalid character"));
        assert!(error_msg.contains("/"));
    }

    #[test]
    fn test_validate_github_name_invalid_space() {
        let result = validate_github_name("my repo", "repository");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid character"));
    }

    #[test]
    fn test_validate_github_name_max_length() {
        let max_name = "a".repeat(100);
        assert!(validate_github_name(&max_name, "repository").is_ok());
    }

    // ========================================================================
    // parse_repo_spec() tests
    // ========================================================================

    #[test]
    fn test_parse_repo_spec_full_format() {
        let result = parse_repo_spec("octocat/Hello-World");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "Hello-World");
    }

    #[test]
    fn test_parse_repo_spec_full_format_with_dots() {
        let result = parse_repo_spec("my-org/repo.name.test");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "my-org");
        assert_eq!(repo, "repo.name.test");
    }

    #[test]
    fn test_parse_repo_spec_empty() {
        let result = parse_repo_spec("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_parse_repo_spec_with_whitespace() {
        let result = parse_repo_spec("owner/repo name");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot contain whitespace"));
    }

    #[test]
    fn test_parse_repo_spec_multiple_slashes() {
        let result = parse_repo_spec("owner/repo/extra");
        assert!(result.is_err());
        // First slash will parse as owner/repo, but "repo/extra" will fail validation
    }

    #[test]
    fn test_parse_repo_spec_missing_owner() {
        let result = parse_repo_spec("/myrepo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid repository format"));
    }

    #[test]
    fn test_parse_repo_spec_missing_repo() {
        let result = parse_repo_spec("owner/");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid repository format"));
    }

    #[test]
    fn test_parse_repo_spec_invalid_owner_chars() {
        let result = parse_repo_spec("owner@bad/repo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid character"));
    }

    #[test]
    fn test_parse_repo_spec_invalid_repo_chars() {
        let result = parse_repo_spec("owner/repo@bad");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid character"));
    }

    #[test]
    fn test_parse_repo_spec_owner_starts_with_dot() {
        let result = parse_repo_spec(".owner/repo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot start with"));
    }

    #[test]
    fn test_parse_repo_spec_repo_starts_with_hyphen() {
        let result = parse_repo_spec("owner/-repo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot start with"));
    }

    // ========================================================================
    // resolve_repository_info() tests
    // ========================================================================

    #[test]
    fn test_resolve_repository_info_with_full_format() {
        let result = resolve_repository_info(Some("octocat/Hello-World".to_string()));
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "octocat");
        assert_eq!(repo, "Hello-World");
    }

    #[test]
    fn test_resolve_repository_info_with_invalid_format() {
        let result = resolve_repository_info(Some("".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_repository_info_with_invalid_chars() {
        let result = resolve_repository_info(Some("owner@bad/repo".to_string()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid character"));
    }

    // Note: Testing with None (auto-detection from git) and short format
    // (requires config) would need more complex test setup with temp directories
    // and mock configs. These are tested in integration tests instead.

    // ========================================================================
    // encrypt_secret() tests
    // ========================================================================

    #[test]
    fn test_encrypt_secret_with_valid_key() {
        let client = GitHubClient::new("fake-token").unwrap();

        // Generate a valid Curve25519 public key for testing
        let mut rng = rand::thread_rng();
        let secret_key = crypto_box::SecretKey::generate(&mut rng);
        let public_key = secret_key.public_key();
        let public_key_base64 = BASE64_STANDARD.encode(public_key.as_bytes());

        let result = client.encrypt_secret(&public_key_base64, "my-secret-value");
        assert!(result.is_ok());

        let encrypted = result.unwrap();
        // Encrypted value should be base64 encoded
        assert!(BASE64_STANDARD.decode(&encrypted).is_ok());
        // Should not contain the original secret
        assert!(!encrypted.contains("my-secret-value"));
    }

    #[test]
    fn test_encrypt_secret_with_invalid_base64() {
        let client = GitHubClient::new("fake-token").unwrap();
        let result = client.encrypt_secret("not-valid-base64!@#", "my-secret");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to decode public key"));
    }

    #[test]
    fn test_encrypt_secret_with_invalid_key_length() {
        let client = GitHubClient::new("fake-token").unwrap();
        // Valid base64 but wrong length for a libsodium public key
        let short_key = BASE64_STANDARD.encode(b"tooshort");
        let result = client.encrypt_secret(&short_key, "my-secret");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid public key"));
    }
}
