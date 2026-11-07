//! Decode native execution evidence into the existing typed comparison contract.
use super::errors::InventoryComparisonError as Failure;
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{ObservationDocument, RecordedMetadata, ReferenceProcess, ReportedSkips};

use crate::domain::diagnostics::service::compare_with_contract;
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ProjectContext, RunReport,
};

/// The run report an observation row records, when its execution accounting is explicit.
fn report(row: &ObservationDocument) -> Result<RunReport, Failure> {
    let RecordedMetadata::Recorded(metadata) = row.execution_metadata.clone() else {
        return Err(Failure::MissingExecutionCapability);
    };
    if matches!(metadata, ExecutionMetadata::Unknown)
        || matches!(&metadata, ExecutionMetadata::Unavailable { reason } if reason.0.trim().is_empty())
    {
        return Err(Failure::UndeclaredExecutionCapability);
    }
    let files = row
        .files
        .iter()
        .map(|file| {
            Some(file)
                .filter(|file| !file.0.is_empty())
                .map(|file| ryo::linter::RelativePath(file.0.as_str().into()))
                .ok_or(Failure::InvalidExecutionFile)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ReportedSkips::Reported(skips) = &row.skipped else {
        return Err(Failure::MissingExecutionSkips);
    };
    let skipped = skips
        .iter()
        .map(|skip| {
            Ok(ryo::linter::Skipped {
                rule: ryo::linter::RuleId::try_from(skip.rule.0.as_str())
                    .map_err(|_| Failure::InvalidSkippedRule)?,
                path: ryo::linter::RelativePath(skip.path.0.as_str().into()),
                reason: ryo::linter::SkipReason::SyntaxRejected,
            })
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    let coverage = row.coverage;
    if matches!(metadata, ExecutionMetadata::Unavailable { .. })
        && (coverage != CoverageAccounting::Unaccounted || !skipped.is_empty())
    {
        return Err(Failure::UnavailableCapabilityAsserted);
    }
    Ok(RunReport {
        execution_metadata: metadata,
        project_context: ProjectContext::NotRequested,
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
) -> Result<Comparison, Failure> {
    if matches!(reference.process_observation, ReferenceProcess::Observed(_)) {
        if contract != ComparisonContract::Diagnostics {
            return Err(Failure::ProcessFailureHasNoCompleteReport);
        }
        let candidate = report(candidate)?;
        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(&candidate)
            .map_err(|error| Failure::CandidateExecutionIncomplete(RefusalText(error.to_string())))?;
        return Ok(Comparison::Incomparable);
    }
    let reference = report(reference)?;
    let candidate = report(candidate)?;
    compare_with_contract(&reference, &candidate, contract)
        .map(|comparison| comparison.execution_metadata)
        .map_err(|_| Failure::ExecutionEvidenceUnsatisfied)
}
