use crate::{
    audit,
    config::Config,
    github::{resolve_repository_info, GitHubClient},
};
use anyhow::Result;

pub async fn handle(environment: String, secret: String, repo_flag: Option<String>) -> Result<()> {
    // Validate inputs to prevent injection attacks
    crate::validation::validate_environment_name(&environment)?;
    crate::validation::validate_secret_name(&secret)?;

    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = resolve_repository_info(repo_flag)?;

    let client = GitHubClient::new(&token)?;
    let secret_info = client
        .get_secret(&owner, &repo, &environment, &secret)
        .await?;

    // Log audit event (never ignore failures)
    audit::log_event(
        "get_secret".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        Some(secret.clone()),
        config.audit_url.as_deref(),
        config.get_audit_token(),
    )
    .await?;

    println!("Secret: {}", secret_info.name);
    println!("Created: {}", secret_info.created_at);
    println!("Updated: {}", secret_info.updated_at);
    println!("\nℹ️  Note: Secret values cannot be retrieved from GitHub API for security reasons.");

    Ok(())
}
