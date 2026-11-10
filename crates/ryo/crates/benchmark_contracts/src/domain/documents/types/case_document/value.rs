//! The registered case a provider's authoring writes, as a plain value.
use crate::domain::artifacts::types::FileExecutable;
use crate::domain::documents::types::finding::value::RecordedFinding;
use crate::domain::documents::types::observation_document::value::{
    ContextMode, RuleCohort, RuleContextRecord, RuleObservationsRecord,
};
use crate::domain::documents::types::reference_discrepancy::value::DiscrepancyRecord;
use crate::domain::documents::types::text::value::{
    CaseName, FileMetadataDigest, PathText, ProjectInputsDigest, RuleName, Sha256Text,
};
use crate::domain::reports::types::ComparisonContract;
use crate::domain::workloads::types::{CaseComparison, FileObservation};
use std::collections::BTreeMap;

/// A documented difference from the reference tool, or its absence.
#[derive(Clone, Debug, PartialEq)]
pub enum CaseReference {
    /// The case documents no difference.
    Agrees,
    /// The case documents this difference.
    Differs(DiscrepancyRecord),
}

/// The listed context a case was authored against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseContextIdentity {
    /// The mode is isolated: no listed context is written.
    Isolated,
    /// The mode is listed-disk: the context the case was authored against.
    Listed(ContextIdentity),
}

/// The evidence a caller requires of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectationContract {
    /// The authoring never writes it: the evidence is diagnostics.
    Unwritten,
    /// The document states the evidence.
    Declared(ComparisonContract),
}

/// The declared execute bit of each file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileMetadata {
    /// The case declares no file metadata.
    Undeclared,
    /// The case declares the execute bit of each file.
    Declared(BTreeMap<PathText, FileExecutable>),
}

/// The project search roots a case or profile declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclaredScope {
    /// No roots are declared: namespace membership stays unknown.
    Undeclared,
    /// These roots are declared; an empty list is a declared empty scope.
    Roots(Vec<PathText>),
}

/// The project entries a case declares, written with the project search roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectInventory {
    /// No entries are declared.
    Undeclared,
    /// These entries are declared.
    Declared(ProjectInventoryDocument),
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
    Regular,
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
#[derive(Clone, Debug, PartialEq)]
pub struct CaseDocument {
    /// The rule the case belongs to.
    pub rule: RuleName,
    /// The case's name within its rule.
    pub name: CaseName,
    /// Digest of the authored case text.
    pub case_sha256: Sha256Text,
    /// Digest of the case's sources.
    pub source_sha256: Sha256Text,
    /// The case's source files.
    pub files: Vec<PathText>,
    /// How much of the report the case compares.
    pub comparison: CaseComparison,
    /// The diagnostics the candidate must report.
    pub expected: Vec<RecordedFinding>,
    /// How the sources are staged.
    pub project_context_mode: ContextMode,
    /// The listed context the case was authored against.
    pub context_identity: CaseContextIdentity,
    /// A documented difference from the reference tool: written when the case documents one.
    pub reference_discrepancy: CaseReference,
    /// The evidence a caller requires.
    pub expectation_contract: ExpectationContract,
    /// The prepared rule context.
    pub rule_context: RuleContextRecord,
    /// The declared rule observations.
    pub rule_observations: RuleObservationsRecord,
    /// The explicit rule cohort.
    pub selected_rules: RuleCohort,
    /// The file observation the profile declares.
    pub file_observation: FileObservation,
    /// The declared execute bit of each file.
    pub file_metadata: FileMetadata,
    /// Digest of the declared file metadata: written with the file metadata.
    pub file_metadata_sha256: FileMetadataDigest,
    /// The project search roots.
    pub project_search_roots: DeclaredScope,
    /// The declared project entries.
    pub given_project_inventory: ProjectInventory,
    /// Digest of the declared project inputs.
    pub project_inputs_sha256: ProjectInputsDigest,
}
