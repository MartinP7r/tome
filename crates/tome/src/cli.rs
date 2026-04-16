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

    /// Override tome home directory (default: ~/.tome/, or TOME_HOME env var)
    #[arg(long, global = true)]
    pub tome_home: Option<PathBuf>,

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

    /// Disable all interactive prompts (implies --no-triage for sync)
    #[arg(long, global = true)]
    pub no_input: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LintFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a git skill repository
    #[command(
        after_help = "Examples:\n  tome add https://github.com/user/skills.git\n  tome add https://github.com/user/skills.git --name my-skills\n  tome add git@github.com:user/skills.git --branch main"
    )]
    Add {
        /// Git repository URL (HTTPS or SSH)
        #[arg(value_name = "URL")]
        url: String,
        /// Custom directory name (default: extracted from URL)
        #[arg(long)]
        name: Option<String>,
        /// Track a specific branch
        #[arg(long, conflicts_with_all = ["tag", "rev"])]
        branch: Option<String>,
        /// Pin to a specific tag
        #[arg(long, conflicts_with_all = ["branch", "rev"])]
        tag: Option<String>,
        /// Pin to a specific commit SHA
        #[arg(long, conflicts_with_all = ["branch", "tag"])]
        rev: Option<String>,
    },

    /// Interactive wizard to configure sources and targets
    #[command(after_help = "Examples:\n  tome init")]
    Init,

    /// Discover, consolidate, and distribute skills
    #[command(
        after_help = "Examples:\n  tome sync\n  tome sync --dry-run\n  tome sync --force\n  tome sync --no-triage\n  tome sync --no-input"
    )]
    Sync {
        /// Recreate all symlinks even if they appear up-to-date
        #[arg(short, long)]
        force: bool,
        /// Skip interactive triage of new/changed skills
        #[arg(long)]
        no_triage: bool,
    },

    /// Show library, sources, targets, and health summary
    #[command(after_help = "Examples:\n  tome status\n  tome status --json")]
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Diagnose and repair broken symlinks or config issues
    #[command(
        after_help = "Examples:\n  tome doctor\n  tome doctor --dry-run\n  tome doctor --json"
    )]
    Doctor {
        /// Output as JSON (skips repair)
        #[arg(long)]
        json: bool,
    },

    /// List all discovered skills with their sources
    #[command(
        alias = "ls",
        after_help = "Examples:\n  tome list\n  tome list --json"
    )]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate skill frontmatter and report issues
    #[command(
        after_help = "Examples:\n  tome lint\n  tome lint path/to/skill\n  tome lint --format json"
    )]
    Lint {
        /// Specific skill directory to lint (default: entire library)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: LintFormat,
    },

    /// Interactively browse discovered skills
    #[command(after_help = "Examples:\n  tome browse")]
    Browse,

    /// Remove tome's symlinks from all target tool directories (reversible via `tome sync`)
    #[command(after_help = "Examples:\n  tome eject\n  tome eject --dry-run")]
    Eject,

    /// Remove a directory entry and clean up its artifacts
    #[command(
        after_help = "Examples:\n  tome remove my-git-source\n  tome remove my-git-source --dry-run\n  tome remove my-git-source --force"
    )]
    Remove {
        /// Name of the directory to remove (as shown in `tome status`)
        #[arg(value_name = "NAME")]
        name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Reassign a skill to a different directory
    #[command(after_help = "Examples:\n  tome reassign my-skill --to local-skills")]
    Reassign {
        /// Skill name to reassign
        #[arg(value_name = "SKILL")]
        skill: String,
        /// Target directory name
        #[arg(long)]
        to: String,
    },

    /// Fork a managed skill to a local directory for customization
    #[command(
        after_help = "Examples:\n  tome fork my-skill --to local-skills\n  tome fork my-skill --to local-skills --force"
    )]
    Fork {
        /// Skill name to fork
        #[arg(value_name = "SKILL")]
        skill: String,
        /// Target local directory
        #[arg(long)]
        to: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Move the skill library to a new location safely
    #[command(after_help = "Examples:\n  tome relocate ~/new-library")]
    Relocate {
        /// New library directory path
        #[arg(value_name = "NEW_PATH")]
        new_path: PathBuf,
    },

    /// Install shell completions for bash, zsh, fish, or powershell
    #[command(after_help = "Examples:\n  tome completions fish\n  tome completions zsh --print")]
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
    #[command(after_help = "Examples:\n  tome config\n  tome config --path")]
    Config {
        /// Print config file path only
        #[arg(long)]
        path: bool,
    },

    /// Git-backed backup and restore for the skill library
    #[command(
        after_help = "Examples:\n  tome backup init\n  tome backup snapshot -m 'before update'\n  tome backup list\n  tome backup diff"
    )]
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
