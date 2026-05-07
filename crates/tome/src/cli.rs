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

    /// Disable all interactive prompts — takes the default at every prompt.
    ///
    /// For `tome init`, this also skips the optional git-init-for-backup step
    /// (run `tome backup init` separately if you want that). For `tome sync`,
    /// this implies `--no-triage`.
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
    #[command(
        after_help = "Examples:\n  tome init\n  tome init --dry-run\n  tome init --no-input\n  tome init --dry-run --no-input"
    )]
    Init,

    /// Discover, consolidate, and distribute skills
    #[command(
        after_help = "Examples:\n  tome sync\n  tome sync --dry-run\n  tome sync --force\n  tome sync --no-triage\n  tome sync --no-input\n  tome sync --no-install"
    )]
    Sync {
        /// Recreate all symlinks even if they appear up-to-date
        #[arg(short, long)]
        force: bool,
        /// Skip interactive triage of new/changed skills
        #[arg(long)]
        no_triage: bool,
        /// Skip auto-install/update of missing or drifted managed plugins this run.
        ///
        /// Doesn't change the persisted `auto_install_plugins` setting in
        /// `machine.toml`. Mirrors Cargo's `--frozen` / `--locked`.
        #[arg(long)]
        no_install: bool,
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

    /// One-shot migration: convert a v0.9-shape library (managed skills as
    /// symlinks) to v0.10 shape (real directory copies). Run once after
    /// upgrading from v0.9.x. Idempotent on re-run.
    ///
    /// Commit your library (or back it up) BEFORE running — there is no
    /// path back to v0.9 shape.
    #[command(
        after_help = "Examples:\n  tome migrate-library --dry-run\n  tome migrate-library\n\nThis is a one-shot command for migrating from tome v0.9.x to v0.10. \
                       On v0.10 fresh installs it has nothing to do."
    )]
    MigrateLibrary {
        /// Preview changes without modifying filesystem
        #[arg(long)]
        dry_run: bool,
    },

    /// Interactively browse discovered skills
    #[command(after_help = "Examples:\n  tome browse")]
    Browse,

    /// Remove tome's symlinks from all target tool directories (reversible via `tome sync`)
    #[command(after_help = "Examples:\n  tome eject\n  tome eject --dry-run")]
    Eject,

    /// Manage skills and directories — remove a configured directory entry
    /// or delete an Unowned skill from the library.
    #[command(
        after_help = "Examples:\n  tome remove dir my-git-source\n  tome remove dir my-git-source --force\n  tome remove skill orphaned-foo\n  tome remove skill orphaned-foo --yes"
    )]
    Remove {
        #[command(subcommand)]
        kind: RemoveKind,
    },

    /// Reassign a skill to a different directory. Accepts both Owned skills
    /// (today's behaviour) and Unowned skills (re-anchors them per UNOWN-01 /
    /// D-API-1). Refuses to overwrite an existing skill in the target with
    /// different content unless `--force` is passed (D-A1).
    #[command(
        after_help = "Examples:\n  tome reassign my-skill --to local-skills\n  tome reassign orphaned-foo --to local-skills\n  tome reassign my-skill --to local-skills --force"
    )]
    Reassign {
        /// Skill name to reassign
        #[arg(value_name = "SKILL")]
        skill: String,
        /// Target directory name
        #[arg(long)]
        to: String,
        /// Overwrite an existing skill in the target if its content hash
        /// differs from the library copy (D-A1). Same-content collisions
        /// always relink without `--force`.
        #[arg(long)]
        force: bool,
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

/// Variant of `tome remove` — directory removal vs unowned-skill deletion.
/// Per D-API-2 (Phase 14): the merge replaces today's `tome remove <name>`
/// shape (BREAKING). `tome remove dir` keeps today's directory-removal
/// behaviour; `tome remove skill` deletes an Unowned skill from the library.
#[derive(Debug, Subcommand)]
pub enum RemoveKind {
    /// Remove a directory entry from `tome.toml` and clean up its artifacts
    /// (today's `tome remove <name>` behaviour, renamed). Owned skills
    /// transition to Unowned per LIB-04.
    Dir {
        /// Name of the directory to remove (as shown in `tome status`)
        #[arg(value_name = "NAME")]
        name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Delete an Unowned skill from the library — manifest entry, library
    /// directory, distribution symlinks, lockfile entry, and machine.toml
    /// membership all cleaned. Owned skills are refused with a hint to
    /// run `tome remove dir` first (D-B2).
    Skill {
        /// Skill name to forget (must currently be Unowned)
        #[arg(value_name = "NAME")]
        name: String,
        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_remove_dir_with_force() {
        let cli = Cli::try_parse_from(["tome", "remove", "dir", "my-source", "--force"]).unwrap();
        match cli.command {
            Command::Remove {
                kind: RemoveKind::Dir { name, force },
            } => {
                assert_eq!(name, "my-source");
                assert!(force);
            }
            _ => panic!("expected Remove::Dir"),
        }
    }

    #[test]
    fn parse_remove_skill_with_yes() {
        let cli = Cli::try_parse_from(["tome", "remove", "skill", "orphan-foo", "--yes"]).unwrap();
        match cli.command {
            Command::Remove {
                kind: RemoveKind::Skill { name, yes },
            } => {
                assert_eq!(name, "orphan-foo");
                assert!(yes);
            }
            _ => panic!("expected Remove::Skill"),
        }
    }

    #[test]
    fn parse_remove_skill_short_y() {
        let cli = Cli::try_parse_from(["tome", "remove", "skill", "orphan-foo", "-y"]).unwrap();
        match cli.command {
            Command::Remove {
                kind: RemoveKind::Skill { yes, .. },
            } => assert!(yes),
            _ => panic!("expected Remove::Skill"),
        }
    }

    #[test]
    fn parse_reassign_force_flag_recognised() {
        let cli = Cli::try_parse_from(["tome", "reassign", "my-skill", "--to", "dst", "--force"])
            .unwrap();
        match cli.command {
            Command::Reassign { skill, to, force } => {
                assert_eq!(skill, "my-skill");
                assert_eq!(to, "dst");
                assert!(force);
            }
            _ => panic!("expected Reassign"),
        }
    }

    #[test]
    fn old_shape_remove_with_bare_name_fails() {
        // Today's `tome remove my-source` should NO LONGER parse — clap
        // requires an explicit subcommand. BREAKING per D-API-2.
        let result = Cli::try_parse_from(["tome", "remove", "my-source"]);
        assert!(
            result.is_err(),
            "bare `tome remove <name>` must fail post-restructure (BREAKING)"
        );
    }
}
