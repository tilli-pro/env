use crate::{
    audit,
    config::Config,
    github::{resolve_repository_info, GitHubClient},
};
use anyhow::{Context, Result};

pub async fn handle(
    environment: String,
    secret: String,
    from_file: Option<String>,
    repo_flag: Option<String>,
) -> Result<()> {
    // Validate inputs to prevent injection attacks
    crate::validation::validate_environment_name(&environment)?;
    crate::validation::validate_secret_name(&secret)?;

    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = resolve_repository_info(repo_flag)?;

    // Read secret value securely
    let value = if let Some(file_path) = from_file {
        // Read from file
        std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read secret from file: {}", file_path))?
            .trim()
            .to_string()
    } else {
        // Read from stdin (hidden input)
        println!("Enter secret value for '{}' (input hidden):", secret);
        rpassword::read_password().context("Failed to read secret value")?
    };

    // Validate secret value
    crate::validation::validate_secret_value(&value)?;

    let client = GitHubClient::new(&token)?;
    client
        .set_secret(&owner, &repo, &environment, &secret, &value)
        .await?;

    // Log audit event (never ignore failures)
    audit::log_event(
        "set_secret".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        Some(secret.clone()),
        config.audit_url.as_deref(),
        config.get_audit_token(),
    )
    .await?;

    println!(
        "✓ Secret '{}' set successfully in environment '{}'",
        secret, environment
    );

    Ok(())
}
