//! Shared source snapshots and observed lookup receipts.
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use crate::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo::linter::{RelativePath, RuleId};
use std::{collections::BTreeMap, path::PathBuf};
/// The exact bytes of one authored auxiliary file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextBytes(
    /// Unmodified file content, including malformed UTF-8.
    pub Vec<u8>,
);

/// Authored auxiliary files and missing paths, all relative to an isolated snapshot boundary.
#[derive(Clone, Debug, Default)]
pub struct CaseContext {
    /// Run root within the snapshot; its directory components retain their spelling.
    pub logical_root: PathBuf,
    /// Auxiliary bytes supplied to both tools but never linted as requested sources.
    pub auxiliary_files: BTreeMap<PathBuf, ContextBytes>,
    /// Paths explicitly sealed as absent.
    pub missing: Vec<PathBuf>,
}

/// Immutable expected state of a materialized context, independent of project lookup output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextManifest {
    /// Absolute snapshot boundary; dependencies must resolve inside it.
    pub boundary: PathBuf,
    /// Run root relative to the boundary, including its directory name.
    pub logical_root: PathBuf,
    /// Auxiliary paths and their content artifact hashes, relative to the boundary.
    pub auxiliary_files: BTreeMap<PathBuf, ArtifactDigest>,
    /// Explicit lint input bytes, relative to the same boundary, sealed independently of auxiliary files.
    pub source_files: BTreeMap<PathBuf, ArtifactDigest>,
    /// Paths explicitly sealed as missing, relative to the boundary.
    pub missing: Vec<PathBuf>,
}

/// The context actually used by one candidate invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectContextReceipt {
    /// Explicit candidate isolation mode.
    pub mode: ProjectContextMode,
    /// Run root's stable position within the materialized snapshot.
    pub logical_root: std::path::PathBuf,
    /// Actual physical run directory supplied to both tools.
    pub actual_root: std::path::PathBuf,
    /// Hash of declared files, physically validated observed absences and logical root.
    pub manifest_hash: ArtifactDigest,
    /// Every observed dependency, including those retained by warm reuse.
    pub dependencies:
        std::collections::BTreeMap<std::path::PathBuf, Option<ryo::linter::DependencyFingerprint>>,
}

/// How profile-default bindings choose producers; explicit case bindings are always declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationSelectionPolicy {
    /// Bind exactly the profile's declared producers, independent of diagnostic selection.
    Declared,
    /// Bind only producers in the resolved diagnostic cohort.
    Selected,
}

pub mod project_context_mode;
pub mod rule_observation_declarations;

/// A literal prefix the tool treats as an external rule identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalPrefix(
    /// The prefix text, compared literally.
    pub String,
);

/// The name of a hosted request-scope configuration table.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableName(
    /// The table's exact name.
    pub String,
);

/// Positive configuration knowledge, distinct from whether an executable producer completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleConfiguration {
    /// Literal prefixes the tool/query treats as external rule identifiers.
    pub external_prefixes: Vec<ExternalPrefix>,
    /// Exact requested relative files and the rules disabled there; these are not glob patterns.
    pub disabled_rules: BTreeMap<RelativePath, Vec<RuleId>>,
}

/// Explicit names of existing hosted request-scope configuration tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleContextTables {
    /// Selected configured rule IDs, one text column `code`.
    pub selected: TableName,
    /// Disabled file/rule pairs, columns `file` and `code`.
    pub disabled: TableName,
    /// Declared external prefixes, one text column `prefix`.
    pub external: TableName,
}

/// A profile opts into configuration inputs with required table names and explicit defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleContextDeclaration {
    /// Required semantic table names; no implicit fallback names exist.
    pub tables: RuleContextTables,
    /// Complete declared configuration when a case supplies no override.
    pub defaults: RuleConfiguration,
}

/// The resolved data transported to one candidate invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedRuleContext {
    /// Actual declared table names.
    pub tables: RuleContextTables,
    /// Complete configuration chosen from the same profile/case data as the reference.
    pub configuration: RuleConfiguration,
}
