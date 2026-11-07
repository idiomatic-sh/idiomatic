//! The keys and words the TOML adapter reads and writes.

/// The key no rule, spec or entry holds: every finding is an error, so nothing sets a level.
pub(super) const LEVEL_KEY: &str = "level";
/// The `from` a param defaults from.
pub(super) const DIRECTORY_NAME: &str = "directory-name";
/// The `from` a param defaults from: the workspace's name.
pub(super) const WORKSPACE_NAME: &str = "workspace-name";
/// The `default` that denies imports.
pub(super) const DENY: &str = "deny";
/// The head of a lock.
pub(super) const LOCK_HEAD: &str = "# Written by Tastemaker: the packages this workspace resolved,\n# direct and transitive, and the definition releases it uses. Do not edit it by hand.\n";
/// The key a Tastemaker file never holds: no file has a format version.
pub(super) const VERSION_KEY: &str = "version";
/// The version every lock writes first: it stays 1 until the first release.
pub(super) const LOCK_VERSION: i64 = 1;
/// The name of the workspace a scenario without its own is placed in.
pub(super) const SCENARIO_WORKSPACE: &str = "scenario";
/// The key of a scenario's sentence, which every scenario writes.
pub(super) const SCENARIO_KEY: &str = "scenario";
/// What the entry of a scenario's workspace says it places.
pub(super) const PLACED_ALONE: &str = "The definition under test, placed alone.";
/// How serde begins the refusal of a key a table does not take, which lists the keys it does.
pub(super) const UNKNOWN_FIELD: &str = "unknown field";
/// The format-2 keys of a `spec.toml` the reader takes and never offers.
pub(super) const SPEC_DROPPED: &[&str] = &["rules-exclude", "layers", "independent", "acyclic"];
