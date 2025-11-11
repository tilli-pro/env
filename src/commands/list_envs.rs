use crate::{
    config::Config,
    github::{get_repository_info, GitHubClient},
};
use anyhow::Result;

pub async fn handle() -> Result<()> {
    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = get_repository_info()?;

    let client = GitHubClient::new(&token)?;
    let environments = client.list_environments(&owner, &repo).await?;

    if environments.is_empty() {
        println!("No environments found for {}/{}", owner, repo);
    } else {
        println!("Environments for {}/{}:", owner, repo);
        for env in environments {
            println!("  - {}", env);
        }
    }

    Ok(())
}
