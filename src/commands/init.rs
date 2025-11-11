use anyhow::Result;
use crate::config::Config;

pub async fn handle(org: String, audit_url: Option<String>, token: Option<String>) -> Result<()> {
    let config = Config {
        organization: org.clone(),
        audit_url,
        github_token: token,
    };

    config.save()?;

    println!("Configuration saved successfully!");
    println!("Organization: {}", org);
    
    if let Some(url) = &config.audit_url {
        println!("Audit URL: {}", url);
    }

    let config_path = Config::config_path()?;
    println!("Configuration file: {}", config_path.display());

    Ok(())
}
