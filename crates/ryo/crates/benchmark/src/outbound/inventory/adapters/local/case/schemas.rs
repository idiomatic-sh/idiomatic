//! One case a provider's authoring registers.
use crate::outbound::inventory::adapters::local::types::{CaseText, DiscrepancyText};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::{
    CaseContextIdentity, CaseDocument, CaseReference, ContextIdentity, DeclaredScope, EntryState,
    ExpectationContract, FileMetadata, InventoryCompleteness, ProjectEntry, ProjectInventory,
    ProjectInventoryDocument,
};
use ryo_benchmark_contracts::domain::documents::types::finding::value::{
    FindingOffset, RecordedFinding,
};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    ContextMode, RuleCohort, RuleContextRecord, RuleObservationsRecord,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::DiscrepancyRecord;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, CaseName, PathText, RuleName, Sha256Text,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    FileMetadataDigest, ProjectInputsDigest,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::workloads::types::{CaseComparison, FileObservation};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The digest of an optional wire text.
fn sha(value: Option<String>) -> Option<Sha256Text> {
    value.map(Sha256Text)
}
/// The path a document spells.
fn path(value: String) -> PathText {
    PathText(value)
}
/// The canonical text of an optional wire value.
fn canonical(value: Option<serde_json::Value>) -> Option<CanonicalJson> {
    value.map(|value| CanonicalJson(value.to_string()))
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ComparisonSchema {
    Rule,
    Full,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContextModeSchema {
    Isolated,
    ListedDisk,
}

impl From<ContextModeSchema> for ContextMode {
    fn from(wire: ContextModeSchema) -> Self {
        match wire {
            ContextModeSchema::Isolated => Self::Isolated,
            ContextModeSchema::ListedDisk => Self::ListedDisk,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContractSchema {
    Diagnostics,
    CompleteReport,
}

impl From<ContractSchema> for ComparisonContract {
    fn from(wire: ContractSchema) -> Self {
        match wire {
            ContractSchema::Diagnostics => Self::Diagnostics,
            ContractSchema::CompleteReport => Self::CompleteReport,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FileObservationSchema {
    PosixExecutable,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextIdentitySchema {
    mode: ContextModeSchema,
    logical_root: String,
    manifest_hash: String,
    present_paths: Vec<String>,
    missing_paths: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CompletenessSchema {
    #[default]
    Partial,
    CompleteWithinDeclaredScope,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntrySchema {
    path: String,
    state: EntryStateSchema,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventorySchema {
    #[serde(default)]
    completeness: CompletenessSchema,
    #[serde(default)]
    entries: Vec<EntrySchema>,
}

/// One case a provider's authoring registers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseSchema {
    rule: String,
    name: String,
    case_sha256: String,
    source_sha256: String,
    files: Vec<String>,
    comparison: ComparisonSchema,
    expected: Vec<FindingSchema>,
    project_context_mode: ContextModeSchema,
    context_identity: Option<ContextIdentitySchema>,
    reference_discrepancy: Option<serde_json::Value>,
    expectation_contract: Option<ContractSchema>,
    rule_context: Option<serde_json::Value>,
    rule_observations: Option<serde_json::Value>,
    selected_rules: Option<Vec<String>>,
    file_observation: Option<FileObservationSchema>,
    file_metadata: Option<BTreeMap<String, bool>>,
    file_metadata_sha256: Option<String>,
    project_search_roots: Option<Vec<String>>,
    given_project_inventory: Option<InventorySchema>,
    project_inputs_sha256: Option<String>,
}

impl TryFrom<CaseSchema> for CaseDocument {
    type Error = Error;

    fn try_from(wire: CaseSchema) -> Result<Self, Error> {
        Ok(Self {
            rule: RuleName(wire.rule),
            name: CaseName(wire.name),
            case_sha256: Sha256Text(wire.case_sha256),
            source_sha256: Sha256Text(wire.source_sha256),
            files: wire.files.into_iter().map(path).collect(),
            comparison: match wire.comparison {
                ComparisonSchema::Full => CaseComparison::Full,
                ComparisonSchema::Rule => CaseComparison::Rule,
            },
            expected: wire
                .expected
                .into_iter()
                .map(RecordedFinding::from)
                .collect(),
            project_context_mode: wire.project_context_mode.into(),
            context_identity: wire.context_identity.map_or(
                CaseContextIdentity::Isolated,
                |identity| {
                    CaseContextIdentity::Listed(ContextIdentity {
                        mode: identity.mode.into(),
                        logical_root: path(identity.logical_root),
                        manifest_hash: Sha256Text(identity.manifest_hash),
                        present_paths: identity.present_paths.into_iter().map(path).collect(),
                        missing_paths: identity.missing_paths.into_iter().map(path).collect(),
                    })
                },
            ),
            reference_discrepancy: wire
                .reference_discrepancy
                .map(|value| DiscrepancyRecord::try_from(DiscrepancyText(&value.to_string())))
                .transpose()?
                .map_or(CaseReference::Agrees, CaseReference::Differs),
            expectation_contract: wire
                .expectation_contract
                .map_or(ExpectationContract::Unwritten, |wire| {
                    ExpectationContract::Declared(wire.into())
                }),
            rule_context: canonical(wire.rule_context)
                .map_or(RuleContextRecord::Undeclared, RuleContextRecord::Declared),
            rule_observations: canonical(wire.rule_observations).map_or(
                RuleObservationsRecord::Undeclared,
                RuleObservationsRecord::Declared,
            ),
            selected_rules: wire.selected_rules.map_or(RuleCohort::Undeclared, |rules| {
                RuleCohort::Declared(rules.into_iter().map(RuleName).collect())
            }),
            file_observation: wire
                .file_observation
                .map_or(FileObservation::Undeclared, |value| match value {
                    FileObservationSchema::PosixExecutable => FileObservation::PosixExecutable,
                }),
            file_metadata: wire
                .file_metadata
                .map_or(FileMetadata::Undeclared, |values| {
                    FileMetadata::Declared(
                        values
                            .into_iter()
                            .map(|(file, executable)| {
                                (
                                    path(file),
                                    if executable {
                                        FileExecutable::Set
                                    } else {
                                        FileExecutable::Unset
                                    },
                                )
                            })
                            .collect(),
                    )
                }),
            file_metadata_sha256: FileMetadataDigest::from(sha(wire.file_metadata_sha256)),
            project_search_roots: wire
                .project_search_roots
                .map_or(DeclaredScope::Undeclared, |roots| {
                    DeclaredScope::Roots(roots.into_iter().map(path).collect())
                }),
            given_project_inventory: wire.given_project_inventory.map_or(
                ProjectInventory::Undeclared,
                |inventory| {
                    ProjectInventory::Declared(ProjectInventoryDocument {
                        completeness: match inventory.completeness {
                            CompletenessSchema::Partial => InventoryCompleteness::Partial,
                            CompletenessSchema::CompleteWithinDeclaredScope => {
                                InventoryCompleteness::CompleteWithinDeclaredScope
                            }
                        },
                        entries: inventory
                            .entries
                            .into_iter()
                            .map(|entry| ProjectEntry {
                                path: path(entry.path),
                                state: match entry.state {
                                    EntryStateSchema::File => EntryState::Regular,
                                    EntryStateSchema::Directory => EntryState::Directory,
                                    EntryStateSchema::Other => EntryState::Other,
                                    EntryStateSchema::Missing => EntryState::Missing,
                                    EntryStateSchema::Unknown => EntryState::Unknown,
                                },
                            })
                            .collect(),
                    })
                },
            ),
            project_inputs_sha256: ProjectInputsDigest::from(sha(wire.project_inputs_sha256)),
        })
    }
}

/// One diagnostic of a registered case.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingSchema {
    rule: String,
    file: String,
    start: u64,
    end: u64,
}

impl From<FindingSchema> for RecordedFinding {
    fn from(wire: FindingSchema) -> Self {
        Self {
            rule: RuleName(wire.rule),
            file: PathText(wire.file),
            span: ryo_benchmark_contracts::domain::documents::types::finding::value::RecordedSpan {
                start: FindingOffset(wire.start),
                end: FindingOffset(wire.end),
            },
        }
    }
}

impl TryFrom<CaseText<'_>> for CaseDocument {
    type Error = Error;

    fn try_from(text: CaseText<'_>) -> Result<Self, Error> {
        serde_json::from_str::<CaseSchema>(text.0)
            .map_err(|error| Error::Json(Box::new(error)))?
            .try_into()
    }
}

/// The canonical text of a case's raw declaration, as a value.
fn raw(text: &CanonicalJson) -> Result<serde_json::Value, Error> {
    serde_json::from_str(&text.0).map_err(|error| Error::Json(Box::new(error)))
}

/// The wire form of a registered case, as the authoring writes it: optional inputs are written only when declared.
pub(in super::super) fn case_value(case: &CaseDocument) -> Result<serde_json::Value, Error> {
    let mut written = serde_json::Map::new();
    let mut put = |key: &str, value: serde_json::Value| {
        let _previous = written.insert(key.to_owned(), value);
    };
    put("rule", case.rule.0.clone().into());
    put("name", case.name.0.clone().into());
    put("case_sha256", case.case_sha256.0.clone().into());
    put("source_sha256", case.source_sha256.0.clone().into());
    put("files", case.files.iter().map(|file| file.0.clone()).collect());
    put(
        "comparison",
        match case.comparison {
            CaseComparison::Rule => "rule",
            CaseComparison::Full => "full",
        }
        .into(),
    );
    put(
        "expected",
        crate::outbound::inventory::adapters::local::discrepancy::schemas::findings_value(
            &case.expected,
        ),
    );
    put("project_context_mode", mode(case.project_context_mode).into());
    put(
        "context_identity",
        match &case.context_identity {
            CaseContextIdentity::Isolated => serde_json::Value::Null,
            CaseContextIdentity::Listed(identity) => serde_json::json!({
                "mode": mode(identity.mode),
                "logical_root": identity.logical_root.0,
                "manifest_hash": identity.manifest_hash.0,
                "present_paths": identity.present_paths.iter().map(|path| path.0.clone()).collect::<Vec<_>>(),
                "missing_paths": identity.missing_paths.iter().map(|path| path.0.clone()).collect::<Vec<_>>(),
            }),
        },
    );
    put(
        "reference_discrepancy",
        match &case.reference_discrepancy {
            CaseReference::Agrees => serde_json::Value::Null,
            CaseReference::Differs(record) => {
                crate::outbound::inventory::adapters::local::discrepancy::schemas::discrepancy_value(
                    record,
                )
            }
        },
    );
    if let ExpectationContract::Declared(contract) = case.expectation_contract {
        put(
            "expectation_contract",
            match contract {
                ComparisonContract::Diagnostics => "diagnostics",
                ComparisonContract::CompleteReport => "complete-report",
            }
            .into(),
        );
    }
    if let RuleContextRecord::Declared(text) = &case.rule_context {
        put("rule_context", raw(text)?);
    }
    if let RuleObservationsRecord::Declared(text) = &case.rule_observations {
        put("rule_observations", raw(text)?);
    }
    if let RuleCohort::Declared(rules) = &case.selected_rules {
        put("selected_rules", rules.iter().map(|rule| rule.0.clone()).collect());
    }
    if case.file_observation == FileObservation::PosixExecutable {
        put("file_observation", "posix-executable".into());
    }
    if let FileMetadata::Declared(files) = &case.file_metadata {
        put(
            "file_metadata",
            files
                .iter()
                .map(|(file, executable)| (file.0.clone(), (*executable == FileExecutable::Set).into()))
                .collect::<serde_json::Map<_, _>>()
                .into(),
        );
    }
    if let FileMetadataDigest::Declared(digest) = &case.file_metadata_sha256 {
        put("file_metadata_sha256", digest.0.clone().into());
    }
    let declared_scope = !matches!(case.project_search_roots, DeclaredScope::Undeclared);
    let declared_inventory = !matches!(case.given_project_inventory, ProjectInventory::Undeclared);
    if declared_scope || declared_inventory {
        put(
            "project_search_roots",
            match &case.project_search_roots {
                DeclaredScope::Roots(roots) => roots.iter().map(|root| root.0.clone()).collect(),
                DeclaredScope::Undeclared => serde_json::Value::Null,
            },
        );
        let inventory = match &case.given_project_inventory {
            ProjectInventory::Declared(inventory) => inventory.clone(),
            ProjectInventory::Undeclared => ProjectInventoryDocument::default(),
        };
        put(
            "given_project_inventory",
            serde_json::json!({
                "completeness": match inventory.completeness {
                    InventoryCompleteness::Partial => "partial",
                    InventoryCompleteness::CompleteWithinDeclaredScope => "complete-within-declared-scope",
                },
                "entries": inventory.entries.iter().map(|entry| serde_json::json!({
                    "path": entry.path.0,
                    "state": match entry.state {
                        EntryState::Regular => "file",
                        EntryState::Directory => "directory",
                        EntryState::Other => "other",
                        EntryState::Missing => "missing",
                        EntryState::Unknown => "unknown",
                    },
                })).collect::<Vec<_>>(),
            }),
        );
    }
    if let ProjectInputsDigest::Declared(digest) = &case.project_inputs_sha256 {
        put("project_inputs_sha256", digest.0.clone().into());
    }
    Ok(written.into())
}

/// The wire spelling of a context mode.
fn mode(mode: ContextMode) -> &'static str {
    match mode {
        ContextMode::Isolated => "isolated",
        ContextMode::ListedDisk => "listed-disk",
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
