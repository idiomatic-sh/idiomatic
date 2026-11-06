//! The registered case a provider's authoring writes.
use super::discrepancy::ReferenceDiscrepancy;
use super::finding::Finding;
use super::text::{CanonicalJson, CaseName, PathText, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use std::collections::BTreeMap;

/// How much of the report a case compares.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Comparison {
    /// Diagnostics of the case's rule.
    #[default]
    Rule,
    /// The full report.
    Full,
}

/// Whether the case's sources are staged alone or in a listed project context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextMode {
    /// Only the case's own files.
    #[default]
    Isolated,
    /// The files of a declared project directory.
    ListedDisk,
}

/// The file observation a profile declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileObservation {
    /// The POSIX execute bit of every registered file.
    PosixExecutable,
}

/// The listed context a case was authored against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextIdentity {
    /// How the context is staged.
    pub mode: ContextMode,
    /// The root the sources are logically under.
    pub logical_root: PathText,
    /// Framed digest of the declared context bytes.
    pub manifest_hash: Sha256Text,
    /// Every declared present path.
    pub present_paths: Vec<PathText>,
    /// Every declared missing path.
    pub missing_paths: Vec<PathText>,
}

/// How complete a project inventory is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InventoryCompleteness {
    /// Entries listed are known; others are not.
    #[default]
    Partial,
    /// Every entry within the declared roots is listed.
    CompleteWithinDeclaredScope,
}

/// What an inventory entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryState {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// Another kind of entry.
    Other,
    /// Known to be absent.
    Missing,
    /// Not observed.
    Unknown,
}

/// One project entry a case declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectEntry {
    /// The entry's project-relative path.
    pub path: PathText,
    /// What the entry is.
    pub state: EntryState,
}

/// The project entries a case declares for namespace resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectInventoryDocument {
    /// How complete the listing is.
    pub completeness: InventoryCompleteness,
    /// The listed entries.
    pub entries: Vec<ProjectEntry>,
}

/// A registered case: identity, expectations and every declared input.
#[derive(Clone, Debug, Default)]
pub struct CaseDocument {
    /// The rule the case belongs to.
    pub rule: Option<RuleName>,
    /// The case's name within its rule.
    pub name: Option<CaseName>,
    /// Digest of the authored case text.
    pub case_sha256: Option<Sha256Text>,
    /// Digest of the case's sources.
    pub source_sha256: Option<Sha256Text>,
    /// The case's source files.
    pub files: Vec<PathText>,
    /// How much of the report the case compares.
    pub comparison: Comparison,
    /// The diagnostics the candidate must report.
    pub expected: Vec<Finding>,
    /// How the sources are staged.
    pub project_context_mode: ContextMode,
    /// The listed context, when the mode is listed-disk.
    pub context_identity: Option<ContextIdentity>,
    /// A documented difference from the reference tool.
    pub reference_discrepancy: Option<ReferenceDiscrepancy>,
    /// The evidence a caller requires, when declared.
    pub expectation_contract: Option<ComparisonContract>,
    /// The prepared rule context, compared for equality.
    pub rule_context: Option<CanonicalJson>,
    /// The declared rule observations, compared for equality.
    pub rule_observations: Option<CanonicalJson>,
    /// An explicit rule cohort.
    pub selected_rules: Option<Vec<RuleName>>,
    /// The file observation the profile declares.
    pub file_observation: Option<FileObservation>,
    /// The declared execute bit of each file.
    pub file_metadata: Option<BTreeMap<PathText, FileExecutable>>,
    /// Digest of the declared file metadata.
    pub file_metadata_sha256: Option<Sha256Text>,
    /// Explicit project search roots.
    pub project_search_roots: Option<Vec<PathText>>,
    /// The declared project entries.
    pub given_project_inventory: Option<ProjectInventoryDocument>,
    /// Digest of the declared project inputs.
    pub project_inputs_sha256: Option<Sha256Text>,
}
