//! Decode native execution evidence into the existing typed comparison contract.
use crate::application::run::constants;
use serde_json::Value;

use crate::domain::diagnostics::service::compare_with_contract;
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::RuleCoverage;
use ryo_benchmark_contracts::domain::reports::types::RunReport;

pub(crate) fn contract(value: &Value) -> Result<ComparisonContract, String> {
    match value.as_str() {
        Some(constants::DIAGNOSTICS) => Ok(ComparisonContract::Diagnostics),
        Some(constants::COMPLETE_REPORT) => Ok(ComparisonContract::CompleteReport),
        _ => Err(constants::MISSING_OR_INVALID_EXPECTATION_CONTRACT.into()),
    }
}

fn report(row: &Value) -> Result<RunReport, String> {
    let metadata: crate::outbound::worker::schemas::ExecutionMetadataSchema =
        serde_json::from_value(row[constants::EXECUTION_METADATA].clone())
            .map_err(|_| constants::MISSING_OR_MALFORMED_EXECUTION_CAPABILITY)?;
    let metadata: ExecutionMetadata = metadata.into();
    if matches!(metadata, ExecutionMetadata::Unknown)
        || matches!(&metadata, ExecutionMetadata::Unavailable { reason } if reason.trim().is_empty())
    {
        return Err(constants::EXECUTION_CAPABILITY_IS_NOT_EXPLICITLY_DECLA.into());
    }
    let files = row[constants::FILES]
        .as_array()
        .ok_or(constants::MISSING_EXECUTION_FILES)?
        .iter()
        .map(|file| {
            file.as_str()
                .filter(|file| !file.is_empty())
                .map(|file| ryo::linter::RelativePath(file.into()))
                .ok_or_else(|| constants::INVALID_EXECUTION_FILE.to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let skipped = row[constants::SKIPPED]
        .as_array()
        .ok_or(constants::MISSING_EXECUTION_SKIPS)?
        .iter()
        .map(|skip| {
            if skip[constants::REASON] != constants::SYNTAX_REJECTED {
                return Err(constants::INVALID_EXECUTION_SKIP_REASON.to_owned());
            }
            Ok(ryo::linter::Skipped {
                rule: ryo::linter::RuleId::try_from(
                    skip[constants::RULE]
                        .as_str()
                        .ok_or(constants::MISSING_SKIPPED_RULE)?,
                )
                .map_err(|_| constants::INVALID_SKIPPED_RULE)?,
                path: ryo::linter::RelativePath(
                    skip[constants::PATH]
                        .as_str()
                        .ok_or(constants::MISSING_SKIPPED_PATH)?
                        .into(),
                ),
                reason: ryo::linter::SkipReason::SyntaxRejected,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let coverage = if row[constants::COVERAGE].is_null() {
        None
    } else {
        let count = |key| {
            row[constants::COVERAGE][key]
                .as_u64()
                .and_then(|count| usize::try_from(count).ok())
                .ok_or_else(|| constants::MALFORMED_EXECUTION_COVERAGE.to_owned())
        };
        Some(RuleCoverage {
            selected: count(constants::SELECTED_RULE_FILE_PAIRS)?,
            evaluated: count(constants::EVALUATED_RULE_FILE_PAIRS)?,
            skipped: count(constants::SKIPPED_RULE_FILE_PAIRS)?,
            unknown_or_unread: count(constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS)?,
        })
    };
    if matches!(metadata, ExecutionMetadata::Unavailable { .. })
        && (coverage.is_some() || !skipped.is_empty())
    {
        return Err(constants::UNAVAILABLE_EXECUTION_CAPABILITY_CONTAINS_AS.into());
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

pub(crate) fn compare(
    reference: &Value,
    candidate: &Value,
    contract: ComparisonContract,
) -> Result<Comparison, String> {
    if reference[constants::PROCESS_OBSERVATION].is_object() {
        if contract != ComparisonContract::Diagnostics {
            return Err(constants::PROCESS_FAILURE_HAS_NO_COMPLETE_REPORT.into());
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
        .map_err(|_| constants::EXECUTION_EVIDENCE_DOES_NOT_SATISFY_THE.into())
}

#[cfg(test)]
#[path = "inventory_execution_test.rs"]
mod tests;
