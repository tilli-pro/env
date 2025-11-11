# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Security Features

### Secret Encryption

All secrets are encrypted using libsodium (via sodiumoxide) before being sent to GitHub. The tool uses:

- **Sealed boxes**: Public-key authenticated encryption
- **Base64 encoding**: For safe transmission
- **GitHub's public key**: Retrieved dynamically for each repository

### Token Management

- GitHub tokens are stored in the configuration file at `~/.config/with-env/config.toml`
- Alternatively, use the `GITHUB_TOKEN` environment variable (recommended)
- Never commit tokens to version control

### Local Environment Files

- Environment files are stored at `~/.config/with-env/envs/<environment>.env`
- These files contain sensitive data - protect them with appropriate file permissions
- Consider using `chmod 600` on environment files

### Audit Logging

All secret operations can be logged to an external audit URL:
- Secret listing
- Secret retrieval
- Secret creation/update
- Secret deletion
- Command execution with environment variables

## Reporting a Vulnerability

If you discover a security vulnerability, please follow these steps:

1. **Do NOT** open a public issue
2. Email the maintainers privately at: [security contact email]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We aim to respond to security reports within 48 hours and will work with you to understand and address the issue promptly.

## Best Practices

### For Users

1. **Protect your GitHub token**
   - Use environment variables instead of storing in config files
   - Use tokens with minimal required permissions
   - Rotate tokens regularly

2. **Secure your environment files**
   ```bash
   chmod 600 ~/.config/with-env/envs/*.env
   ```

3. **Enable audit logging**
   - Configure an audit URL to track all secret operations
   - Monitor audit logs for suspicious activity

4. **Regular updates**
   - Keep the tool updated to receive security patches
   - Monitor security advisories

### For Developers

1. **Never log sensitive data**
   - Avoid logging secret values
   - Redact sensitive information in error messages

2. **Validate inputs**
   - Sanitize user inputs
   - Validate URLs and file paths

3. **Dependencies**
   - Keep dependencies updated
   - Use `cargo audit` to check for known vulnerabilities
   - Review dependency licenses

4. **Code review**
   - All code changes require review
   - Security-sensitive changes require extra scrutiny

## Security Considerations

### GitHub API Limitations

- GitHub API does not allow retrieving secret values
- Local environment files are used as a workaround
- This means secrets must be managed in two places:
  1. GitHub (for CI/CD)
  2. Local files (for command execution)

### Risk Mitigation

- Always use HTTPS for API communication
- Verify SSL certificates
- Use authenticated encryption for secrets
- Implement rate limiting for API calls
- Log all security-relevant operations

## Security Checklist

Before using in production:

- [ ] Configure audit logging
- [ ] Set proper file permissions on config and env files
- [ ] Use environment variables for GitHub token
- [ ] Review GitHub token permissions
- [ ] Enable monitoring of audit logs
- [ ] Establish token rotation policy
- [ ] Document security procedures
- [ ] Train team on security best practices

## Acknowledgments

We appreciate the security research community's efforts in responsibly disclosing vulnerabilities. Security researchers who report valid vulnerabilities will be credited (with their permission) in our changelog.
