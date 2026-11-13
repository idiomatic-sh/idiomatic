//! A registered case in the wire form the registration writes it back in: optional inputs are written only when declared.
use super::super::canonical::schemas::{Canonical, text};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::{
    CaseContextIdentity, CaseDocument, CaseReference, DeclaredScope, EntryState,
    ExpectationContract, FileMetadata, InventoryCompleteness, ProjectInventory,
    ProjectInventoryDocument,
};
use ryo_benchmark_contracts::domain::documents::types::finding::value::RecordedFinding;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    ContextMode, RuleCohort, RuleContextRecord, RuleObservationsRecord,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    FileMetadataDigest, ProjectInputsDigest,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::workloads::types::{CaseComparison, FileObservation};

/// A registered case as the authoring writes it: optional inputs are written only when declared.
#[derive(Debug, serde::Serialize)]
pub(in super::super) struct CaseWire {
    pub(in super::super) rule: String,
    pub(in super::super) name: Canonical,
    pub(in super::super) case_sha256: String,
    pub(in super::super) source_sha256: String,
    pub(in super::super) files: Vec<Canonical>,
    pub(in super::super) comparison: Canonical,
    pub(in super::super) expected: Canonical,
    pub(in super::super) project_context_mode: String,
    pub(in super::super) context_identity: Canonical,
    pub(in super::super) reference_discrepancy: Canonical,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) expectation_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) rule_context: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) rule_observations: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) selected_rules: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) file_observation: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) file_metadata: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) file_metadata_sha256: Option<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub(in super::super) inputs: Option<InputsWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) project_inputs_sha256: Option<String>,
}

/// A case's explicit project inputs: its search roots, null when only its inventory is declared, and its inventory.
#[derive(Debug, serde::Serialize)]
pub(in super::super) struct InputsWire {
    pub(in super::super) given_project_inventory: Canonical,
    pub(in super::super) project_search_roots: Canonical,
}

/// The canonical value of a case's raw declaration.
fn raw(text: &CanonicalJson) -> Result<Canonical, Error> {
    serde_json::from_str(&text.0).map_err(|error| Error::Json(Box::new(error)))
}

/// The wire spelling of a context mode.
fn mode(mode: ContextMode) -> &'static str {
    match mode {
        ContextMode::Isolated => super::super::constants::ISOLATED,
        ContextMode::ListedDisk => super::super::constants::LISTED_DISK,
    }
}

/// The findings of a case, as the authoring writes them.
fn findings(rows: &[RecordedFinding]) -> Canonical {
    raw_value(
        crate::outbound::inventory::adapters::local::discrepancy::schemas::findings_value(rows),
    )
}

/// A JSON value as a canonical value.
fn raw_value(value: serde_json::Value) -> Canonical {
    serde_json::from_value(value).unwrap_or(Canonical::Null)
}

/// A list of texts.
fn texts(items: impl Iterator<Item = String>) -> Canonical {
    Canonical::List(items.map(Canonical::Text).collect())
}

impl TryFrom<&CaseDocument> for CaseWire {
    type Error = Error;

    fn try_from(case: &CaseDocument) -> Result<Self, Error> {
        let declared_scope = !matches!(case.project_search_roots, DeclaredScope::Undeclared);
        let declared_inventory =
            !matches!(case.given_project_inventory, ProjectInventory::Undeclared);
        Ok(Self {
            rule: case.rule.0.clone(),
            name: text(&case.name.0),
            case_sha256: case.case_sha256.0.clone(),
            source_sha256: case.source_sha256.0.clone(),
            files: case.files.iter().map(|file| text(&file.0)).collect(),
            comparison: text(match case.comparison {
                CaseComparison::Rule => super::super::constants::RULE,
                CaseComparison::Full => super::super::constants::FULL,
            }),
            expected: findings(&case.expected),
            project_context_mode: mode(case.project_context_mode).to_owned(),
            context_identity: match &case.context_identity {
                CaseContextIdentity::Isolated => Canonical::Null,
                CaseContextIdentity::Listed(identity) => raw_value(serde_json::to_value(ContextIdentityWire {
                    mode: mode(identity.mode),
                    logical_root: &identity.logical_root.0,
                    manifest_hash: &identity.manifest_hash.0,
                    present_paths: identity.present_paths.iter().map(|path| path.0.as_str()).collect(),
                    missing_paths: identity.missing_paths.iter().map(|path| path.0.as_str()).collect(),
                }).map_err(|error| Error::Json(Box::new(error)))?),
            },
            reference_discrepancy: match &case.reference_discrepancy {
                CaseReference::Agrees => Canonical::Null,
                CaseReference::Differs(record) => raw_value(
                    crate::outbound::inventory::adapters::local::discrepancy::schemas::discrepancy_value(record),
                ),
            },
            expectation_contract: match case.expectation_contract {
                ExpectationContract::Declared(ComparisonContract::Diagnostics) => Some(super::super::constants::DIAGNOSTICS_CONTRACT.to_owned()),
                ExpectationContract::Declared(ComparisonContract::CompleteReport) => Some(super::super::constants::CONTRACT_COMPLETE_REPORT.to_owned()),
                ExpectationContract::Unwritten => None,
            },
            rule_context: match &case.rule_context {
                RuleContextRecord::Declared(text) => Some(raw(text)?),
                RuleContextRecord::Undeclared => None,
            },
            rule_observations: match &case.rule_observations {
                RuleObservationsRecord::Declared(text) => Some(raw(text)?),
                RuleObservationsRecord::Undeclared => None,
            },
            selected_rules: match &case.selected_rules {
                RuleCohort::Declared(rules) => Some(texts(rules.iter().map(|rule| rule.0.clone()))),
                RuleCohort::Undeclared => None,
            },
            file_observation: (case.file_observation == FileObservation::PosixExecutable)
                .then(|| text(super::super::constants::POSIX_EXECUTABLE)),
            file_metadata: match &case.file_metadata {
                FileMetadata::Declared(files) => Some(Canonical::Table(
                    files
                        .iter()
                        .map(|(file, executable)| (file.0.clone(), Canonical::Flag(*executable == FileExecutable::Set)))
                        .collect(),
                )),
                FileMetadata::Undeclared => None,
            },
            file_metadata_sha256: match &case.file_metadata_sha256 {
                FileMetadataDigest::Declared(digest) => Some(digest.0.clone()),
                FileMetadataDigest::Undeclared => None,
            },
            inputs: (declared_scope || declared_inventory).then(|| {
                let inventory = match &case.given_project_inventory {
                    ProjectInventory::Declared(inventory) => inventory.clone(),
                    ProjectInventory::Undeclared => ProjectInventoryDocument::default(),
                };
                InputsWire {
                    project_search_roots: match &case.project_search_roots {
                        DeclaredScope::Roots(roots) => texts(roots.iter().map(|root| root.0.clone())),
                        DeclaredScope::Undeclared => Canonical::Null,
                    },
                    given_project_inventory: raw_value(
                        serde_json::to_value(InventoryWire {
                            completeness: match inventory.completeness {
                                InventoryCompleteness::Partial => super::super::constants::PARTIAL,
                                InventoryCompleteness::CompleteWithinDeclaredScope => {
                                    super::super::constants::COMPLETE_SCOPE
                                }
                            },
                            entries: inventory
                                .entries
                                .iter()
                                .map(|entry| EntryWire {
                                    path: &entry.path.0,
                                    state: match entry.state {
                                        EntryState::Regular => super::super::constants::STATE_FILE,
                                        EntryState::Directory => super::super::constants::STATE_DIRECTORY,
                                        EntryState::Other => super::super::constants::STATE_OTHER,
                                        EntryState::Missing => super::super::constants::STATE_MISSING,
                                        EntryState::Unknown => super::super::constants::STATE_UNKNOWN,
                                    },
                                })
                                .collect(),
                        })
                        .unwrap_or_default(),
                    ),
                }
            }),
            project_inputs_sha256: match &case.project_inputs_sha256 {
                ProjectInputsDigest::Declared(digest) => Some(digest.0.clone()),
                ProjectInputsDigest::Undeclared => None,
            },
        })
    }
}

/// A listed context identity on the wire.
#[derive(serde::Serialize)]
struct ContextIdentityWire<'a> {
    mode: &'a str,
    logical_root: &'a str,
    manifest_hash: &'a str,
    present_paths: Vec<&'a str>,
    missing_paths: Vec<&'a str>,
}

/// A Given project inventory on the wire.
#[derive(serde::Serialize)]
struct InventoryWire<'a> {
    completeness: &'a str,
    entries: Vec<EntryWire<'a>>,
}

/// One Given project entry on the wire.
#[derive(serde::Serialize)]
struct EntryWire<'a> {
    path: &'a str,
    state: &'a str,
}

/// The wire form of a registered case.
pub(in super::super) fn case_value(case: &CaseDocument) -> Result<serde_json::Value, Error> {
    serde_json::to_value(CaseWire::try_from(case)?).map_err(|error| Error::Json(Box::new(error)))
}
