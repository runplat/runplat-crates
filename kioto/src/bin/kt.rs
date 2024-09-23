pub mod cli;
use cli::engine::EngineArgs;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// `kioto` Utility CLI
///
/// Provides tools for interacting with a `.kt` environment,
/// which is where files are loading to load and create a kioto engine.
#[derive(Parser)]
pub struct KiotoUtil {
    /// Directory to use as the root.
    ///
    /// If not set, will default to the current directory
    #[clap(long)]
    root_dir: Option<PathBuf>,
    /// Engine environment name
    ///
    /// If not set, will default to "default"
    #[clap(long, short, default_value = "default")]
    env: String,
    /// Command to execute
    #[clap(subcommand)]
    command: Commands,
}

/// kioto util commands
#[derive(Subcommand)]
enum Commands {
    /// Interact with the engine systems
    Engine(EngineArgs),
    /// Builds an environment and engine config that can be used to load
    /// kioto engines with
    Build(cli::build::BuildArgs),
    /// List currently defined plugins and plugin handlers in the current environment
    List,
    /// Creates a new environment
    NewEnv,
    /// Removes an environment
    RemoveEnv,
    /// Shows the current settings of an environment
    ShowEnv,
}

#[tokio::main]
async fn main() {
    let cli = KiotoUtil::parse();

    let kt_dir = cli
        .root_dir
        .unwrap_or(std::env::current_dir().expect("should have access to current dir"))
        .join(".kt");
    if !kt_dir.exists() {
        std::fs::create_dir_all(kt_dir).expect("should be able to create .kt dir");
    }
    match cli.command {
        Commands::Engine(_) => {}
        Commands::Build(args) => {
            args.build().exec();
        }
        Commands::List => {}
        Commands::NewEnv => {}
        Commands::RemoveEnv => {}
        Commands::ShowEnv => {}
    }
}
