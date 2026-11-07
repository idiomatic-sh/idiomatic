//! Decode native execution evidence into the existing typed comparison contract.
use super::errors::InventoryComparisonError as Failure;
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::documents::types::observation_document::ObservationDocument;

use crate::domain::diagnostics::service::compare_with_contract;
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ProjectContext, RunReport,
};

/// The run report an observation row records, when its execution accounting is explicit.
fn report(row: &ObservationDocument) -> Result<RunReport, Failure> {
    let metadata = row
        .execution_metadata
        .clone()
        .ok_or(Failure::MissingExecutionCapability)?;
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
    let skipped = row
        .skipped
        .as_ref()
        .ok_or(Failure::MissingExecutionSkips)?
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
    let coverage = row.coverage.map_or(
        CoverageAccounting::Unaccounted,
        CoverageAccounting::Accounted,
    );
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
    if reference.process_observation.is_some() {
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
