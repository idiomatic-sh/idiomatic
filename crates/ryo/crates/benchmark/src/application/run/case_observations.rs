//! Convert one verification outcome into a durable boundary-owned observation.
use super::service::finding_values;
use crate::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::RunReport},
    suite::types::PreparedCase,
};
use crate::outbound::suite::interface::SuitePort;
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
                    "rule": skip.rule.as_ref(),
                    "path": skip.path.0.as_ref(),
                    "reason": match skip.reason {
                        ryo::linter::SkipReason::SyntaxRejected => "syntax-rejected",
                    },
                })
            })
            .collect::<Vec<_>>()
    });
    let expected = finding_values(&case.expected.diagnostics);
    let source_identity = suite.case_source_digest(
        crate::outbound::suite::types::ArtifactPath(&case.request.root),
        &case
            .request
            .staged_sources
            .as_deref()
            .unwrap_or(&case.request.files)
            .iter()
            .filter(|file| {
                case.request.staged_sources.is_some()
                    || !case
                        .request
                        .context_manifest
                        .as_ref()
                        .is_some_and(|manifest| {
                            crate::domain::suite::service::declares_missing(manifest, file)
                        })
            })
            .map(|file| std::path::PathBuf::from(file.0.as_ref()))
            .collect::<Vec<_>>(),
    );
    let metadata_identity =
        crate::domain::suite::input_metadata::digest(case.request.file_metadata.as_ref());
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
        serde_json::json!({ "mode": "listed-disk", "logical_root": manifest.logical_root,
            "actual_root": case.request.root.to_string_lossy(), "manifest_hash": hash.0 })
    });
    let mut record = serde_json::json!({
        "observation_schema": "benchmark-case/v2",
        "expectation_contract": "diagnostics",
        "execution_metadata": report.map(|report| &report.execution_metadata),
        "rule": case.rule.as_ref(),
        "name": case.name.to_string(),
        "side": side,
        "files": files,
        "actual_request_root": case.request.root.to_string_lossy(),
        "diagnostics": findings,
        "skipped": skipped,
        "coverage": report.and_then(|report| {
            report.coverage.map(|coverage| {
                serde_json::json!({
                    "selected_rule_file_pairs": coverage.selected,
                    "evaluated_rule_file_pairs": coverage.evaluated,
                    "skipped_rule_file_pairs": coverage.skipped,
                    "unknown_or_unread_rule_file_pairs": coverage.unknown_or_unread,
                })
            })
        }),
        "context_manifest": context_identity,
        "project_context": report.and_then(|report| report.project_context.clone()).map(crate::outbound::context::adapters::local::schemas::ReceiptSchema::from),
        "expected": expected,
        "reference_expected": super::discrepancy::reference_expected(case).ok().map(|report| finding_values(&report.diagnostics)),
        "documented_reference_discrepancy": case.reference_discrepancy.as_ref().is_some_and(|item| item.basis == crate::domain::suite::types::DiscrepancyBasis::LanguageDefect),
        "documented_tool_contract_difference": case.reference_discrepancy.as_ref().is_some_and(|item| item.basis == crate::domain::suite::types::DiscrepancyBasis::ToolContract),
        "discrepancy_validation": validation,
        "raw_parity_matches_intended": report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        "diagnostics_match_intended": report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        "source_sha256": source_sha256,
        "project_inputs_sha256": project_inputs_sha256,
        "error": recorded_error,
        "case_passed": passed && source_error.is_none(),
        "complete": true,
    });
    if let Some(context) = &case.request.rule_context {
        record["rule_context"] = crate::domain::suite::rule_context::prepared_value(context);
    }
    if let Some(declarations) = &case.request.rule_observations {
        record["rule_observations"] = crate::domain::suite::observations::value(declarations);
    }
    if let Some(selected) = &case.request.selected_rules {
        record["selected_rules"] =
            serde_json::json!(selected.iter().map(ToString::to_string).collect::<Vec<_>>());
    }
    if let Some(digest) = &metadata_sha256 {
        record["file_metadata_sha256"] = serde_json::json!(digest);
    }
    let mut raw_invocation = capture.map(|capture| capture.invocation.clone());
    if let Some(invocation) = &mut raw_invocation {
        invocation["source_sha256"] = serde_json::json!(source_sha256);
        if let Some(digest) = &metadata_sha256 {
            invocation["file_metadata_sha256"] = serde_json::json!(digest);
        }
        record["process_observation"] = serde_json::json!({
            "kind":"process-failure", "exit":invocation["exit"], "signal":invocation["signal"],
            "actual_request_root":invocation["actual_request_root"], "observer_no_deadline":invocation["observer_no_deadline"],
            "executable_sha256":invocation["executable_sha256"], "settings_sha256":invocation["settings_sha256"],
            "metadata_error":invocation.get("metadata_error"),
            "template_sha256":super::process_discrepancy::expected(case).map(|process| super::process_discrepancy::output_digest(process).0),
        });
        record["raw_parity_matches_intended"] = serde_json::json!(false);
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
