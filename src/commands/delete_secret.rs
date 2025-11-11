use crate::{
    audit,
    config::Config,
    github::{get_repository_info, GitHubClient},
};
use anyhow::Result;

pub async fn handle(environment: String, secret: String) -> Result<()> {
    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = get_repository_info()?;

    let client = GitHubClient::new(&token)?;
    client
        .delete_secret(&owner, &repo, &environment, &secret)
        .await?;

    // Log audit event
    audit::log_event(
        "delete_secret".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        Some(secret.clone()),
        config.audit_url.as_deref(),
    )
    .await?;

    println!(
        "Secret '{}' deleted successfully from environment '{}'",
        secret, environment
    );

    Ok(())
}
