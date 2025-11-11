mod audit;
mod commands;
mod config;
mod github;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "with-env")]
#[command(version)]
#[command(about = "Manage GitHub secrets and environments", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run a command with environment variables (when no subcommand is provided)
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration
    Init {
        /// GitHub organization name
        #[arg(short, long)]
        org: String,

        /// Audit URL for logging
        #[arg(short, long)]
        audit_url: Option<String>,

        /// GitHub token (if not provided, will use GITHUB_TOKEN env var)
        #[arg(short, long)]
        token: Option<String>,
    },

    /// List environments for the current repository
    ListEnvs,

    /// List secrets for an environment
    ListSecrets {
        /// Environment name
        environment: String,
    },

    /// Get a secret value
    GetSecret {
        /// Environment name
        environment: String,

        /// Secret name
        secret: String,
    },

    /// Set a secret value
    SetSecret {
        /// Environment name
        environment: String,

        /// Secret name
        secret: String,

        /// Secret value
        value: String,
    },

    /// Delete a secret
    DeleteSecret {
        /// Environment name
        environment: String,

        /// Secret name
        secret: String,
    },

    /// Run a command with environment variables from a specified environment
    Run {
        /// Environment name
        environment: String,

        /// Command and arguments to run
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init {
            org,
            audit_url,
            token,
        }) => commands::init::handle(org, audit_url, token).await,
        Some(Commands::ListEnvs) => commands::list_envs::handle().await,
        Some(Commands::ListSecrets { environment }) => {
            commands::list_secrets::handle(environment).await
        }
        Some(Commands::GetSecret {
            environment,
            secret,
        }) => commands::get_secret::handle(environment, secret).await,
        Some(Commands::SetSecret {
            environment,
            secret,
            value,
        }) => commands::set_secret::handle(environment, secret, value).await,
        Some(Commands::DeleteSecret {
            environment,
            secret,
        }) => commands::delete_secret::handle(environment, secret).await,
        Some(Commands::Run {
            environment,
            command,
        }) => commands::run::handle(environment, command).await,
        None => {
            // If no subcommand is provided, treat it as a run command with the default environment
            if cli.args.is_empty() {
                eprintln!("Error: No command provided");
                eprintln!("Usage: with-env <command> [args...]");
                eprintln!("   or: with-env <subcommand>");
                std::process::exit(1);
            }
            // Use "default" as the environment name
            commands::run::handle("default".to_string(), cli.args).await
        }
    }
}
