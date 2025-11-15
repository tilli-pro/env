use crate::{
    audit,
    config::Config,
    github::{resolve_repository_info, GitHubClient},
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub async fn handle(
    environment: String,
    command: Vec<String>,
    repo_flag: Option<String>,
    no_exec: bool,
) -> Result<()> {
    // Validate environment name to prevent path traversal
    crate::validation::validate_environment_name(&environment)?;

    if command.is_empty() {
        anyhow::bail!("No command provided");
    }

    // Validate command name for security
    crate::validation::validate_command_name(&command[0])
        .context("Command validation failed. This prevents potential security issues.")?;

    // Load local environment file first
    let env_vars = load_local_env_file(&environment)?;

    if env_vars.is_empty() {
        let safe_name = crate::validation::sanitize_for_filepath(&environment);
        eprintln!("⚠️  WARNING: No local environment file found.");
        eprintln!(
            "   Create a file at ~/.config/with-env/envs/{}.env with your environment variables.",
            safe_name
        );
        eprintln!("   Format: KEY=VALUE (one per line)");
        eprintln!();
    }

    // Try to get GitHub secrets list (but don't fail if GitHub API is unavailable)
    let github_secrets = match try_list_github_secrets(&environment, repo_flag.clone()).await {
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

    // Log audit event if we have GitHub access (never ignore failures)
    if github_secrets.is_some() {
        if let Ok(config) = Config::load() {
            if let Ok((owner, repo)) = resolve_repository_info(repo_flag) {
                audit::log_event(
                    "run_with_env".to_string(),
                    format!("{}/{}", owner, repo),
                    environment.clone(),
                    None,
                    config.audit_url.as_deref(),
                    config.get_audit_token(),
                )
                .await?;
            }
        }
    }

    println!("Running: {}", command.join(" "));
    println!();

    // Build the command
    let mut cmd = Command::new(&command[0]);

    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    // Add environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Execute: either replace current process (exec) or spawn child process
    if !no_exec {
        // Default: exec replacement (Unix only - replaces current process)
        // On Windows, falls back to spawn behavior
        exec_command(cmd)
    } else {
        // --no-exec: spawn as child process and wait (original behavior)
        spawn_and_wait(cmd)
    }
}

/// Execute command by replacing the current process (Unix exec)
/// On Unix: replaces the with-env process entirely (no child process)
/// On Windows: falls back to spawn behavior (exec not available)
#[cfg(unix)]
fn exec_command(mut cmd: Command) -> Result<()> {
    // exec() replaces the current process - this never returns on success
    let err = cmd.exec();
    // If we get here, exec failed
    Err(anyhow::anyhow!("Failed to exec command: {}", err))
}

#[cfg(not(unix))]
fn exec_command(cmd: Command) -> Result<()> {
    // Windows doesn't have exec - fall back to spawn behavior
    spawn_and_wait(cmd)
}

/// Spawn command as child process and wait for completion
fn spawn_and_wait(mut cmd: Command) -> Result<()> {
    let status = cmd.status().context("Failed to execute command")?;

    if !status.success() {
        anyhow::bail!("Command exited with status: {}", status);
    }

    Ok(())
}

async fn try_list_github_secrets(
    environment: &str,
    repo_flag: Option<String>,
) -> Result<Vec<crate::github::Secret>> {
    let config = Config::load()?;
    let token = config.get_github_token()?;
    let (owner, repo) = resolve_repository_info(repo_flag)?;

    let client = GitHubClient::new(&token)?;
    client.list_secrets(&owner, &repo, environment).await
}

fn load_local_env_file(environment: &str) -> Result<HashMap<String, String>> {
    let config_dir = dirs::config_dir().context("Failed to determine config directory")?;

    // Sanitize environment name for safe file path usage
    let safe_name = crate::validation::sanitize_for_filepath(environment);

    let env_file = config_dir
        .join("with-env")
        .join("envs")
        .join(format!("{}.env", safe_name));

    if !env_file.exists() {
        return Ok(HashMap::new());
    }

    // Check file permissions before reading
    crate::security::check_file_permissions(&env_file)?;

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

            // Remove quotes if present (handle both single and double quotes properly)
            let value = strip_quotes(&value);

            env_vars.insert(key, value);
        }
    }

    Ok(env_vars)
}

fn strip_quotes(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return value[1..value.len() - 1].to_string();
    }
    value.to_string()
}
