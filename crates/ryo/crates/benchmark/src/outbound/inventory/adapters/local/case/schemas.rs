// ignore: idiomatic.arch/file-length - one case document read and written: its decoding and the wire form the registration writes it back in share every field, and no placed role splits a document's two directions
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
use sha2::{Digest as _, Sha256};
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

/// A value as the authoring's canonical JSON writes it: keys sorted, compact, characters beyond ASCII unescaped.
#[derive(Clone, Debug, PartialEq, serde::Serialize, Deserialize)]
#[serde(untagged)]
pub(in super::super) enum Canonical {
    /// No value.
    Null,
    /// A flag.
    Flag(bool),
    /// A whole number.
    Integer(i64),
    /// A number with a fraction.
    Float(f64),
    /// A text.
    Text(String),
    /// Values in order.
    List(Vec<Canonical>),
    /// Values by key, in key order.
    Table(BTreeMap<String, Canonical>),
}

impl Canonical {
    /// The compact text of the value.
    pub(in super::super) fn text(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(|error| Error::Json(Box::new(error)))
    }
    /// The SHA-256 of the compact text.
    pub(in super::super) fn digest(&self) -> Result<String, Error> {
        Ok(format!("{:x}", Sha256::digest(self.text()?.as_bytes())))
    }
    /// The text, when the value is one.
    pub(in super::super) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Null
            | Self::Flag(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::List(_)
            | Self::Table(_) => None,
        }
    }
    /// The table, when the value is one.
    pub(in super::super) fn as_table(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Table(table) => Some(table),
            Self::Null
            | Self::Flag(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Text(_)
            | Self::List(_) => None,
        }
    }
    /// The list, when the value is one.
    pub(in super::super) fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items),
            Self::Null
            | Self::Flag(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Text(_)
            | Self::Table(_) => None,
        }
    }
    /// The whole number, when the value is one.
    pub(in super::super) fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(number) => Some(*number),
            Self::Null
            | Self::Flag(_)
            | Self::Float(_)
            | Self::Text(_)
            | Self::List(_)
            | Self::Table(_) => None,
        }
    }
}

impl TryFrom<toml::Value> for Canonical {
    type Error = Error;
    fn try_from(value: toml::Value) -> Result<Self, Error> {
        Ok(match value {
            toml::Value::String(text) => Self::Text(text),
            toml::Value::Integer(number) => Self::Integer(number),
            toml::Value::Float(number) => Self::Float(number),
            toml::Value::Boolean(flag) => Self::Flag(flag),
            toml::Value::Datetime(_) => {
                return Err(Error::Invalid(
                    super::super::constants::UNSUPPORTED_DOCUMENT_VALUE.into(),
                ));
            }
            toml::Value::Array(items) => Self::List(
                items
                    .into_iter()
                    .map(Self::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            toml::Value::Table(table) => Self::Table(
                table
                    .into_iter()
                    .map(|(key, value)| Ok((key, Self::try_from(value)?)))
                    .collect::<Result<_, Error>>()?,
            ),
        })
    }
}

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

/// A text value.
fn text(value: &str) -> Canonical {
    Canonical::Text(value.to_owned())
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

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
