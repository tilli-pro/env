use crate::{
    audit,
    config::Config,
    github::{get_repository_info, GitHubClient},
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;

pub async fn handle(environment: String, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command provided");
    }

    // Load local environment file first
    let env_vars = load_local_env_file(&environment)?;

    if env_vars.is_empty() {
        eprintln!("Warning: No local environment file found.");
        eprintln!(
            "Create a file at ~/.config/with-env/envs/{}.env with your environment variables.",
            environment
        );
        eprintln!("Format: KEY=VALUE (one per line)");
        eprintln!();
    }

    // Try to get GitHub secrets list (but don't fail if GitHub API is unavailable)
    let github_secrets = match try_list_github_secrets(&environment).await {
        Ok(secrets) => {
            if !secrets.is_empty() {
                println!(
                    "Available secrets in GitHub environment '{}' (values in local env file):",
                    environment
                );
                for secret in &secrets {
                    let in_local = if env_vars.contains_key(&secret.name) {
                        "✓"
                    } else {
                        "✗"
                    };
                    println!("  {} {}", in_local, secret.name);
                }
                println!();
            }
            Some(secrets)
        }
        Err(e) => {
            eprintln!("Note: Could not fetch GitHub secrets: {}", e);
            eprintln!("Continuing with local environment variables only.");
            eprintln!();
            None
        }
    };

    // Log audit event if we have GitHub access
    if github_secrets.is_some() {
        if let Ok(config) = Config::load() {
            if let Ok((owner, repo)) = get_repository_info() {
                let _ = audit::log_event(
                    "run_with_env".to_string(),
                    format!("{}/{}", owner, repo),
                    environment.clone(),
                    None,
                    config.audit_url.as_deref(),
                )
                .await;
            }
        }
    }

    println!("Running: {}", command.join(" "));
    println!();

    // Execute the command with environment variables
    let mut cmd = Command::new(&command[0]);

    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    // Add environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let status = cmd.status().context("Failed to execute command")?;

    if !status.success() {
        anyhow::bail!("Command exited with status: {}", status);
    }

    Ok(())
}

async fn try_list_github_secrets(environment: &str) -> Result<Vec<crate::github::Secret>> {
    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = get_repository_info()?;

    let client = GitHubClient::new(&token)?;
    client.list_secrets(&owner, &repo, environment).await
}

fn load_local_env_file(environment: &str) -> Result<HashMap<String, String>> {
    let config_dir = dirs::config_dir().context("Failed to determine config directory")?;

    let env_file = config_dir
        .join("with-env")
        .join("envs")
        .join(format!("{}.env", environment));

    if !env_file.exists() {
        return Ok(HashMap::new());
    }

    let contents = std::fs::read_to_string(&env_file).context("Failed to read environment file")?;

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
