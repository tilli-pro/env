use anyhow::Result;
use crate::{audit, config::Config, github::{get_repository_info, GitHubClient}};

pub async fn handle(environment: String, secret: String, value: String) -> Result<()> {
    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = get_repository_info()?;

    let client = GitHubClient::new(&token)?;
    client.set_secret(&owner, &repo, &environment, &secret, &value).await?;

    // Log audit event
    audit::log_event(
        "set_secret".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        Some(secret.clone()),
        config.audit_url.as_deref(),
    )
    .await?;

    println!("Secret '{}' set successfully in environment '{}'", secret, environment);

    Ok(())
}
