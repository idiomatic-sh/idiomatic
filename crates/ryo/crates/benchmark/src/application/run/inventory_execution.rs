//! Decode native execution evidence into the existing typed comparison contract.
use serde_json::Value;

use crate::domain::diagnostics::service::compare_with_contract;
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::RuleCoverage;
use ryo_benchmark_contracts::domain::reports::types::RunReport;

pub(super) fn contract(value: &Value) -> Result<ComparisonContract, String> {
    match value.as_str() {
        Some("diagnostics") => Ok(ComparisonContract::Diagnostics),
        Some("complete-report") => Ok(ComparisonContract::CompleteReport),
        _ => Err("missing or invalid expectation contract".into()),
    }
}

fn report(row: &Value) -> Result<RunReport, String> {
    let metadata: crate::outbound::worker::schemas::ExecutionMetadataSchema =
        serde_json::from_value(row["execution_metadata"].clone())
            .map_err(|_| "missing or malformed execution capability")?;
    let metadata: ExecutionMetadata = metadata.into();
    if matches!(metadata, ExecutionMetadata::Unknown)
        || matches!(&metadata, ExecutionMetadata::Unavailable { reason } if reason.trim().is_empty())
    {
        return Err("execution capability is not explicitly declared".into());
    }
    let files = row["files"]
        .as_array()
        .ok_or("missing execution files")?
        .iter()
        .map(|file| {
            file.as_str()
                .filter(|file| !file.is_empty())
                .map(|file| ryo::linter::RelativePath(file.into()))
                .ok_or_else(|| "invalid execution file".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let skipped = row["skipped"]
        .as_array()
        .ok_or("missing execution skips")?
        .iter()
        .map(|skip| {
            if skip["reason"] != "syntax-rejected" {
                return Err("invalid execution skip reason".to_owned());
            }
            Ok(ryo::linter::Skipped {
                rule: ryo::linter::RuleId::try_from(
                    skip["rule"].as_str().ok_or("missing skipped rule")?,
                )
                .map_err(|_| "invalid skipped rule")?,
                path: ryo::linter::RelativePath(
                    skip["path"].as_str().ok_or("missing skipped path")?.into(),
                ),
                reason: ryo::linter::SkipReason::SyntaxRejected,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let coverage = if row["coverage"].is_null() {
        None
    } else {
        let count = |key| {
            row["coverage"][key]
                .as_u64()
                .and_then(|count| usize::try_from(count).ok())
                .ok_or_else(|| "malformed execution coverage".to_owned())
        };
        Some(RuleCoverage {
            selected: count("selected_rule_file_pairs")?,
            evaluated: count("evaluated_rule_file_pairs")?,
            skipped: count("skipped_rule_file_pairs")?,
            unknown_or_unread: count("unknown_or_unread_rule_file_pairs")?,
        })
    };
    if matches!(metadata, ExecutionMetadata::Unavailable { .. })
        && (coverage.is_some() || !skipped.is_empty())
    {
        return Err("unavailable execution capability contains asserted accounting".into());
    }
    Ok(RunReport {
        execution_metadata: metadata,
        project_context: None,
        diagnostics: Vec::new(),
        skipped,
        coverage,
        files,
    })
}

pub(super) fn compare(
    reference: &Value,
    candidate: &Value,
    contract: ComparisonContract,
) -> Result<Comparison, String> {
    if reference["process_observation"].is_object() {
        if contract != ComparisonContract::Diagnostics {
            return Err("process failure has no complete-report execution accounting".into());
        }
        let candidate = report(candidate)?;
        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(&candidate)
            .map_err(|error| error.to_string())?;
        return Ok(Comparison::Incomparable);
    }
    let reference = report(reference)?;
    let candidate = report(candidate)?;
    compare_with_contract(&reference, &candidate, contract)
        .map(|comparison| comparison.execution_metadata)
        .map_err(|_| "execution evidence does not satisfy the declared contract".into())
}

#[cfg(test)]
#[path = "inventory_execution_test.rs"]
mod tests;
