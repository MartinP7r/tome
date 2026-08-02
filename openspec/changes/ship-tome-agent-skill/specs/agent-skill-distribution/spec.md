## ADDED Requirements

### Requirement: Official Tome operations skill
The repository SHALL provide one `using-tome` skill under `skills/` that guides agents through safe user setup, configuration, synchronization, diagnosis, and recovery using progressive disclosure.

#### Scenario: Package-manager-owned source
- **WHEN** an agent is asked to add a package-manager-owned skill directory
- **THEN** the skill directs it to inspect current state, choose the `managed` role, preview supported mutations, and verify after sync

#### Scenario: Troubleshoot missing skills
- **WHEN** an agent is asked to repair missing distributed skills
- **THEN** the skill starts with Tome status, config, doctor, and dry-run diagnostics before narrow repair and does not lead with generated manifest or lockfile edits

### Requirement: Cross-tool Git installation
The skill package SHALL be installable as a Tome Git source using repository `MartinP7r/tome`, subdirectory `skills`, and role `source`.

#### Scenario: Explicit nested source
- **WHEN** a user runs `tome add MartinP7r/tome --subdir skills --role source` followed by sync
- **THEN** Tome discovers `using-tome` and distributes it to configured targets according to existing machine preferences

### Requirement: Local directory addition
`tome add` SHALL classify explicitly path-shaped inputs as local directories, SHALL accept every role valid for `DirectoryType::Directory`, and SHALL preserve existing Git input behavior.

#### Scenario: Managed package-manager directory
- **WHEN** a user runs `tome add ~/.pfw/skills --role managed`
- **THEN** Tome creates a directory-type entry named `skills` with managed role and a portable local path

#### Scenario: Local input uses Git-only flags
- **WHEN** a user passes `--branch`, `--tag`, `--rev`, or `--subdir` with an explicitly local path
- **THEN** Tome rejects the command with an actionable error and leaves configuration unchanged

#### Scenario: Bare GitHub slug remains Git
- **WHEN** a user runs `tome add owner/repo`
- **THEN** Tome preserves existing GitHub slug expansion and Git source behavior regardless of filesystem contents

### Requirement: Claude plugin distribution
The repository SHALL expose the same skill package through a valid Claude plugin named `tome` in a valid marketplace named `tome`.

#### Scenario: Marketplace installation
- **WHEN** a user adds marketplace `MartinP7r/tome` and installs `tome@tome`
- **THEN** Claude Code validates the manifests and discovers the `using-tome` skill from the repository's root `skills/` directory

### Requirement: Canonical Tome data folder display
The status report SHALL expose canonical `tome_home` independently from `library_dir`, and the desktop status view SHALL identify it as the Tome data folder rather than deriving it from the library path.

#### Scenario: Custom library outside Tome home
- **WHEN** Tome home and the configured library are unrelated absolute paths
- **THEN** the desktop displays the canonical Tome home under `TOME DATA FOLDER` and the configured library under a separate `LIBRARY` row

#### Scenario: Data folder explanation
- **WHEN** the status view renders the Tome data folder
- **THEN** explanatory copy identifies it as portable Tome data and distinguishes machine settings under `~/.config/tome`

### Requirement: Interactive init recommendation
Interactive `tome init` SHALL offer the official skill as a default-selected Git/source entry, display the equivalent standalone add command, and disclose that post-init sync clones the repository immediately.

#### Scenario: User accepts recommendation
- **WHEN** interactive init has no equivalent source or conflicting `tome-skills` name and the user accepts the default-yes prompt
- **THEN** the generated config contains `tome-skills` with the official URL, Git type, source role, and `subdir = "skills"`

#### Scenario: Equivalent source already exists
- **WHEN** an existing configuration already contains the official Git source scoped to `skills`
- **THEN** init does not prompt for or insert a duplicate source

#### Scenario: Recommendation name is occupied
- **WHEN** `tome-skills` names a different existing directory
- **THEN** init preserves that entry and does not overwrite it

### Requirement: Canonical init data-folder selection
Interactive init SHALL resolve the selected Tome data folder before existing-config detection and SHALL use that same path for configuration save and post-init synchronization.

#### Scenario: Existing custom repository
- **WHEN** a user selects a custom repository containing `.tome/tome.toml`
- **THEN** init detects the existing configuration and offers the normal brownfield choices without overwriting it

#### Scenario: New custom data folder
- **WHEN** a user selects a new custom data folder and continues setup
- **THEN** init saves configuration and constructs post-init sync paths from that selected folder rather than the initial default

#### Scenario: Data folder copy
- **WHEN** Step 0 prompts for the storage root
- **THEN** it calls the location the `Tome data folder` and distinguishes it from machine-local settings under `~/.config/tome`

### Requirement: Noninteractive initialization preservation
`tome init --no-input` MUST omit the official repository recommendation and MUST NOT introduce a clone or network dependency for that source.

#### Scenario: Empty noninteractive home
- **WHEN** init runs with `--no-input` against an empty home
- **THEN** generated configuration has no `tome-skills` entry and output contains no interactive recommendation prompt

### Requirement: Deferred automatic subdirectory adoption
This change MUST retain automatic `skills/` subdirectory adoption as separate pending work and MUST NOT modify production `tome add` behavior to implement it.

#### Scenario: Follow-up remains traceable
- **WHEN** the agent-skill change is complete
- **THEN** the existing auto-subdirectory task includes a regression criterion for `MartinP7r/tome` without changing the current requirement for explicit `--subdir skills`
