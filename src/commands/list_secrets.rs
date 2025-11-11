use anyhow::Result;
use crate::{audit, config::Config, github::{get_repository_info, GitHubClient}};

pub async fn handle(environment: String) -> Result<()> {
    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = get_repository_info()?;

    let client = GitHubClient::new(&token)?;
    let secrets = client.list_secrets(&owner, &repo, &environment).await?;

    // Log audit event
    audit::log_event(
        "list_secrets".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        None,
        config.audit_url.as_deref(),
    )
    .await?;

    if secrets.is_empty() {
        println!("No secrets found in environment '{}'", environment);
    } else {
        println!("Secrets in environment '{}':", environment);
        for secret in secrets {
            println!("  - {} (updated: {})", secret.name, secret.updated_at);
        }
    }

    Ok(())
}
