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

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LintFormat {
    Text,
    Json,
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
        /// Skip interactive triage of new/changed skills
        #[arg(long)]
        no_triage: bool,
    },

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

    /// Validate skill frontmatter and report issues
    Lint {
        /// Specific skill directory to lint (default: entire library)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: LintFormat,
    },

    /// Interactively browse discovered skills
    Browse,

    /// Remove tome's symlinks from all target tool directories (reversible via `tome sync`)
    Eject,

    /// Move the skill library to a new location safely
    Relocate {
        /// New library directory path
        #[arg(value_name = "NEW_PATH")]
        new_path: PathBuf,
    },

    /// Install shell completions for bash, zsh, fish, or powershell
    Completions {
        /// Shell to install completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
        /// Print completions to stdout instead of installing
        #[arg(long)]
        print: bool,
    },

    /// Print version information
    Version,

    /// Show configuration
    Config {
        /// Print config file path only
        #[arg(long)]
        path: bool,
    },

    /// Git-backed backup and restore for the skill library
    Backup {
        #[command(subcommand)]
        sub: BackupCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum BackupCommand {
    /// Initialize git repo in the library for backup tracking
    Init,
    /// Create a snapshot of the current library state
    Snapshot {
        /// Custom commit message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Show backup history
    List {
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
    },
    /// Restore library to a previous snapshot
    Restore {
        /// Git ref to restore to (commit hash, HEAD~1, etc.)
        #[arg(default_value = "HEAD~1")]
        target: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Show changes since last backup
    Diff {
        /// Compare against specific ref (default: last commit)
        #[arg(default_value = "HEAD")]
        target: String,
    },
}
