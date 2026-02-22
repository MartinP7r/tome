//! CLI argument parsing with clap. Defines the `Cli` struct and `Command` enum.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "tome",
    version,
    about = "Sync AI coding skills across tools",
    after_help = "Examples:\n  tome init\n  tome sync --dry-run\n  tome status\n  tome list\n  tome doctor"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Path to config file (default: ~/.config/tome/config.toml)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Preview changes without modifying filesystem
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Detailed output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive wizard to configure sources and targets
    Init,

    /// Discover, consolidate, and distribute skills
    Sync,

    /// Show current state of skills, symlinks, and targets
    Status,

    /// Diagnose and repair broken symlinks or config issues
    Doctor,

    /// Start the MCP server
    Serve,

    /// List all discovered skills with their sources
    #[command(alias = "ls")]
    List,

    /// Show or edit configuration
    Config {
        /// Print config file path only
        #[arg(long)]
        path: bool,
    },
}
