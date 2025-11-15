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

    pub async fn send(&self, audit_url: &str, audit_token: Option<&str>) -> Result<()> {
        // Validate HTTPS
        crate::validation::validate_https_url(audit_url)
            .context("Audit URL must use HTTPS for security")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;

        let mut request = client.post(audit_url).json(self);

        // Add authentication if provided
        if let Some(token) = audit_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to send audit event")?;

        if !response.status().is_success() {
            let status = response.status();
            anyhow::bail!("Audit server returned error: HTTP {}", status.as_u16());
        }

        Ok(())
    }
}

pub async fn log_event(
    action: String,
    repository: String,
    environment: String,
    secret_name: Option<String>,
    audit_url: Option<&str>,
    audit_token: Option<&str>,
) -> Result<()> {
    let event = AuditEvent::new(action, repository, environment, secret_name);

    if let Some(url) = audit_url {
        // Never ignore audit failures - they indicate a security issue
        event.send(url, audit_token).await.context("Failed to log audit event. This is a security concern.")?;
    }

    Ok(())
}
