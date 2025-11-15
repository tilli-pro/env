# env

Environment and secret management for tilli products, backed by GitHub secrets.

A Rust CLI tool for managing GitHub repository secrets and environments with audit logging capabilities.

## Features

- 🔐 Manage GitHub repository secrets and environments
- 📝 Audit logging for all secret operations
- 🚀 Run commands with environment variables from GitHub environments
- 🏢 Organization-level configuration
- 🔒 Secure secret encryption using libsodium
- 🔑 Secure token storage using system keyring
- 🛡️ Input validation to prevent injection attacks
- ✅ File permission checks and warnings

## Security Improvements (v0.1.1)

This version includes significant security enhancements:

- ✅ **Fixed command injection vulnerability** - Commands are now validated
- ✅ **Secure secret input** - Secrets are read via stdin (hidden) instead of CLI args
- ✅ **Keyring token storage** - GitHub tokens stored in system keyring instead of plaintext
- ✅ **Path traversal prevention** - Environment names are validated
- ✅ **HTTPS enforcement** - Audit URLs must use HTTPS
- ✅ **Audit authentication** - Support for audit endpoint authentication
- ✅ **Error message sanitization** - Sensitive data redacted from errors
- ✅ **File permission checks** - Warns about insecure file permissions
- ✅ **Request timeouts** - All HTTP requests have proper timeouts
- ✅ **Input validation** - All inputs validated to prevent injection

See [SECURITY_AUDIT.md](SECURITY_AUDIT.md) for the complete security audit report.

## Installation

### From Source

```bash
cargo build --release
cp target/release/with-env /usr/local/bin/
```

## Usage

### Repository Selection

By default, the tool auto-detects the repository from the current directory's git remote. You can override this with the `--repo` flag:

```bash
# Use current directory's git repository (auto-detect)
with-env list-envs

# Specify repository explicitly (uses organization from config)
with-env --repo my-repo list-envs

# Specify repository with owner
with-env --repo owner/repo list-envs
```

The `--repo` flag accepts two formats:
- `repo` - Uses the organization from your config file
- `owner/repo` - Explicitly specifies both owner and repository

This is useful when:
- You're not in a git repository directory
- You want to manage secrets for a different repository
- You're writing scripts that need to work across multiple repositories

### Initialize Configuration

First, initialize the tool with your GitHub organization:

```bash
# With token stored in system keyring (recommended)
with-env init --org your-organization --token ghp_xxxxxxxxxxxxx

# Without token (will use GITHUB_TOKEN env var)
with-env init --org your-organization

# With audit logging
with-env init --org your-organization --audit-url https://audit.example.com/events
```

This will:
- Create a configuration file at `~/.config/with-env/config.toml`
- Store your GitHub token securely in the system keyring (if provided)
- Set secure file permissions (0600) automatically

**Security Note:** Tokens are now stored in your system's secure keyring instead of plaintext files. You can also use the `GITHUB_TOKEN` environment variable.

### List Environments

List all environments for a repository:

```bash
# Current directory's repository
with-env list-envs

# Specific repository
with-env --repo my-repo list-envs
with-env --repo owner/repo list-envs
```

### List Secrets

List all secrets in a specific environment:

```bash
# Current directory's repository
with-env list-secrets production

# Specific repository
with-env --repo my-repo list-secrets production
with-env --repo owner/repo list-secrets production
```

**Note:** Environment names must contain only alphanumeric characters, hyphens, and underscores.

### Get Secret Information

Get metadata about a specific secret (note: values cannot be retrieved via GitHub API):

```bash
with-env get-secret production DATABASE_URL
```

**Note:** Secret names must be uppercase with underscores only (e.g., API_KEY, DATABASE_URL).

### Set a Secret

Set a secret value for an environment (secure input methods):

```bash
# Interactive input (recommended - hides your typing)
with-env set-secret production API_KEY
# You'll be prompted to enter the secret value (hidden)

# From a file
with-env set-secret production API_KEY --from-file secret.txt

# From stdin (for scripts)
echo "my-secret-value" | with-env set-secret production API_KEY

# For a specific repository
with-env --repo my-repo set-secret production API_KEY
with-env --repo owner/repo set-secret production API_KEY --from-file secret.txt
```

**⚠️ Security Warning:** Never pass secrets as command-line arguments! They would be visible in:
- Process lists (`ps aux`)
- Shell history (`.bash_history`)
- System logs

### Delete a Secret

Delete a secret from an environment:

```bash
with-env delete-secret production OLD_SECRET
```

### Run Commands with Environment Variables

Run a command with environment variables from a specific environment:

```bash
# Current directory's repository
with-env run production npm start

# Shorthand (uses "default" environment)
with-env npm start

# Specific repository
with-env --repo my-repo run production npm start
with-env --repo owner/repo npm start
```

**Security Note:** Commands are validated to prevent injection attacks. Commands with shell metacharacters (`|`, `&`, `;`, etc.) are rejected.

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

The tool will automatically:
- Load these variables when running commands
- Check file permissions and warn if insecure
- Validate environment file syntax

**Security Best Practice:** Set secure permissions on environment files:

```bash
chmod 600 ~/.config/with-env/envs/*.env
```

## Audit Logging

When an audit URL is configured, the tool automatically logs the following events:

- `list_secrets` - When secrets are listed
- `get_secret` - When secret metadata is retrieved
- `set_secret` - When a secret is created or updated
- `delete_secret` - When a secret is deleted
- `run_with_env` - When a command is run with environment variables

### Audit Configuration

Configure audit logging during initialization:

```bash
with-env init --org your-org --audit-url https://audit.example.com/events
```

**Security Requirements:**
- Audit URL **must** use HTTPS (enforced)
- Audit endpoint should require authentication (use audit token)
- Audit failures will now cause operations to fail (no silent failures)

Audit events include:
- Timestamp (RFC3339 format)
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
audit_url = "https://your-audit-server.com/events"  # Optional
audit_token = "your-audit-token"  # Optional
```

**Note:** GitHub tokens are no longer stored in the config file. They are stored securely in the system keyring or provided via `GITHUB_TOKEN` environment variable.

## Security Considerations

### Best Practices

1. **Token Storage**
   - Use system keyring (preferred): `with-env init --token YOUR_TOKEN`
   - Or use environment variable: `export GITHUB_TOKEN=ghp_xxx`
   - Never commit tokens to version control

2. **File Permissions**
   - Config file: `chmod 600 ~/.config/with-env/config.toml`
   - Env files: `chmod 600 ~/.config/with-env/envs/*.env`
   - Tool will warn you if permissions are insecure

3. **Secret Management**
   - Always use interactive input or files for secrets
   - Never pass secrets as CLI arguments
   - Use `--from-file` for automation

4. **Audit Logging**
   - Always use HTTPS for audit URLs
   - Configure audit authentication tokens
   - Monitor audit logs for suspicious activity

5. **Input Validation**
   - Environment names: only alphanumeric, hyphens, underscores
   - Secret names: only uppercase letters, numbers, underscores
   - Commands: automatically validated for safety

### Security Features

- ✅ Secrets encrypted using libsodium before GitHub upload
- ✅ Tokens stored in system keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- ✅ All inputs validated to prevent injection attacks
- ✅ HTTPS enforced for audit endpoints
- ✅ Request timeouts to prevent hanging
- ✅ Sensitive data redacted from error messages
- ✅ File permission checks with warnings
- ✅ Command validation to prevent shell injection

### Known Limitations

- GitHub API does not allow retrieving secret values
- Local environment files must be kept in sync manually
- Secrets must be managed in two places:
  1. GitHub (for CI/CD)
  2. Local files (for command execution)

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
cargo clippy -- -D warnings
```

## Security Vulnerability Reporting

See [SECURITY.md](SECURITY.md) for details on reporting security vulnerabilities.

For the complete security audit and remediation details, see [SECURITY_AUDIT.md](SECURITY_AUDIT.md).

## License

See [LICENSE](LICENSE) file for details.
