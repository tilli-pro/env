use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Sets secure file permissions (owner read/write only - 0600)
pub fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {}", path.display()))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)
            .with_context(|| format!("Failed to set permissions for {}", path.display()))?;
    }

    #[cfg(not(unix))]
    {
        // On Windows, we'll just ensure the file exists
        // Windows has different permission model (ACLs)
        let _ = path;
    }

    Ok(())
}

/// Checks if file has secure permissions and warns if not
pub fn check_file_permissions(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {}", path.display()))?;
        let perms = metadata.permissions();
        let mode = perms.mode();

        // Check if file is readable by group or others (masked with 0o077)
        if mode & 0o077 != 0 {
            eprintln!(
                "⚠️  WARNING: File {} has insecure permissions ({:o})",
                path.display(),
                mode & 0o777
            );
            eprintln!("   Recommended: Run 'chmod 600 {}'", path.display());
            eprintln!("   This file may contain sensitive data and should only be readable by you.");
            eprintln!();
        }
    }

    Ok(())
}

/// Redacts sensitive information from error messages
pub fn redact_error_message(error: &str) -> String {
    // Remove anything that looks like a token or secret
    let mut redacted = error.to_string();

    // Redact GitHub tokens (ghp_, gho_, ghs_, etc.)
    let token_patterns = ["ghp_", "gho_", "ghs_", "ghu_", "ghr_"];
    for pattern in token_patterns {
        if let Some(pos) = redacted.find(pattern) {
            // Find the end of the token (delimiter or end of string)
            let end = redacted[pos..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .unwrap_or(redacted[pos..].len());

            redacted.replace_range(pos..pos + end, &format!("{}***REDACTED***", pattern));
        }
    }

    // Redact basic auth credentials in URLs
    if let Some(start) = redacted.find("://") {
        if let Some(at_pos) = redacted[start + 3..].find('@') {
            let credential_end = start + 3 + at_pos;
            if redacted[start + 3..credential_end].contains(':') {
                redacted.replace_range(start + 3..credential_end, "***:***");
            }
        }
    }

    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    // ========================================================================
    // redact_error_message() tests
    // ========================================================================

    #[test]
    fn test_redact_github_token_ghp() {
        let error = "Failed to authenticate with token ghp_1234567890abcdef";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghp_***REDACTED***"));
        assert!(!redacted.contains("1234567890"));
        assert!(!redacted.contains("abcdef"));
    }

    #[test]
    fn test_redact_github_token_gho() {
        let error = "OAuth token gho_abcdefghijklmnop failed";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("gho_***REDACTED***"));
        assert!(!redacted.contains("abcdefghijk"));
    }

    #[test]
    fn test_redact_github_token_ghs() {
        let error = "Server token: ghs_secrettoken123";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghs_***REDACTED***"));
        assert!(!redacted.contains("secrettoken"));
    }

    #[test]
    fn test_redact_github_token_ghu() {
        let error = "User token ghu_test123456";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghu_***REDACTED***"));
    }

    #[test]
    fn test_redact_github_token_ghr() {
        let error = "Refresh token ghr_xyz789abc";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghr_***REDACTED***"));
    }

    #[test]
    fn test_redact_multiple_tokens() {
        let error = "Tokens: ghp_111 and ghs_222 found";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghp_***REDACTED***"));
        // Note: current implementation only redacts first occurrence per pattern
    }

    #[test]
    fn test_redact_token_with_quote() {
        let error = "Token is \"ghp_secret123\"";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghp_***REDACTED***"));
        assert!(!redacted.contains("secret123"));
    }

    #[test]
    fn test_redact_token_with_single_quote() {
        let error = "Token is 'ghp_secret123'";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghp_***REDACTED***"));
    }

    #[test]
    fn test_redact_incomplete_token() {
        let error = "Found ghp_ but nothing after";
        let redacted = redact_error_message(error);
        // Should handle gracefully
        assert!(redacted.contains("ghp_"));
    }

    #[test]
    fn test_redact_url_with_credentials() {
        let url_error = "Failed to connect to https://user:password@example.com/api";
        let redacted = redact_error_message(url_error);
        assert!(redacted.contains("***:***"));
        assert!(!redacted.contains("user"));
        assert!(!redacted.contains("password"));
        assert!(redacted.contains("example.com"));
    }

    #[test]
    fn test_redact_http_url_with_credentials() {
        let error = "Connection to http://admin:secret@host.com failed";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("***:***"));
        assert!(!redacted.contains("admin"));
        assert!(!redacted.contains("secret"));
    }

    #[test]
    fn test_redact_url_without_credentials() {
        let error = "Failed: https://example.com/api";
        let redacted = redact_error_message(error);
        // Should not change anything
        assert_eq!(redacted, error);
    }

    #[test]
    fn test_redact_url_with_port() {
        let error = "Error at https://user:pass@localhost:8080/path";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("***:***"));
        assert!(redacted.contains("localhost:8080"));
    }

    #[test]
    fn test_redact_empty_string() {
        let redacted = redact_error_message("");
        assert_eq!(redacted, "");
    }

    #[test]
    fn test_redact_no_secrets() {
        let error = "Just a regular error message with no secrets";
        let redacted = redact_error_message(error);
        assert_eq!(redacted, error);
    }

    #[test]
    fn test_redact_combined_token_and_url() {
        let error = "Failed with token ghp_abc123 at https://user:pass@api.com";
        let redacted = redact_error_message(error);
        assert!(redacted.contains("ghp_***REDACTED***"));
        assert!(redacted.contains("***:***"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("user"));
        assert!(!redacted.contains("pass"));
    }

    // ========================================================================
    // set_secure_permissions() tests
    // ========================================================================

    #[test]
    #[cfg(unix)]
    fn test_set_secure_permissions_creates_600() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Set insecure permissions first
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(path, perms).unwrap();

        // Verify insecure
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);

        // Apply secure permissions
        set_secure_permissions(path).unwrap();

        // Verify secure
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    #[cfg(unix)]
    fn test_set_secure_permissions_already_secure() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Set secure permissions
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms).unwrap();

        // Apply again - should not error
        set_secure_permissions(path).unwrap();

        // Verify still secure
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    #[cfg(windows)]
    fn test_set_secure_permissions_windows_no_op() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Should not error on Windows
        set_secure_permissions(path).unwrap();
    }

    // ========================================================================
    // check_file_permissions() tests
    // ========================================================================

    #[test]
    #[cfg(unix)]
    fn test_check_permissions_secure_no_warning() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Set secure permissions
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms).unwrap();

        // Should not error
        check_file_permissions(path).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn test_check_permissions_insecure_warns() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Set insecure permissions
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(path, perms).unwrap();

        // Should not error, but prints warning (can't easily test stderr)
        check_file_permissions(path).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn test_check_permissions_world_readable_warns() {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Set world-readable permissions
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o777);
        fs::set_permissions(path, perms).unwrap();

        // Should not error
        check_file_permissions(path).unwrap();
    }

    #[test]
    fn test_check_permissions_nonexistent_file_ok() {
        let path = std::path::Path::new("/nonexistent/file/that/does/not/exist");
        // Should not error for non-existent file
        check_file_permissions(path).unwrap();
    }

    #[test]
    #[cfg(windows)]
    fn test_check_permissions_windows_no_op() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Should not error on Windows
        check_file_permissions(path).unwrap();
    }
}
