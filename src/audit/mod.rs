use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub action: String,
    pub repository: String,
    pub environment: String,
    pub secret_name: Option<String>,
    pub user: String,
}

impl AuditEvent {
    pub fn new(
        action: String,
        repository: String,
        environment: String,
        secret_name: Option<String>,
    ) -> Self {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            timestamp: Utc::now().to_rfc3339(),
            action,
            repository,
            environment,
            secret_name,
            user,
        }
    }

    pub async fn send(&self, audit_url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        
        client
            .post(audit_url)
            .json(self)
            .send()
            .await
            .context("Failed to send audit event")?;
        
        Ok(())
    }
}

pub async fn log_event(
    action: String,
    repository: String,
    environment: String,
    secret_name: Option<String>,
    audit_url: Option<&str>,
) -> Result<()> {
    let event = AuditEvent::new(action, repository, environment, secret_name);
    
    if let Some(url) = audit_url {
        event.send(url).await?;
    }
    
    Ok(())
}
