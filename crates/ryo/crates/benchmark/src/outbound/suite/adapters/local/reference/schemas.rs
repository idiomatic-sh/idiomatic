//! The shape of the reference discrepancy certificates authored beside a rule's cases, and their checked conversion into the port's claims.
use crate::outbound::suite::adapters::local::process::schemas::ProcessFailureSchema;
use crate::outbound::suite::types::{
    ClaimedDiscrepancy, ClaimedEvidence, ClaimedFinding, ClaimedIdentity, ClaimedMetadata,
    ClaimedProcess,
};
use ryo_benchmark_contracts::domain::documents::types::text::{PathText, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyReason;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    ContractSourcePath, LanguageVersion, ParserTarget, Rationale, SourceRevision, WitnessCase,
};
use serde::Deserialize;
use std::path::PathBuf;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// Serialized reference contract; semantic conversion belongs to the application.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(in super::super) struct ReferenceDiscrepancySchema {
    /// Explicit process expectation; absent preserves the existing diagnostic contract.
    #[serde(default)]
    process: Option<ProcessFailureSchema>,
    /// Exact reference findings.
    #[serde(default)]
    expected: Vec<ReferenceFindingSchema>,
    /// Immutable certificate inputs.
    identity: ReferenceIdentitySchema,
    /// Independent evidence files.
    evidence: Vec<EvidenceSchema>,
    /// Authored explanation.
    reason: String,
}
/// One complete explicitly identified diagnostic.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceFindingSchema {
    /// Rule identity, never inherited from an enclosing rule.
    rule: String,
    /// Source-relative path.
    file: String,
    /// UTF8 byte start.
    start: usize,
    /// UTF8 byte end.
    end: usize,
}
/// Exact certificate identities, validated by application policy.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct ReferenceIdentitySchema {
    /// Separate optional case metadata identity, mandatory when observations are declared.
    #[serde(default)]
    file_metadata_sha256: Option<String>,
    /// Reference name.
    tool: String,
    /// Pinned version.
    version: String,
    /// Originally observed approved native executable.
    observed_executable_sha256: String,
    /// Exact profile bytes.
    profile_sha256: String,
    /// Normalized effective invocation.
    settings_sha256: String,
    /// Source filenames and bytes.
    source_sha256: String,
    /// Complete reference expectation.
    reference_output_sha256: String,
    /// Intended candidate expectation.
    intended_output_sha256: String,
}
/// Independent artifact observation.
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    deny_unknown_fields
)]
enum EvidenceSchema {
    /// Versioned language specification.
    LanguageSpec {
        path: PathBuf,
        sha256: String,
        language_version: String,
        target_configuration: String,
        version_rationale: String,
    },
    /// Independently executed language implementation.
    IndependentLanguageRun {
        path: PathBuf,
        sha256: String,
        language_version: String,
        target_configuration: String,
        version_rationale: String,
    },
    /// The exact prior public API contract whose policy is asserted.
    PublicToolContract {
        path: PathBuf,
        sha256: String,
        source_revision: String,
        source_path: String,
        rationale: String,
    },
    /// An actual prior accepted worker exercising the same selection boundary.
    PriorWorkerRun {
        path: PathBuf,
        sha256: String,
        worker_sha256: String,
        control_source_sha256: String,
        control_query_sha256: String,
        control_profile_sha256: String,
        witness_case: String,
        rationale: String,
    },
}

fn sha(value: String) -> Sha256Text {
    Sha256Text(value)
}

impl From<ReferenceFindingSchema> for ClaimedFinding {
    fn from(schema: ReferenceFindingSchema) -> Self {
        Self {
            rule: RuleName(schema.rule),
            file: PathText(schema.file),
            start: SourceByteOffset(schema.start),
            end: SourceByteOffset(schema.end),
        }
    }
}

impl From<ReferenceIdentitySchema> for ClaimedIdentity {
    fn from(schema: ReferenceIdentitySchema) -> Self {
        Self {
            file_metadata_sha256: schema
                .file_metadata_sha256
                .map_or(ClaimedMetadata::Undeclared, |digest| {
                    ClaimedMetadata::Declared(sha(digest))
                }),
            tool: schema.tool.into(),
            version: schema.version.into(),
            observed_executable_sha256: sha(schema.observed_executable_sha256),
            profile_sha256: sha(schema.profile_sha256),
            settings_sha256: sha(schema.settings_sha256),
            source_sha256: sha(schema.source_sha256),
            reference_output_sha256: sha(schema.reference_output_sha256),
            intended_output_sha256: sha(schema.intended_output_sha256),
        }
    }
}

impl From<EvidenceSchema> for ClaimedEvidence {
    fn from(schema: EvidenceSchema) -> Self {
        match schema {
            EvidenceSchema::LanguageSpec {
                path,
                sha256,
                language_version,
                target_configuration,
                version_rationale,
            } => Self::LanguageSpec {
                path,
                sha256: sha(sha256),
                language_version: LanguageVersion(language_version),
                target_configuration: ParserTarget(target_configuration),
                version_rationale: Rationale(version_rationale),
            },
            EvidenceSchema::IndependentLanguageRun {
                path,
                sha256,
                language_version,
                target_configuration,
                version_rationale,
            } => Self::IndependentLanguageRun {
                path,
                sha256: sha(sha256),
                language_version: LanguageVersion(language_version),
                target_configuration: ParserTarget(target_configuration),
                version_rationale: Rationale(version_rationale),
            },
            EvidenceSchema::PublicToolContract {
                path,
                sha256,
                source_revision,
                source_path,
                rationale,
            } => Self::PublicToolContract {
                path,
                sha256: sha(sha256),
                source_revision: SourceRevision(source_revision),
                source_path: ContractSourcePath(source_path),
                rationale: Rationale(rationale),
            },
            EvidenceSchema::PriorWorkerRun {
                path,
                sha256,
                worker_sha256,
                control_source_sha256,
                control_query_sha256,
                control_profile_sha256,
                witness_case,
                rationale,
            } => Self::PriorWorkerRun {
                path,
                sha256: sha(sha256),
                worker_sha256: sha(worker_sha256),
                control_source_sha256: sha(control_source_sha256),
                control_query_sha256: sha(control_query_sha256),
                control_profile_sha256: sha(control_profile_sha256),
                witness_case: WitnessCase(witness_case),
                rationale: Rationale(rationale),
            },
        }
    }
}

impl TryFrom<ReferenceDiscrepancySchema> for ClaimedDiscrepancy {
    type Error = Error;

    fn try_from(schema: ReferenceDiscrepancySchema) -> Result<Self, Error> {
        Ok(Self {
            process: match schema.process {
                Some(process) => ClaimedProcess::Failure(ProcessFailure::try_from(process)?),
                None => ClaimedProcess::Absent,
            },
            expected: schema.expected.into_iter().map(Into::into).collect(),
            identity: schema.identity.into(),
            evidence: schema.evidence.into_iter().map(Into::into).collect(),
            reason: DiscrepancyReason(schema.reason),
        })
    }
}
