//! Decode native execution evidence into the existing typed comparison contract.
use super::constants;
use crate::outbound::suite::types::observation_document::ObservationDocument;

use crate::domain::diagnostics::service::compare_with_contract;
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::RunReport;

/// The run report an observation row records, when its execution accounting is explicit.
fn report(row: &ObservationDocument) -> Result<RunReport, String> {
    let metadata = row
        .execution_metadata
        .clone()
        .ok_or(constants::MISSING_OR_MALFORMED_EXECUTION_CAPABILITY)?;
    if matches!(metadata, ExecutionMetadata::Unknown)
        || matches!(&metadata, ExecutionMetadata::Unavailable { reason } if reason.trim().is_empty())
    {
        return Err(constants::EXECUTION_CAPABILITY_IS_NOT_EXPLICITLY_DECLA.into());
    }
    let files = row
        .files
        .iter()
        .map(|file| {
            Some(file)
                .filter(|file| !file.0.is_empty())
                .map(|file| ryo::linter::RelativePath(file.0.as_str().into()))
                .ok_or_else(|| constants::INVALID_EXECUTION_FILE.to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let skipped = row
        .skipped
        .as_ref()
        .ok_or(constants::MISSING_EXECUTION_SKIPS)?
        .iter()
        .map(|skip| {
            Ok(ryo::linter::Skipped {
                rule: ryo::linter::RuleId::try_from(skip.rule.0.as_str())
                    .map_err(|_| constants::INVALID_SKIPPED_RULE)?,
                path: ryo::linter::RelativePath(skip.path.0.as_str().into()),
                reason: ryo::linter::SkipReason::SyntaxRejected,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let coverage = row.coverage;
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

/// Compare a registered reference and candidate observation under the case contract.
///
/// # Errors
/// Refuses evidence that cannot satisfy the declared contract.
pub(crate) fn compare(
    reference: &ObservationDocument,
    candidate: &ObservationDocument,
    contract: ComparisonContract,
) -> Result<Comparison, String> {
    if reference.process_observation.is_some() {
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
