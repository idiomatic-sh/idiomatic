//! Convert one verification outcome into a durable boundary-owned observation.
use super::service::finding_values;
use crate::application::run::constants;
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

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
    let findings = report.map(|report| finding_values(&report.diagnostics));
    let files = report
        .map(|report| {
            report
                .files
                .iter()
                .map(|file| file.0.as_ref())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            case.request
                .files
                .iter()
                .map(|file| file.0.as_ref())
                .collect()
        });
    let skipped = report.map(|report| {
        report
            .skipped
            .iter()
            .map(|skip| {
                serde_json::json!({
                    constants::RULE: skip.rule.as_ref(),
                    constants::PATH: skip.path.0.as_ref(),
                    constants::REASON: match skip.reason {
                        ryo::linter::SkipReason::SyntaxRejected => constants::SYNTAX_REJECTED,
                    },
                })
            })
            .collect::<Vec<_>>()
    });
    let expected = finding_values(&case.expected.diagnostics);
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
    let metadata_error = metadata_identity.as_ref().err().map(ToString::to_string);
    let metadata_sha256 = metadata_identity.ok().flatten().map(|value| value.0);
    let project_error = project_identity.as_ref().err().map(ToString::to_string);
    let project_inputs_sha256 = project_identity.ok().flatten().map(|value| value.0);
    let source_error = source_identity
        .as_ref()
        .err()
        .map(ToString::to_string)
        .or_else(|| {
            metadata_error
                .map(|reason| format!("file metadata identity unavailable: {reason}"))
                .or_else(|| {
                    project_error
                        .map(|reason| format!("project input identity unavailable: {reason}"))
                })
        });
    let source_sha256 = source_identity.ok();
    let recorded_error = match (error, source_error.as_deref()) {
        (Some(error), Some(source)) => {
            Some(format!("{error}; source identity unavailable: {source}"))
        }
        (Some(error), None) => Some(error.to_owned()),
        (None, Some(source)) => Some(format!("source identity unavailable: {source}")),
        (None, None) => None,
    };
    let context_identity = case.request.context_manifest.as_ref().map(|manifest| {
        let hash = crate::outbound::context::adapters::local::adapter::fingerprint(manifest);
        serde_json::json!({ constants::MODE: constants::LISTED_DISK, constants::LOGICAL_ROOT: manifest.logical_root,
            constants::ACTUAL_ROOT: case.request.root.to_string_lossy(), constants::MANIFEST_HASH: hash.0 })
    });
    let mut record = serde_json::json!({
        constants::OBSERVATION_SCHEMA: constants::BENCHMARK_CASE_V2,
        constants::EXPECTATION_CONTRACT: constants::DIAGNOSTICS,
        constants::EXECUTION_METADATA: report.map(|report| crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&report.execution_metadata)),
        constants::RULE: case.rule.as_ref(),
        constants::NAME: case.name.to_string(),
        constants::SIDE: side,
        constants::FILES: files,
        constants::ACTUAL_REQUEST_ROOT: case.request.root.to_string_lossy(),
        constants::DIAGNOSTICS: findings,
        constants::SKIPPED: skipped,
        constants::COVERAGE: report.and_then(|report| {
            report.coverage.map(|coverage| {
                serde_json::json!({
                    constants::SELECTED_RULE_FILE_PAIRS: coverage.selected,
                    constants::EVALUATED_RULE_FILE_PAIRS: coverage.evaluated,
                    constants::SKIPPED_RULE_FILE_PAIRS: coverage.skipped,
                    constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: coverage.unknown_or_unread,
                })
            })
        }),
        constants::CONTEXT_MANIFEST: context_identity,
        constants::PROJECT_CONTEXT: report.and_then(|report| report.project_context.clone()).map(crate::outbound::context::adapters::local::schemas::ReceiptSchema::from),
        constants::EXPECTED: expected,
        constants::REFERENCE_EXPECTED: super::discrepancy::reference_expected(case).ok().map(|report| finding_values(&report.diagnostics)),
        constants::DOCUMENTED_REFERENCE_DISCREPANCY: case.reference_discrepancy.as_ref().is_some_and(|item| item.basis == ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect),
        constants::DOCUMENTED_TOOL_CONTRACT_DIFFERENCE: case.reference_discrepancy.as_ref().is_some_and(|item| item.basis == ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::ToolContract),
        constants::DISCREPANCY_VALIDATION: validation,
        constants::RAW_PARITY_MATCHES_INTENDED: report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        constants::DIAGNOSTICS_MATCH_INTENDED: report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        constants::SOURCE_SHA256: source_sha256,
        constants::PROJECT_INPUTS_SHA256: project_inputs_sha256,
        constants::ERROR: recorded_error,
        constants::CASE_PASSED: passed && source_error.is_none(),
        constants::COMPLETE: true,
    });
    if let Some(context) = &case.request.rule_context {
        record[constants::RULE_CONTEXT] = crate::outbound::worker::schemas::prepared_value(context);
    }
    if let Some(declarations) = &case.request.rule_observations {
        record[constants::RULE_OBSERVATIONS] =
            crate::outbound::worker::schemas::observation_declarations_value(declarations);
    }
    if let Some(selected) = &case.request.selected_rules {
        record[constants::SELECTED_RULES] =
            serde_json::json!(selected.iter().map(ToString::to_string).collect::<Vec<_>>());
    }
    if let Some(digest) = &metadata_sha256 {
        record[constants::FILE_METADATA_SHA256] = serde_json::json!(digest);
    }
    let mut raw_invocation = capture.map(|capture| capture.invocation.clone());
    if let Some(invocation) = &mut raw_invocation {
        invocation[constants::SOURCE_SHA256] = serde_json::json!(source_sha256);
        if let Some(digest) = &metadata_sha256 {
            invocation[constants::FILE_METADATA_SHA256] = serde_json::json!(digest);
        }
        record[constants::PROCESS_OBSERVATION] = serde_json::json!({
            constants::KIND:constants::PROCESS_FAILURE, constants::EXIT:invocation[constants::EXIT], constants::SIGNAL:invocation[constants::SIGNAL],
            constants::ACTUAL_REQUEST_ROOT:invocation[constants::ACTUAL_REQUEST_ROOT], constants::OBSERVER_NO_DEADLINE:invocation[constants::OBSERVER_NO_DEADLINE],
            constants::EXECUTABLE_SHA256:invocation[constants::EXECUTABLE_SHA256], constants::SETTINGS_SHA256:invocation[constants::SETTINGS_SHA256],
            constants::METADATA_ERROR:invocation.get(constants::METADATA_ERROR),
            constants::TEMPLATE_SHA256:super::process_discrepancy::expected(case).map(|process| super::process_discrepancy::output_digest(process).0),
        });
        record[constants::RAW_PARITY_MATCHES_INTENDED] = serde_json::json!(false);
    }
    let key = crate::domain::suite::process::service::observation_key(
        case.rule.as_ref(),
        &case.name.to_string(),
        side,
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
