//! Convert one verification outcome into a durable boundary-owned observation.
use super::service::finding_values;
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
                    crate::application::run::constants::RULE: skip.rule.as_ref(),
                    crate::application::run::constants::PATH: skip.path.0.as_ref(),
                    crate::application::run::constants::REASON: match skip.reason {
                        ryo::linter::SkipReason::SyntaxRejected => crate::application::run::constants::SYNTAX_REJECTED,
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
        serde_json::json!({ crate::application::run::constants::MODE: crate::application::run::constants::LISTED_DISK, crate::application::run::constants::LOGICAL_ROOT: manifest.logical_root,
            crate::application::run::constants::ACTUAL_ROOT: case.request.root.to_string_lossy(), crate::application::run::constants::MANIFEST_HASH: hash.0 })
    });
    let mut record = serde_json::json!({
        crate::application::run::constants::OBSERVATION_SCHEMA: crate::application::run::constants::BENCHMARK_CASE_V2,
        crate::application::run::constants::EXPECTATION_CONTRACT: crate::application::run::constants::DIAGNOSTICS,
        crate::application::run::constants::EXECUTION_METADATA: report.map(|report| crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&report.execution_metadata)),
        crate::application::run::constants::RULE: case.rule.as_ref(),
        crate::application::run::constants::NAME: case.name.to_string(),
        crate::application::run::constants::SIDE: side,
        crate::application::run::constants::FILES: files,
        crate::application::run::constants::ACTUAL_REQUEST_ROOT: case.request.root.to_string_lossy(),
        crate::application::run::constants::DIAGNOSTICS: findings,
        crate::application::run::constants::SKIPPED: skipped,
        crate::application::run::constants::COVERAGE: report.and_then(|report| {
            report.coverage.map(|coverage| {
                serde_json::json!({
                    crate::application::run::constants::SELECTED_RULE_FILE_PAIRS: coverage.selected,
                    crate::application::run::constants::EVALUATED_RULE_FILE_PAIRS: coverage.evaluated,
                    crate::application::run::constants::SKIPPED_RULE_FILE_PAIRS: coverage.skipped,
                    crate::application::run::constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: coverage.unknown_or_unread,
                })
            })
        }),
        crate::application::run::constants::CONTEXT_MANIFEST: context_identity,
        crate::application::run::constants::PROJECT_CONTEXT: report.and_then(|report| report.project_context.clone()).map(crate::outbound::context::adapters::local::schemas::ReceiptSchema::from),
        crate::application::run::constants::EXPECTED: expected,
        crate::application::run::constants::REFERENCE_EXPECTED: super::discrepancy::reference_expected(case).ok().map(|report| finding_values(&report.diagnostics)),
        crate::application::run::constants::DOCUMENTED_REFERENCE_DISCREPANCY: case.reference_discrepancy.as_ref().is_some_and(|item| item.basis == ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect),
        crate::application::run::constants::DOCUMENTED_TOOL_CONTRACT_DIFFERENCE: case.reference_discrepancy.as_ref().is_some_and(|item| item.basis == ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::ToolContract),
        crate::application::run::constants::DISCREPANCY_VALIDATION: validation,
        crate::application::run::constants::RAW_PARITY_MATCHES_INTENDED: report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        crate::application::run::constants::DIAGNOSTICS_MATCH_INTENDED: report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        crate::application::run::constants::SOURCE_SHA256: source_sha256,
        crate::application::run::constants::PROJECT_INPUTS_SHA256: project_inputs_sha256,
        crate::application::run::constants::ERROR: recorded_error,
        crate::application::run::constants::CASE_PASSED: passed && source_error.is_none(),
        crate::application::run::constants::COMPLETE: true,
    });
    if let Some(context) = &case.request.rule_context {
        record[crate::application::run::constants::RULE_CONTEXT] =
            crate::outbound::worker::schemas::prepared_value(context);
    }
    if let Some(declarations) = &case.request.rule_observations {
        record[crate::application::run::constants::RULE_OBSERVATIONS] =
            crate::outbound::worker::schemas::observation_declarations_value(declarations);
    }
    if let Some(selected) = &case.request.selected_rules {
        record[crate::application::run::constants::SELECTED_RULES] =
            serde_json::json!(selected.iter().map(ToString::to_string).collect::<Vec<_>>());
    }
    if let Some(digest) = &metadata_sha256 {
        record[crate::application::run::constants::FILE_METADATA_SHA256] =
            serde_json::json!(digest);
    }
    let mut raw_invocation = capture.map(|capture| capture.invocation.clone());
    if let Some(invocation) = &mut raw_invocation {
        invocation[crate::application::run::constants::SOURCE_SHA256] =
            serde_json::json!(source_sha256);
        if let Some(digest) = &metadata_sha256 {
            invocation[crate::application::run::constants::FILE_METADATA_SHA256] =
                serde_json::json!(digest);
        }
        record[crate::application::run::constants::PROCESS_OBSERVATION] = serde_json::json!({
            crate::application::run::constants::KIND:crate::application::run::constants::PROCESS_FAILURE, crate::application::run::constants::EXIT:invocation[crate::application::run::constants::EXIT], crate::application::run::constants::SIGNAL:invocation[crate::application::run::constants::SIGNAL],
            crate::application::run::constants::ACTUAL_REQUEST_ROOT:invocation[crate::application::run::constants::ACTUAL_REQUEST_ROOT], crate::application::run::constants::OBSERVER_NO_DEADLINE:invocation[crate::application::run::constants::OBSERVER_NO_DEADLINE],
            crate::application::run::constants::EXECUTABLE_SHA256:invocation[crate::application::run::constants::EXECUTABLE_SHA256], crate::application::run::constants::SETTINGS_SHA256:invocation[crate::application::run::constants::SETTINGS_SHA256],
            crate::application::run::constants::METADATA_ERROR:invocation.get(crate::application::run::constants::METADATA_ERROR),
            crate::application::run::constants::TEMPLATE_SHA256:super::process_discrepancy::expected(case).map(|process| super::process_discrepancy::output_digest(process).0),
        });
        record[crate::application::run::constants::RAW_PARITY_MATCHES_INTENDED] =
            serde_json::json!(false);
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
