//! CLI argument parsing with clap. Defines the `Cli` struct and `Command` enum.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Verbosity level resolved from `--verbose` / `--quiet` flags.
///
/// Per HARD-07: collapses what was previously `pub verbose: bool` +
/// `pub quiet: bool` on `Cli` into a single typed accessor. The flag UX is
/// preserved exactly — clap continues to parse `--verbose` / `--quiet` with
/// `conflicts_with` — only the public `Cli` field surface changes from two
/// booleans to a single `pub fn log_level(&self) -> LogLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    /// Suppress non-error output (`--quiet`).
    Quiet,
    /// Default output (no flag).
    #[default]
    Normal,
    /// Detailed output (`--verbose`).
    Verbose,
}

impl LogLevel {
    /// Compile-time exhaustiveness sentinel array (POLISH-04 pattern).
    /// Adding a variant without updating ALL is a compile error via
    /// the const-len assert below.
    pub const ALL: [Self; 3] = [Self::Quiet, Self::Normal, Self::Verbose];

    /// True iff variant is `Verbose`.
    pub fn is_verbose(self) -> bool {
        matches!(self, Self::Verbose)
    }

    /// True iff variant is `Quiet`.
    pub fn is_quiet(self) -> bool {
        matches!(self, Self::Quiet)
    }

    /// Map verbosity to a `tracing_subscriber::EnvFilter` directive string.
    /// Single source of truth for the flag → tracing level translation per
    /// D-ENV-3 (Phase 18). Called by `tracing_init::install` when `TOME_LOG`
    /// is unset.
    pub fn directive(self) -> &'static str {
        match self {
            Self::Quiet => "warn",
            Self::Normal => "info",
            Self::Verbose => "debug",
        }
    }
}

// POLISH-04 exhaustiveness sentinel — compile fails if a new variant is
// added without updating LogLevel::ALL. Mirrors marketplace.rs's
// InstallFailureKind sentinel pattern.
#[allow(dead_code)]
fn _log_level_exhaustiveness(l: LogLevel) {
    match l {
        LogLevel::Quiet => {}
        LogLevel::Normal => {}
        LogLevel::Verbose => {}
    }
}
const _: () = assert!(
    LogLevel::ALL.len() == 3,
    "LogLevel::ALL must list every variant",
);

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
    //
    // Private — read via `Cli::log_level()` per HARD-07. clap requires
    // the field be on `Cli` directly to wire the `--verbose` / `-v` flag,
    // so visibility is the only thing that flips.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

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

impl Cli {
    /// Resolve the parsed `--verbose` / `--quiet` flags into a typed
    /// `LogLevel`. Replaces direct `cli.verbose` / `cli.quiet` reads per
    /// HARD-07. clap's `conflicts_with` already prevents both flags being
    /// set, so the order of branches here is observationally irrelevant —
    /// `Verbose` first matches the precedence in the previous boolean code.
    pub fn log_level(&self) -> LogLevel {
        if self.verbose {
            LogLevel::Verbose
        } else if self.quiet {
            LogLevel::Quiet
        } else {
            LogLevel::Normal
        }
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LintFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a git skill repository
    #[command(after_help = "Examples:\n  \
                      tome add https://github.com/user/skills.git\n  \
                      tome add user/skills                                 # bare slug → github.com\n  \
                      tome add user/skills/tree/main/skills                # /tree/<ref>/<subdir> shortcut\n  \
                      tome add user/skills --subdir skills                 # explicit --subdir flag\n  \
                      tome add user/skills --name my-skills                # custom directory name\n  \
                      tome add git@github.com:user/skills.git --branch main")]
    Add {
        /// Git repository URL (HTTPS or SSH) or `owner/repo` slug.
        ///
        /// Supports a GitHub `/tree/<ref>/<subdir>` suffix — the URL form
        /// the browser shows when navigating into a subdir on github.com.
        /// The `<ref>` becomes the default branch and `<subdir>` becomes
        /// the discovery subdir. Explicit `--branch` / `--subdir` flags
        /// override the URL-embedded values (with a warning).
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
        /// Restrict discovery to a subdirectory of the clone.
        ///
        /// When set, discovery scans `<clone>/<SUBDIR>/*/SKILL.md` instead
        /// of `<clone>/*/SKILL.md`. Common for Claude Code plugin repos
        /// that keep skills under `skills/` (try `--subdir skills`).
        #[arg(long, value_name = "PATH")]
        subdir: Option<String>,
    },

    /// Interactive wizard to configure directories
    #[command(
        after_help = "Examples:\n  tome init\n  tome init --dry-run\n  tome init --no-input\n  tome init --dry-run --no-input"
    )]
    Init,

    /// Discover, consolidate, and distribute skills
    #[command(
        long_about = "Discover, consolidate, and distribute skills.\n\n\
                      For the cross-machine library-as-dotfiles workflow — committing \
                      ~/.tome/ to dotfiles, bootstrapping a fresh machine, and the \
                      auto_install_plugins consent flow — see docs/src/cross-machine-sync.md \
                      (or the rendered mdbook page at the same path if you have the docs \
                      built locally).",
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

    /// Show library, directories, last-sync, and health summary
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

    /// List all discovered skills with their directory
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
        after_help = "Examples:\n  tome migrate-library --dry-run\n  tome migrate-library\n  tome migrate-library --yes\n\nThis is a one-shot command for migrating from tome v0.9.x to v0.10. \
                       On v0.10 fresh installs it has nothing to do."
    )]
    MigrateLibrary {
        /// Preview changes without modifying filesystem
        #[arg(long)]
        dry_run: bool,
        /// Skip the confirmation prompt and proceed directly. Mirrors
        /// `tome remove skill --yes` (Phase 14 D-B3). Required when running
        /// under `--no-input` to confirm the destructive conversion.
        #[arg(long, short)]
        yes: bool,
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

    // -- HARD-07: LogLevel parsing tests --

    #[test]
    fn log_level_default_is_normal() {
        let cli = Cli::try_parse_from(["tome", "status"]).unwrap();
        assert_eq!(cli.log_level(), LogLevel::Normal);
        assert!(!cli.log_level().is_verbose());
        assert!(!cli.log_level().is_quiet());
    }

    #[test]
    fn log_level_verbose_flag_resolves_to_verbose() {
        let cli = Cli::try_parse_from(["tome", "--verbose", "status"]).unwrap();
        assert_eq!(cli.log_level(), LogLevel::Verbose);
        assert!(cli.log_level().is_verbose());
        assert!(!cli.log_level().is_quiet());
    }

    #[test]
    fn log_level_short_v_flag_resolves_to_verbose() {
        let cli = Cli::try_parse_from(["tome", "-v", "status"]).unwrap();
        assert_eq!(cli.log_level(), LogLevel::Verbose);
    }

    #[test]
    fn log_level_quiet_flag_resolves_to_quiet() {
        let cli = Cli::try_parse_from(["tome", "--quiet", "status"]).unwrap();
        assert_eq!(cli.log_level(), LogLevel::Quiet);
        assert!(cli.log_level().is_quiet());
        assert!(!cli.log_level().is_verbose());
    }

    #[test]
    fn log_level_short_q_flag_resolves_to_quiet() {
        let cli = Cli::try_parse_from(["tome", "-q", "status"]).unwrap();
        assert_eq!(cli.log_level(), LogLevel::Quiet);
    }

    #[test]
    fn log_level_verbose_and_quiet_together_rejected() {
        // clap's conflicts_with should reject both flags simultaneously.
        let result = Cli::try_parse_from(["tome", "--verbose", "--quiet", "status"]);
        assert!(
            result.is_err(),
            "--verbose --quiet must conflict (parse error)"
        );
    }

    #[test]
    fn log_level_all_array_lists_every_variant() {
        // POLISH-04 sentinel: ALL must enumerate every variant.
        // The const_assert in cli.rs already enforces len()==3; this test
        // pins the actual variant set.
        let all = LogLevel::ALL;
        assert!(all.contains(&LogLevel::Quiet));
        assert!(all.contains(&LogLevel::Normal));
        assert!(all.contains(&LogLevel::Verbose));
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn log_level_default_trait_impl_is_normal() {
        assert_eq!(LogLevel::default(), LogLevel::Normal);
    }

    #[test]
    fn log_level_directive_maps_three_levels() {
        assert_eq!(LogLevel::Quiet.directive(), "warn");
        assert_eq!(LogLevel::Normal.directive(), "info");
        assert_eq!(LogLevel::Verbose.directive(), "debug");
    }

    // -- UX-02 / Plan 16-02 Task 2 — `tome migrate-library --yes` parsing --

    #[test]
    fn migrate_library_parses_yes_flag() {
        let cli = Cli::try_parse_from(["tome", "migrate-library", "--yes"]).unwrap();
        match cli.command {
            Command::MigrateLibrary { dry_run, yes } => {
                assert!(yes, "--yes must parse as yes: true");
                assert!(!dry_run);
            }
            _ => panic!("expected MigrateLibrary"),
        }
    }

    #[test]
    fn migrate_library_short_y_alias() {
        let cli = Cli::try_parse_from(["tome", "migrate-library", "-y"]).unwrap();
        match cli.command {
            Command::MigrateLibrary { yes, .. } => {
                assert!(yes, "-y short alias must set yes: true")
            }
            _ => panic!("expected MigrateLibrary"),
        }
    }

    #[test]
    fn migrate_library_yes_default_false() {
        let cli = Cli::try_parse_from(["tome", "migrate-library"]).unwrap();
        match cli.command {
            Command::MigrateLibrary { yes, dry_run } => {
                assert!(!yes, "yes must default to false when --yes is absent");
                assert!(!dry_run);
            }
            _ => panic!("expected MigrateLibrary"),
        }
    }

    #[test]
    fn migrate_library_dry_run_and_yes_compose() {
        let cli = Cli::try_parse_from(["tome", "migrate-library", "--dry-run", "--yes"]).unwrap();
        match cli.command {
            Command::MigrateLibrary { dry_run, yes } => {
                assert!(dry_run);
                assert!(yes);
            }
            _ => panic!("expected MigrateLibrary"),
        }
    }
}
