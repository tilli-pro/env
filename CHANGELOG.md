# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-11-14

### Added
- **Exec replacement mode** - Commands now run via exec() on Unix systems by default
  - Parent process is replaced entirely (like `env` or `exec` command)
  - No child process overhead
  - Added `--no-exec` flag to opt into old child process behavior
  - Windows automatically falls back to spawn behavior
- **Flexible environment name validation** - Support for special characters in environment names
  - Now supports: @, /, parentheses, spaces, and other printable characters
  - URL encoding for GitHub API calls to handle special characters safely
  - File path sanitization for local .env files
  - Example: `@tilli-pro/api (env:dev)` is now a valid environment name
- **Automatic GitHub CLI integration** - Seamless token retrieval from GitHub CLI
  - Automatically uses `gh auth token` if no token is provided
  - Improved priority: GITHUB_TOKEN env var → keyring → gh CLI → config file
  - Works out-of-the-box if you have GitHub CLI authenticated

### Fixed
- **Secret encryption errors** - Fixed HTTP 422 "improperly encrypted secret" errors
  - Now uses environment-specific public keys instead of repository-level keys
  - Secrets can now be set successfully via `set-secret` command
- Improved error messages with better user guidance
- Better handling of macOS Keychain limitations with unsigned binaries

### Changed
- Environment names no longer restricted to alphanumeric characters
- Token retrieval order optimized for reliability
- All environment names URL-encoded for GitHub API calls
- Local .env file names sanitized for cross-platform compatibility
- Improved audit logging with repository context

## [0.2.0] - 2025-11-11

### Added
- **Global `--repo` flag** to specify repository explicitly instead of auto-detecting from git remote
  - Supports format: `repo` (uses organization from config) or `owner/repo`
  - Works with all commands except `init`
  - Useful when not in a git directory or managing multiple repositories
  - Example: `with-env --repo my-repo list-envs`

### Changed
- Repository auto-detection now shows helpful error message suggesting `--repo` flag
- All command signatures updated to support optional repository parameter

## [0.1.1] - 2025-11-11

### Security Fixes

#### Critical (4 issues fixed)
- **Fixed command injection vulnerability** - Commands now validated to reject shell metacharacters
- **Fixed secrets exposure in CLI arguments** - Secrets now read via stdin/file instead of CLI args
- **Fixed plaintext token storage** - Tokens now stored in system keyring instead of plaintext files
- **Fixed silent audit failures** - Audit logging errors now propagate instead of being ignored

#### High Severity (4 issues fixed)
- **Fixed path traversal vulnerability** - Environment names validated with strict regex
- **Added audit endpoint authentication** - HTTPS enforcement and Bearer token support
- **Fixed information disclosure** - Error messages sanitized to redact sensitive data
- **Added file permission checks** - Config/env files automatically set to mode 0600

### Added
- `keyring` crate for secure token storage in system keyring
- `rpassword` crate for hidden password input
- `regex` crate for input validation
- New security utilities module (`src/security.rs`)
- New validation module (`src/validation.rs`)
- Comprehensive security audit report (`SECURITY_AUDIT.md`)
- Detailed security fixes documentation (`SECURITY_FIXES.md`)
- HTTP request timeouts (30-60 seconds) to prevent hanging
- Input length limits to prevent DoS attacks

### Changed
- `set-secret` command now reads secrets interactively or from file, not CLI args
- Config structure adds `audit_token` field for secure audit logging
- GitHub tokens preferentially stored in keyring, config file deprecated
- All inputs (environment names, secret names, commands) now validated
- Error messages redact sensitive information before display
- Audit URL must use HTTPS (enforced)

### Deprecated
- Storing GitHub tokens in config file (still supported with warning)

## [0.1.0] - 2025-11-10

### Added
- Initial release
- GitHub secrets and environments management
- Audit logging for all secret operations
- Run commands with environment variables from GitHub environments
- Organization-level configuration
- Secret encryption using libsodium
- Support for local environment files

[0.3.0]: https://github.com/tilli-pro/env/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/tilli-pro/env/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/tilli-pro/env/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/tilli-pro/env/releases/tag/v0.1.0
