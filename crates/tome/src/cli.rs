//! CLI argument parsing with clap. Defines the `Cli` struct and `Command` enum.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "tome",
    version,
    about = "Sync AI coding skills across tools",
    after_help = "Examples:\n  tome init\n  tome sync --dry-run\n  tome status\n  tome list\n  tome browse\n  tome doctor"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Path to config file (default: ~/.tome/tome.toml)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Preview changes without modifying filesystem
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Detailed output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Path to machine preferences file (default: ~/.config/tome/machine.toml)
    #[arg(long, global = true)]
    pub machine: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive wizard to configure sources and targets
    Init,

    /// Discover, consolidate, and distribute skills
    Sync {
        /// Recreate all symlinks even if they appear up-to-date
        #[arg(short, long)]
        force: bool,
    },

    /// Review library changes and sync with interactive triage
    Update,

    /// Show library, sources, targets, and health summary
    Status,

    /// Diagnose and repair broken symlinks or config issues
    Doctor,

    /// List all discovered skills with their sources
    #[command(alias = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Interactively browse discovered skills
    Browse,

    /// Show or edit configuration
    Config {
        /// Print config file path only
        #[arg(long)]
        path: bool,
    },
}
