use crate::config::Config;
use anyhow::{Context, Result};

pub async fn handle(org: String, audit_url: Option<String>, token: Option<String>) -> Result<()> {
    // Validate audit URL if provided
    if let Some(ref url) = audit_url {
        crate::validation::validate_https_url(url)
            .context("Invalid audit URL")?;
    }

    // Prompt for audit token if audit URL is provided
    let audit_token = if audit_url.is_some() {
        println!("Audit URL configured. Do you want to set an audit token? (y/N): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("y") {
            println!("Enter audit token (hidden): ");
            Some(rpassword::read_password()?)
        } else {
            None
        }
    } else {
        None
    };

    let config = Config {
        organization: org.clone(),
        audit_url,
        audit_token,
        github_token: None, // Never store in config anymore
    };

    config.save()?;

    println!("✓ Configuration saved successfully!");
    println!("  Organization: {}", org);

    if let Some(url) = &config.audit_url {
        println!("  Audit URL: {}", url);
    }

    // Handle GitHub token storage
    if let Some(token_value) = token {
        match Config::store_token_in_keyring(&token_value) {
            Ok(_) => {
                println!("✓ GitHub token stored in system keyring");
                println!();
                println!("⚠️  Note: Keyring storage may not work on unsigned binaries on macOS.");
                println!("   If you encounter issues, use: export GITHUB_TOKEN=$(gh auth token)");
            }
            Err(e) => {
                eprintln!("⚠️  Failed to store token in keyring: {}", e);
                eprintln!();
                eprintln!("Alternative: Set the GITHUB_TOKEN environment variable:");
                eprintln!("   export GITHUB_TOKEN=$(gh auth token)");
                eprintln!();
                eprintln!("Or add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
                eprintln!("   export GITHUB_TOKEN=\"your-token-here\"");
            }
        }
    } else {
        println!();
        println!("GitHub token not provided.");
        println!();
        println!("✓ The tool will automatically use your GitHub CLI authentication if available.");
        println!();
        println!("If not authenticated with GitHub CLI:");
        println!("   gh auth login");
        println!();
        println!("Or set GITHUB_TOKEN environment variable:");
        println!("   export GITHUB_TOKEN=$(gh auth token)");
    }

    let config_path = Config::config_path()?;
    println!();
    println!("Configuration file: {}", config_path.display());

    // Check and warn about permissions
    crate::security::check_file_permissions(&config_path)?;

    Ok(())
}
