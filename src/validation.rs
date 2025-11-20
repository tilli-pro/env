use anyhow::Result;
use regex::Regex;
use std::sync::OnceLock;

static SECRET_NAME_REGEX: OnceLock<Regex> = OnceLock::new();

/// Validates environment name to prevent security issues
/// Allows any printable characters including slashes (for namespaced environments)
/// Environment names are:
/// - URL-encoded when used in API calls
/// - Sanitized when used in file paths
pub fn validate_environment_name(env: &str) -> Result<()> {
    if env.is_empty() {
        anyhow::bail!("Environment name cannot be empty");
    }

    if env.len() > 255 {
        anyhow::bail!("Environment name too long (max 255 characters)");
    }

    // Trim and check for leading/trailing whitespace (UX issue, not security)
    if env.trim() != env {
        anyhow::bail!(
            "Environment name cannot have leading or trailing whitespace: '{}'",
            env
        );
    }

    // Prevent null bytes and control characters (security and compatibility)
    for ch in env.chars() {
        if ch.is_control() || ch == '\0' {
            anyhow::bail!(
                "Environment name cannot contain control characters or null bytes: '{}'",
                env
            );
        }
    }

    Ok(())
}

/// Sanitizes environment name for use in file paths
/// Replaces path separators and other problematic characters with safe alternatives
/// This prevents path traversal attacks when creating local env files
pub fn sanitize_for_filepath(env: &str) -> String {
    env.chars()
        .map(|c| match c {
            '/' | '\\' => '_',                        // Path separators
            ':' => '-',                               // Colon (problematic on Windows)
            '<' | '>' | '|' | '"' | '?' | '*' => '_', // Invalid filename chars
            c => c,
        })
        .collect()
}

/// Validates secret name
pub fn validate_secret_name(name: &str) -> Result<()> {
    let regex = SECRET_NAME_REGEX
        .get_or_init(|| Regex::new(r"^[A-Z0-9_]+$").expect("Invalid regex pattern"));

    if name.is_empty() {
        anyhow::bail!("Secret name cannot be empty");
    }

    if name.len() > 100 {
        anyhow::bail!("Secret name too long (max 100 characters)");
    }

    if !regex.is_match(name) {
        anyhow::bail!(
            "Invalid secret name '{}'. Use only uppercase letters, numbers, and underscores (e.g., API_KEY).",
            name
        );
    }

    Ok(())
}

/// Validates secret value
pub fn validate_secret_value(value: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("Secret value cannot be empty");
    }

    if value.len() > 65536 {
        anyhow::bail!("Secret value too long (max 64KB)");
    }

    Ok(())
}

/// Validates that a URL is HTTPS
pub fn validate_https_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        anyhow::bail!("URL must use HTTPS: {}", url);
    }

    Ok(())
}

/// Validates command name to prevent obvious injection attempts
/// This is a basic validation - users should still be careful about what they run
pub fn validate_command_name(cmd: &str) -> Result<()> {
    if cmd.is_empty() {
        anyhow::bail!("Command cannot be empty");
    }

    // Reject commands with path separators or shell metacharacters
    if cmd.contains('/') || cmd.contains('\\') {
        anyhow::bail!(
            "Command cannot contain path separators. Use full path or add to PATH instead."
        );
    }

    // Reject commands with dangerous shell metacharacters
    let dangerous_chars = ['|', '&', ';', '>', '<', '`', '$', '(', ')', '{', '}', ' '];
    if dangerous_chars.iter().any(|c| cmd.contains(*c)) {
        anyhow::bail!(
            "Command contains shell metacharacters. This is not allowed for security reasons."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // validate_environment_name() tests
    // ========================================================================

    #[test]
    fn test_env_name_valid_simple() {
        assert!(validate_environment_name("production").is_ok());
        assert!(validate_environment_name("staging").is_ok());
        assert!(validate_environment_name("dev").is_ok());
    }

    #[test]
    fn test_env_name_valid_with_hyphens() {
        assert!(validate_environment_name("prod-1").is_ok());
        assert!(validate_environment_name("test-env").is_ok());
        assert!(validate_environment_name("my-env-123").is_ok());
    }

    #[test]
    fn test_env_name_valid_with_underscores() {
        assert!(validate_environment_name("test_env").is_ok());
        assert!(validate_environment_name("my_environment").is_ok());
        assert!(validate_environment_name("prod_v2").is_ok());
    }

    #[test]
    fn test_env_name_valid_with_numbers() {
        assert!(validate_environment_name("dev123").is_ok());
        assert!(validate_environment_name("prod2").is_ok());
        assert!(validate_environment_name("123test").is_ok());
    }

    #[test]
    fn test_env_name_valid_mixed() {
        assert!(validate_environment_name("a-b_c123").is_ok());
        assert!(validate_environment_name("Test-Env_01").is_ok());
    }

    #[test]
    fn test_env_name_valid_single_char() {
        assert!(validate_environment_name("a").is_ok());
        assert!(validate_environment_name("1").is_ok());
    }

    #[test]
    fn test_env_name_invalid_empty() {
        let result = validate_environment_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_env_name_valid_with_dots() {
        assert!(validate_environment_name("env.with.dots").is_ok());
        assert!(validate_environment_name(".hidden").is_ok());
        assert!(validate_environment_name("test.env").is_ok());
    }

    #[test]
    fn test_env_name_valid_with_spaces() {
        assert!(validate_environment_name("env with spaces").is_ok());
        assert!(validate_environment_name("my environment").is_ok());
    }

    #[test]
    fn test_env_name_valid_special_chars() {
        assert!(validate_environment_name("prod!").is_ok());
        assert!(validate_environment_name("test@env").is_ok());
        assert!(validate_environment_name("env#1").is_ok());
        assert!(validate_environment_name("prod$").is_ok());
        assert!(validate_environment_name("env%").is_ok());
        assert!(validate_environment_name("test:env").is_ok());
        assert!(validate_environment_name("env(1)").is_ok());
        assert!(validate_environment_name("test[dev]").is_ok());
    }

    #[test]
    fn test_env_name_valid_with_slashes() {
        // Slashes are now allowed (for namespaced environments like @org/package)
        // They are sanitized for file paths and URL-encoded for API calls
        assert!(validate_environment_name("@tilli-pro/api (env:dev)").is_ok());
        assert!(validate_environment_name("prod/test").is_ok());
        assert!(validate_environment_name("@org/package").is_ok());
        assert!(validate_environment_name("test/feature").is_ok());
    }

    #[test]
    fn test_env_name_valid_with_backslashes() {
        // Backslashes are allowed (will be sanitized for file paths)
        assert!(validate_environment_name("C:\\Windows").is_ok());
        assert!(validate_environment_name("path\\to\\env").is_ok());
    }

    #[test]
    fn test_env_name_valid_dots() {
        // Dots are safe (will be URL encoded)
        assert!(validate_environment_name("..").is_ok());
        assert!(validate_environment_name("...").is_ok());
        assert!(validate_environment_name(".env").is_ok());
        assert!(validate_environment_name("../etc/passwd").is_ok());
        assert!(validate_environment_name("./test").is_ok());
    }

    #[test]
    fn test_env_name_invalid_leading_trailing_whitespace() {
        assert!(validate_environment_name(" prod").is_err());
        assert!(validate_environment_name("prod ").is_err());
        assert!(validate_environment_name(" test ").is_err());
        assert!(validate_environment_name("\tprod").is_err());
    }

    #[test]
    fn test_env_name_invalid_too_long() {
        let long_name = "x".repeat(256);
        let result = validate_environment_name(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_env_name_valid_max_length() {
        let max_name = "x".repeat(255);
        assert!(validate_environment_name(&max_name).is_ok());
    }

    #[test]
    fn test_env_name_invalid_newline() {
        assert!(validate_environment_name("prod\ntest").is_err());
        assert!(validate_environment_name("test\r\nenv").is_err());
    }

    #[test]
    fn test_env_name_invalid_control_chars() {
        assert!(validate_environment_name("prod\x00test").is_err());
        assert!(validate_environment_name("test\x01").is_err());
        assert!(validate_environment_name("env\x1B").is_err());
    }

    // ========================================================================
    // validate_secret_name() tests
    // ========================================================================

    #[test]
    fn test_secret_name_valid_simple() {
        assert!(validate_secret_name("API_KEY").is_ok());
        assert!(validate_secret_name("DATABASE_URL").is_ok());
        assert!(validate_secret_name("SECRET").is_ok());
    }

    #[test]
    fn test_secret_name_valid_with_numbers() {
        assert!(validate_secret_name("SECRET_123").is_ok());
        assert!(validate_secret_name("API_V2").is_ok());
        assert!(validate_secret_name("KEY123").is_ok());
    }

    #[test]
    fn test_secret_name_valid_with_underscores() {
        assert!(validate_secret_name("MY_SECRET_KEY").is_ok());
        assert!(validate_secret_name("A_B_C").is_ok());
        assert!(validate_secret_name("___").is_ok());
    }

    #[test]
    fn test_secret_name_valid_single_char() {
        assert!(validate_secret_name("A").is_ok());
        assert!(validate_secret_name("Z").is_ok());
    }

    #[test]
    fn test_secret_name_valid_numbers_only() {
        assert!(validate_secret_name("123").is_ok());
        assert!(validate_secret_name("456_789").is_ok());
    }

    #[test]
    fn test_secret_name_invalid_empty() {
        let result = validate_secret_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_secret_name_invalid_lowercase() {
        assert!(validate_secret_name("api_key").is_err());
        assert!(validate_secret_name("Api_Key").is_err());
        assert!(validate_secret_name("SECRET_key").is_err());
    }

    #[test]
    fn test_secret_name_invalid_hyphens() {
        assert!(validate_secret_name("API-KEY").is_err());
        assert!(validate_secret_name("MY-SECRET").is_err());
    }

    #[test]
    fn test_secret_name_invalid_spaces() {
        assert!(validate_secret_name("API KEY").is_err());
        assert!(validate_secret_name("MY SECRET").is_err());
        assert!(validate_secret_name(" API").is_err());
    }

    #[test]
    fn test_secret_name_invalid_dots() {
        assert!(validate_secret_name("API.KEY").is_err());
        assert!(validate_secret_name("MY.SECRET").is_err());
    }

    #[test]
    fn test_secret_name_invalid_special_chars() {
        assert!(validate_secret_name("API@KEY").is_err());
        assert!(validate_secret_name("SECRET!").is_err());
        assert!(validate_secret_name("KEY#1").is_err());
    }

    #[test]
    fn test_secret_name_invalid_too_long() {
        let long_name = "A".repeat(101);
        let result = validate_secret_name(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_secret_name_valid_max_length() {
        let max_name = "A".repeat(100);
        assert!(validate_secret_name(&max_name).is_ok());
    }

    // ========================================================================
    // validate_secret_value() tests
    // ========================================================================

    #[test]
    fn test_secret_value_valid_simple() {
        assert!(validate_secret_value("simple").is_ok());
        assert!(validate_secret_value("my-secret-123").is_ok());
    }

    #[test]
    fn test_secret_value_valid_with_special_chars() {
        assert!(validate_secret_value("complex!@#$%^&*()").is_ok());
        assert!(validate_secret_value("test-_+={}[]|\\:;\"'<>,.?/~`").is_ok());
    }

    #[test]
    fn test_secret_value_valid_with_newlines() {
        assert!(validate_secret_value("multi\nline\nsecret").is_ok());
        assert!(validate_secret_value("line1\r\nline2").is_ok());
    }

    #[test]
    fn test_secret_value_valid_with_unicode() {
        assert!(validate_secret_value("emoji-🔐-secret").is_ok());
        assert!(validate_secret_value("中文密码").is_ok());
    }

    #[test]
    fn test_secret_value_valid_max_size() {
        let max_value = "x".repeat(65536);
        assert!(validate_secret_value(&max_value).is_ok());
    }

    #[test]
    fn test_secret_value_invalid_empty() {
        let result = validate_secret_value("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_secret_value_invalid_too_long() {
        let too_long = "x".repeat(65537);
        let result = validate_secret_value(&too_long);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    // ========================================================================
    // validate_https_url() tests
    // ========================================================================

    #[test]
    fn test_https_url_valid_simple() {
        assert!(validate_https_url("https://example.com").is_ok());
        assert!(validate_https_url("https://api.example.com").is_ok());
    }

    #[test]
    fn test_https_url_valid_with_path() {
        assert!(validate_https_url("https://example.com/path").is_ok());
        assert!(validate_https_url("https://example.com/api/v1/events").is_ok());
    }

    #[test]
    fn test_https_url_valid_with_port() {
        assert!(validate_https_url("https://example.com:8080").is_ok());
        assert!(validate_https_url("https://localhost:3000/api").is_ok());
    }

    #[test]
    fn test_https_url_valid_with_query() {
        assert!(validate_https_url("https://example.com?key=value").is_ok());
        assert!(validate_https_url("https://example.com/path?a=1&b=2").is_ok());
    }

    #[test]
    fn test_https_url_invalid_http() {
        let result = validate_https_url("http://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));
    }

    #[test]
    fn test_https_url_invalid_ftp() {
        assert!(validate_https_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_https_url_invalid_no_protocol() {
        assert!(validate_https_url("example.com").is_err());
        assert!(validate_https_url("www.example.com").is_err());
    }

    #[test]
    fn test_https_url_invalid_empty() {
        assert!(validate_https_url("").is_err());
    }

    // ========================================================================
    // validate_command_name() tests
    // ========================================================================

    #[test]
    fn test_command_name_valid_simple() {
        assert!(validate_command_name("npm").is_ok());
        assert!(validate_command_name("cargo").is_ok());
        assert!(validate_command_name("node").is_ok());
        assert!(validate_command_name("python").is_ok());
    }

    #[test]
    fn test_command_name_valid_with_numbers() {
        assert!(validate_command_name("python3").is_ok());
        assert!(validate_command_name("node16").is_ok());
    }

    #[test]
    fn test_command_name_valid_with_hyphens() {
        assert!(validate_command_name("my-script").is_ok());
        assert!(validate_command_name("run-tests").is_ok());
    }

    #[test]
    fn test_command_name_valid_with_underscores() {
        assert!(validate_command_name("my_command").is_ok());
        assert!(validate_command_name("test_runner").is_ok());
    }

    #[test]
    fn test_command_name_valid_with_dots() {
        assert!(validate_command_name("script.sh").is_ok());
        assert!(validate_command_name("run.py").is_ok());
    }

    #[test]
    fn test_command_name_invalid_empty() {
        let result = validate_command_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_command_name_invalid_with_slash() {
        let result = validate_command_name("/bin/sh");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path separator"));
    }

    #[test]
    fn test_command_name_invalid_with_backslash() {
        let result = validate_command_name("C:\\Windows\\cmd.exe");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path separator"));
    }

    #[test]
    fn test_command_name_invalid_with_pipe() {
        let result = validate_command_name("npm | grep");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("metacharacter"));
    }

    #[test]
    fn test_command_name_invalid_with_ampersand() {
        assert!(validate_command_name("npm && rm -rf /").is_err());
        assert!(validate_command_name("npm &").is_err());
    }

    #[test]
    fn test_command_name_invalid_with_semicolon() {
        assert!(validate_command_name("npm; ls").is_err());
        assert!(validate_command_name("test;").is_err());
    }

    #[test]
    fn test_command_name_invalid_with_redirect() {
        assert!(validate_command_name("npm > out.txt").is_err());
        assert!(validate_command_name("cat < input").is_err());
    }

    #[test]
    fn test_command_name_invalid_with_backticks() {
        let result = validate_command_name("npm `whoami`");
        assert!(result.is_err());
    }

    #[test]
    fn test_command_name_invalid_with_dollar() {
        assert!(validate_command_name("npm $(whoami)").is_err());
        assert!(validate_command_name("echo $HOME").is_err());
    }

    #[test]
    fn test_command_name_invalid_with_parens() {
        assert!(validate_command_name("(npm)").is_err());
        assert!(validate_command_name("test()").is_err());
    }

    #[test]
    fn test_command_name_invalid_with_braces() {
        assert!(validate_command_name("{npm}").is_err());
        assert!(validate_command_name("test{}").is_err());
    }

    #[test]
    fn test_command_name_with_space_is_invalid() {
        // Spaces indicate arguments, not part of command name
        let result = validate_command_name("sh -c 'echo test'");
        assert!(result.is_err());

        let result = validate_command_name("cat /etc/passwd");
        assert!(result.is_err());
    }

    // ========================================================================
    // sanitize_for_filepath() tests
    // ========================================================================

    #[test]
    fn test_sanitize_filepath_slashes() {
        assert_eq!(sanitize_for_filepath("@org/package"), "@org_package");
        assert_eq!(sanitize_for_filepath("prod/test"), "prod_test");
        assert_eq!(sanitize_for_filepath("path\\to\\env"), "path_to_env");
    }

    #[test]
    fn test_sanitize_filepath_colons() {
        assert_eq!(sanitize_for_filepath("env:dev"), "env-dev");
        assert_eq!(
            sanitize_for_filepath("@org/api (env:prod)"),
            "@org_api (env-prod)"
        );
    }

    #[test]
    fn test_sanitize_filepath_windows_invalid_chars() {
        assert_eq!(sanitize_for_filepath("file<name"), "file_name");
        assert_eq!(sanitize_for_filepath("file>name"), "file_name");
        assert_eq!(sanitize_for_filepath("file|name"), "file_name");
        assert_eq!(sanitize_for_filepath("file\"name"), "file_name");
        assert_eq!(sanitize_for_filepath("file?name"), "file_name");
        assert_eq!(sanitize_for_filepath("file*name"), "file_name");
    }

    #[test]
    fn test_sanitize_filepath_safe_chars() {
        // Safe characters should pass through unchanged
        assert_eq!(sanitize_for_filepath("prod-env_v2"), "prod-env_v2");
        assert_eq!(sanitize_for_filepath("test.staging"), "test.staging");
        assert_eq!(sanitize_for_filepath("env@123"), "env@123");
        assert_eq!(sanitize_for_filepath("app(2024)"), "app(2024)");
    }

    #[test]
    fn test_sanitize_filepath_complex() {
        assert_eq!(
            sanitize_for_filepath("@tilli-pro/api (env:dev)"),
            "@tilli-pro_api (env-dev)"
        );
        assert_eq!(
            sanitize_for_filepath("C:\\Windows\\System32"),
            "C-_Windows_System32"
        );
    }
}
