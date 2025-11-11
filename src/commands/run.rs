use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;
use crate::{audit, config::Config, github::{get_repository_info, GitHubClient}};

pub async fn handle(environment: String, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command provided");
    }

    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = get_repository_info()?;

    let client = GitHubClient::new(&token)?;
    let secrets = client.list_secrets(&owner, &repo, &environment).await?;

    // Note: GitHub API doesn't allow retrieving secret values
    // In a real implementation, you would need to store secrets locally or use a different approach
    // For now, we'll demonstrate the command execution structure
    
    // Log audit event
    audit::log_event(
        "run_with_env".to_string(),
        format!("{}/{}", owner, repo),
        environment.clone(),
        None,
        config.audit_url.as_deref(),
    )
    .await?;

    println!("Running command with environment '{}': {}", environment, command.join(" "));
    println!("Note: Secret values must be stored locally. GitHub API does not allow retrieving secret values.");
    println!("Available secrets in environment:");
    for secret in &secrets {
        println!("  - {}", secret.name);
    }

    // Load local environment file if it exists
    let env_vars = load_local_env_file(&environment)?;

    if env_vars.is_empty() {
        eprintln!("\nWarning: No local environment file found.");
        eprintln!("Create a file at ~/.config/with-env/envs/{}.env with your environment variables.", environment);
        eprintln!("Format: KEY=VALUE (one per line)");
    }

    // Execute the command with environment variables
    let mut cmd = Command::new(&command[0]);
    
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    // Add environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let status = cmd
        .status()
        .context("Failed to execute command")?;

    if !status.success() {
        anyhow::bail!("Command exited with status: {}", status);
    }

    Ok(())
}

fn load_local_env_file(environment: &str) -> Result<HashMap<String, String>> {
    let config_dir = dirs::config_dir()
        .context("Failed to determine config directory")?;
    
    let env_file = config_dir
        .join("with-env")
        .join("envs")
        .join(format!("{}.env", environment));

    if !env_file.exists() {
        return Ok(HashMap::new());
    }

    let contents = std::fs::read_to_string(&env_file)
        .context("Failed to read environment file")?;

    let mut env_vars = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            
            // Remove quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value[1..value.len() - 1].to_string()
            } else {
                value
            };

            env_vars.insert(key, value);
        }
    }

    Ok(env_vars)
}
