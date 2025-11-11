# env

Environment and secret management for tilli products, backed by GitHub secrets.

A Rust CLI tool for managing GitHub repository secrets and environments with audit logging capabilities.

## Features

- 🔐 Manage GitHub repository secrets and environments
- 📝 Audit logging for all secret operations
- 🚀 Run commands with environment variables from GitHub environments
- 🏢 Organization-level configuration
- 🔒 Secure secret encryption using libsodium

## Installation

### From Source

```bash
cargo build --release
cp target/release/with-env /usr/local/bin/
```

## Usage

### Initialize Configuration

First, initialize the tool with your GitHub organization and optional audit URL:

```bash
with-env init --org your-organization --audit-url https://your-audit-server.com/events
```

This will create a configuration file at `~/.config/with-env/config.toml`.

You can also optionally provide a GitHub token:

```bash
with-env init --org your-organization --token ghp_xxxxxxxxxxxxx
```

If no token is provided, the tool will use the `GITHUB_TOKEN` environment variable.

### List Environments

List all environments for the current repository:

```bash
with-env list-envs
```

### List Secrets

List all secrets in a specific environment:

```bash
with-env list-secrets production
```

### Get Secret Information

Get metadata about a specific secret (note: values cannot be retrieved via GitHub API):

```bash
with-env get-secret production DATABASE_URL
```

### Set a Secret

Set a secret value for an environment:

```bash
with-env set-secret production API_KEY "your-secret-value"
```

### Delete a Secret

Delete a secret from an environment:

```bash
with-env delete-secret production OLD_SECRET
```

### Run Commands with Environment Variables

Run a command with environment variables from a specific environment:

```bash
with-env run production npm start
```

Or use the shorthand (uses "default" environment):

```bash
with-env npm start
```

#### Local Environment Files

Since GitHub API doesn't allow retrieving secret values, you need to create local environment files at:

```
~/.config/with-env/envs/<environment-name>.env
```

Example `~/.config/with-env/envs/production.env`:

```env
DATABASE_URL=postgresql://localhost/mydb
API_KEY=abc123
SECRET_TOKEN=xyz789
```

The tool will load these variables and use them when running commands.

## Audit Logging

When an audit URL is configured, the tool automatically logs the following events:

- `list_secrets` - When secrets are listed
- `get_secret` - When secret metadata is retrieved
- `set_secret` - When a secret is created or updated
- `delete_secret` - When a secret is deleted
- `run_with_env` - When a command is run with environment variables

Audit events include:
- Timestamp
- Action type
- Repository (owner/name)
- Environment name
- Secret name (if applicable)
- User who performed the action

Example audit event:

```json
{
  "timestamp": "2025-11-11T19:10:39Z",
  "action": "set_secret",
  "repository": "tilli-pro/env",
  "environment": "production",
  "secret_name": "API_KEY",
  "user": "username"
}
```

## Requirements

- Rust 1.70 or later
- GitHub Personal Access Token with appropriate permissions:
  - `repo` scope for private repositories
  - `admin:org` scope for organization secrets
- Git repository with GitHub remote

## Configuration File

The configuration file is stored at `~/.config/with-env/config.toml`:

```toml
organization = "your-organization"
audit_url = "https://your-audit-server.com/events"
github_token = "ghp_xxxxxxxxxxxxx"  # Optional
```

## Security Considerations

- Store your GitHub token securely (use environment variables when possible)
- Use appropriate GitHub token scopes (principle of least privilege)
- Ensure audit URL endpoints are secured with authentication
- Local environment files contain sensitive data - protect them appropriately
- Secrets are encrypted using libsodium before being sent to GitHub

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Format Code

```bash
cargo fmt
```

### Lint

```bash
cargo clippy
```

## License

See [LICENSE](LICENSE) file for details.

