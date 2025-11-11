# Contributing to with-env

Thank you for your interest in contributing to with-env!

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- GitHub account with a personal access token

### Building from Source

```bash
# Clone the repository
git clone https://github.com/tilli-pro/env.git
cd env

# Build the project
cargo build

# Run the binary
./target/debug/with-env --help
```

### Running Tests

```bash
cargo test
```

### Code Quality

Before submitting a pull request, ensure your code passes all checks:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Build
cargo build

# Run tests
cargo test
```

## Project Structure

```
src/
├── main.rs              # CLI entry point and command definitions
├── audit/               # Audit logging functionality
│   └── mod.rs
├── commands/            # Command implementations
│   ├── mod.rs
│   ├── init.rs         # Initialize configuration
│   ├── list_envs.rs    # List environments
│   ├── list_secrets.rs # List secrets
│   ├── get_secret.rs   # Get secret metadata
│   ├── set_secret.rs   # Set/update secrets
│   ├── delete_secret.rs# Delete secrets
│   └── run.rs          # Run commands with env vars
├── config/              # Configuration management
│   └── mod.rs
└── github/              # GitHub API client
    └── mod.rs
```

## Making Changes

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Run quality checks (fmt, clippy, tests)
5. Commit your changes (`git commit -am 'Add some feature'`)
6. Push to the branch (`git push origin feature/your-feature`)
7. Create a Pull Request

## Code Style

- Follow Rust standard naming conventions
- Use `cargo fmt` for code formatting
- Ensure `cargo clippy` passes without warnings
- Add documentation comments for public APIs
- Write descriptive commit messages

## Security

- Never commit secrets or API tokens
- Use environment variables for sensitive data
- Ensure proper encryption for secret values
- Report security vulnerabilities privately to the maintainers

## Testing

- Add tests for new features
- Ensure existing tests pass
- Test edge cases and error conditions
- Document any manual testing performed

## Documentation

- Update README.md if you add new features
- Add inline documentation for complex code
- Include usage examples for new commands

## Questions?

If you have questions or need help, please open an issue or reach out to the maintainers.
