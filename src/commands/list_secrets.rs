use crate::{
    audit,
    config::Config,
    github::{resolve_repository_info, GitHubClient},
};
use anyhow::Result;

pub async fn handle(environment: String, repo_flag: Option<String>) -> Result<()> {
    // Validate environment name to prevent injection attacks
    crate::validation::validate_environment_name(&environment)?;

    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = resolve_repository_info(repo_flag)?;

    let client = GitHubClient::new(&token)?;
    let secrets = client.list_secrets(&owner, &repo, &environment).await?;

    // Log audit event (never ignore failures)
    audit::log_event(
        "list_secrets".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        None,
        config.audit_url.as_deref(),
        config.get_audit_token(),
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
