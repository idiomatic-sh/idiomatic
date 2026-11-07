//! Convert one verification outcome into a durable boundary-owned observation.
use crate::application::run::constants;
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{
    finding::{ByteOffset, Finding, SkippedPair},
    observation_document::{
        ContextManifest, ContextMode, ContextRun, ExitStatus, FileMetadataDigest, IntentMatch,
        ObservationDocument, ProcessObservation, ProjectContextRecord, ProjectInputsDigest,
        ReferenceExpectation, RuleCohort, RuleContextRecord, RuleObservationsRecord,
        SourceIdentity, ToolContractRecord,
    },
    reference_discrepancy::{DiscrepancyValidation, ExpectationKind},
    text::{
        CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, Side,
        YesNo,
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, ComparisonContract};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

fn findings(rows: &[BenchmarkFinding]) -> Vec<Finding> {
    rows.iter()
        .map(|row| Finding {
            rule: RuleName(row.rule.as_ref().to_owned()),
            file: PathText(row.file.0.as_ref().to_owned()),
            start: ByteOffset(u64::try_from(row.start.0).unwrap_or(u64::MAX)),
            end: ByteOffset(u64::try_from(row.end.0).unwrap_or(u64::MAX)),
        })
        .collect()
}

fn text(value: &serde_json::Value) -> Option<Sha256Text> {
    value.as_str().map(|text| Sha256Text(text.to_owned()))
}

/// The validation the discrepancy check produced, read field by field.
fn validation_document(proof: &serde_json::Value) -> Option<DiscrepancyValidation> {
    Some(DiscrepancyValidation {
        discrepancy_basis: match proof[constants::DISCREPANCY_BASIS].as_str() {
            Some(constants::TOOL_CONTRACT_DIFFERENCE) => DiscrepancyBasis::ToolContract,
            _ => DiscrepancyBasis::LanguageDefect,
        },
        process_observation_group_sha256: text(&proof[constants::PROCESS_OBSERVATION_GROUP_SHA256]),
        reference_expectation_kind: match proof[constants::REFERENCE_EXPECTATION_KIND].as_str() {
            Some(constants::PROCESS_FAILURE) => ExpectationKind::ProcessFailure,
            _ => ExpectationKind::Diagnostics,
        },
        settings_sha256: text(&proof[constants::SETTINGS_SHA256])?,
        actual_executable_sha256: text(&proof[constants::ACTUAL_EXECUTABLE_SHA256])?,
        profile_sha256: text(&proof[constants::PROFILE_SHA256])?,
        source_sha256: text(&proof[constants::SOURCE_SHA256])?,
        reference_output_sha256: text(&proof[constants::REFERENCE_OUTPUT_SHA256])?,
        intended_output_sha256: text(&proof[constants::INTENDED_OUTPUT_SHA256])?,
        file_metadata_sha256: text(&proof[constants::FILE_METADATA_SHA256]),
    })
}

/// The reference process a captured invocation records.
fn process_observation(case: &PreparedCase, invocation: &serde_json::Value) -> ProcessObservation {
    ProcessObservation {
        kind: ExpectationKind::ProcessFailure,
        exit: ExitStatus(invocation[constants::EXIT].as_i64().unwrap_or_default()),
        signal: invocation[constants::SIGNAL].as_i64().map(ExitStatus),
        actual_request_root: PathText(
            invocation[constants::ACTUAL_REQUEST_ROOT]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        ),
        observer_no_deadline: YesNo::from(
            invocation[constants::OBSERVER_NO_DEADLINE].as_bool() == Some(true),
        ),
        executable_sha256: text(&invocation[constants::EXECUTABLE_SHA256]),
        settings_sha256: text(&invocation[constants::SETTINGS_SHA256]),
        metadata_error: invocation[constants::METADATA_ERROR]
            .as_str()
            .map(|reason| ReasonText(reason.to_owned())),
        template_sha256: crate::domain::process_failure::service::expected(case)
            .map(|process| Sha256Text(crate::domain::process_failure::service::output_digest(process).0)),
        artifacts: None,
    }
}

/// Append one complete side record while preserving the unprojected report and source identity.
pub(super) fn write(
    suite: &dyn SuitePort,
    case: &PreparedCase,
    side: &str,
    report: Option<&RunReport>,
    error: Option<&str>,
    passed: bool,
    validation: Option<&serde_json::Value>,
    capture: Option<&super::case_execution::Capture>,
) -> Result<(), Error> {
    let Some(path) = suite.case_receipt_file() else {
        return Ok(());
    };
    let side = match side {
        constants::REFERENCE => Side::Reference,
        constants::CANDIDATE => Side::Candidate,
        _ => return Err(Error::Invalid(constants::UNKNOWN_OBSERVATION_SIDE.into())),
    };
    let files = report
        .map(|report| {
            report
                .files
                .iter()
                .map(|file| PathText(file.0.as_ref().to_owned()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            case.request
                .files
                .iter()
                .map(|file| PathText(file.0.as_ref().to_owned()))
                .collect()
        });
    let skipped = report.map(|report| {
        report
            .skipped
            .iter()
            .map(|skip| SkippedPair {
                rule: RuleName(skip.rule.as_ref().to_owned()),
                path: PathText(skip.path.0.as_ref().to_owned()),
            })
            .collect::<Vec<_>>()
    });
    let source_identity = suite.case_source_digest(
        crate::outbound::suite::types::ArtifactPath(&case.request.root),
        &crate::domain::suite::service::materialized_sources(&case.request),
    );
    let metadata_identity =
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata((case.request.file_metadata.as_ref()).map(|values| values.iter().map(|(path, executable)| FileMetadataEntry { path, executable: if *executable { FileExecutable::Set } else { FileExecutable::Unset } })));
    let project_identity = crate::domain::suite::project_inputs::digest(
        case.request.project_search_roots.as_deref(),
        &case.request.given_project_inventory,
    );
    let (metadata_sha256, metadata_error) = match metadata_identity {
        Ok(identity) => (identity.map(|value| value.0), None),
        Err(failure) => (None, Some(failure.to_string())),
    };
    let (project_inputs_sha256, project_error) = match project_identity {
        Ok(identity) => (identity.map(|value| value.0), None),
        Err(failure) => (None, Some(failure.to_string())),
    };
    let (source_sha256, source_failure) = match source_identity {
        Ok(identity) => (Some(identity), None),
        Err(failure) => (None, Some(failure.to_string())),
    };
    let source_error = source_failure
        .or_else(|| {
            metadata_error
                .map(|reason| format!("file metadata identity unavailable: {reason}"))
                .or_else(|| {
                    project_error
                        .map(|reason| format!("project input identity unavailable: {reason}"))
                })
        });
    let source_text = source_sha256.clone();
    let recorded_error = match (error, source_error.as_deref()) {
        (Some(error), Some(source)) => {
            Some(format!("{error}; source identity unavailable: {source}"))
        }
        (Some(error), None) => Some(error.to_owned()),
        (None, Some(source)) => Some(format!("source identity unavailable: {source}")),
        (None, None) => None,
    };
    let context_manifest = case.request.context_manifest.as_ref().map(|manifest| {
        let hash = crate::outbound::context::adapters::local::adapter::fingerprint(manifest);
        ContextManifest {
            mode: ContextMode::ListedDisk,
            logical_root: PathText(manifest.logical_root.to_string_lossy().into_owned()),
            actual_root: PathText(case.request.root.to_string_lossy().into_owned()),
            manifest_hash: Sha256Text(hash.0),
        }
    });
    let intended_matches = report.map(|actual| {
        YesNo::from(
            crate::domain::discrepancy::service::matches_intended(case, actual)
                == crate::domain::discrepancy::types::IntentMatch::Matches,
        )
    });
    let basis = case.reference_discrepancy.as_ref().map(|item| item.basis);
    let mut record = ObservationDocument {
        rule: RuleName(case.rule.as_ref().to_owned()),
        name: CaseName(case.name.to_string()),
        side,
        complete: YesNo::Yes,
        error: recorded_error.map(ReasonText),
        case_passed: YesNo::from(passed && source_error.is_none()),
        files,
        source_sha256: source_sha256.map_or(SourceIdentity::Unavailable, |digest| {
            SourceIdentity::Sealed(Sha256Text(digest))
        }),
        file_metadata_sha256: FileMetadataDigest::from(metadata_sha256.map(Sha256Text)),
        project_inputs_sha256: ProjectInputsDigest::from(project_inputs_sha256.map(Sha256Text)),
        expected: findings(&case.expected.diagnostics),
        reference_expected: crate::domain::discrepancy::service::reference_expected(case)
            .map_or(ReferenceExpectation::Underivable, |report| {
                ReferenceExpectation::Derived(findings(&report.diagnostics))
            }),
        diagnostics: report.map(|report| findings(&report.diagnostics)),
        skipped,
        observation_schema: SchemaName(constants::BENCHMARK_CASE_V2.to_owned()),
        expectation_contract: ComparisonContract::Diagnostics,
        execution_metadata: report.map(|report| report.execution_metadata.clone()),
        coverage: report.and_then(|report| report.coverage),
        context_manifest: context_manifest.map_or(ContextRun::Isolated, ContextRun::Listed),
        project_context: report
            .and_then(|report| report.project_context.clone())
            .and_then(|context| {
                serde_json::to_value(
                    crate::outbound::context::adapters::local::schemas::ReceiptSchema::from(
                        context,
                    ),
                )
                .ok()
            })
            .map_or(ProjectContextRecord::NotCarried, |value| {
                ProjectContextRecord::Carried(CanonicalJson(value.to_string()))
            }),
        rule_context: case
            .request
            .rule_context
            .as_ref()
            .map_or(RuleContextRecord::Undeclared, |context| {
                RuleContextRecord::Declared(CanonicalJson(
                    crate::outbound::worker::schemas::prepared_value(context).to_string(),
                ))
            }),
        rule_observations: case
            .request
            .rule_observations
            .as_ref()
            .map_or(RuleObservationsRecord::Undeclared, |declarations| {
                RuleObservationsRecord::Declared(CanonicalJson(
                    crate::outbound::worker::schemas::observation_declarations_value(
                        declarations,
                    )
                    .to_string(),
                ))
            }),
        selected_rules: case
            .request
            .selected_rules
            .as_ref()
            .map_or(RuleCohort::Undeclared, |selected| {
                RuleCohort::Declared(
                    selected
                        .iter()
                        .map(|rule| RuleName(rule.to_string()))
                        .collect(),
                )
            }),
        actual_request_root: Some(PathText(case.request.root.to_string_lossy().into_owned())),
        documented_reference_discrepancy: YesNo::from(
            basis == Some(DiscrepancyBasis::LanguageDefect),
        ),
        documented_tool_contract_difference: ToolContractRecord::Recorded(YesNo::from(
            basis == Some(DiscrepancyBasis::ToolContract),
        )),
        discrepancy_validation: validation.and_then(validation_document),
        raw_parity_matches_intended: intended_matches,
        diagnostics_match_intended: intended_matches.map_or(IntentMatch::Unreported, IntentMatch::Matched),
        process_observation: None,
    };
    let mut raw_invocation = capture.map(|capture| capture.invocation.clone());
    if let Some(invocation) = &mut raw_invocation {
        invocation[constants::SOURCE_SHA256] =
            serde_json::json!(source_text);
        if let FileMetadataDigest::Declared(digest) = &record.file_metadata_sha256 {
            invocation[constants::FILE_METADATA_SHA256] = serde_json::json!(digest.0);
        }
        record.process_observation = Some(process_observation(case, invocation));
        record.raw_parity_matches_intended = Some(YesNo::No);
    }
    let key = crate::domain::suite::process::service::observation_key(
        case.rule.as_ref(),
        &case.name.to_string(),
        match side {
            Side::Reference => constants::REFERENCE,
            Side::Candidate => constants::CANDIDATE,
        },
    )
    .map_err(Error::Invalid)?;
    let process = capture
        .zip(raw_invocation.as_ref())
        .map(
            |(capture, invocation)| crate::outbound::suite::types::process::CapturedProcess {
                key: &key,
                invocation,
                stdout: &capture.output.stdout.0,
                stderr: &capture.output.stderr.0,
            },
        );
    suite
        .record_case(
            crate::outbound::suite::types::ArtifactPath(Path::new(&path)),
            record,
            process,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if let Some(error) = source_error {
        return Err(Error::Invalid(format!(
            "source identity unavailable after raw capture: {error}"
        )));
    }
    Ok(())
}
