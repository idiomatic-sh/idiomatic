//! The comparison evidence line, as JSON on the comparison stream.
use super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    Comparison, ComparisonContract, ExecutionMetadata, ReportComparison, RunReport,
};
use serde::Serialize;

/// One producer's execution metadata as the comparison evidence spells it.
#[derive(Serialize)]
#[serde(tag = "availability", rename_all = "kebab-case")]
enum ExecutionMetadataSchema<'a> {
    Unknown {},
    Unavailable { reason: &'a str },
    Complete {},
}
impl<'a> From<&'a ExecutionMetadata> for ExecutionMetadataSchema<'a> {
    fn from(value: &'a ExecutionMetadata) -> Self {
        match value {
            ExecutionMetadata::Unknown => Self::Unknown {},
            ExecutionMetadata::Unavailable { reason } => Self::Unavailable { reason: &reason.0 },
            ExecutionMetadata::Complete => Self::Complete {},
        }
    }
}

/// The name of one comparison outcome.
const fn comparison_name(value: Comparison) -> &'static str {
    match value {
        Comparison::Exact => constants::EXACT,
        Comparison::Different => constants::DIFFERENT,
        Comparison::Incomparable => constants::INCOMPARABLE,
    }
}

/// Both comparison dimensions as the comparison evidence spells them.
#[derive(Serialize)]
struct DimensionsSchema {
    diagnostics: &'static str,
    execution_metadata: &'static str,
}
impl From<ReportComparison> for DimensionsSchema {
    fn from(comparison: ReportComparison) -> Self {
        Self {
            diagnostics: comparison_name(comparison.diagnostics),
            execution_metadata: comparison_name(comparison.execution_metadata),
        }
    }
}

/// The comparison evidence line: the declared contract, both dimensions and both producers' capabilities.
#[derive(Serialize)]
struct ComparisonSchema<'a> {
    schema: &'static str,
    contract: &'static str,
    comparison: DimensionsSchema,
    reference_execution_metadata: ExecutionMetadataSchema<'a>,
    candidate_execution_metadata: ExecutionMetadataSchema<'a>,
}

/// The comparison evidence line as one JSON text.
pub(super) fn comparison(
    reference: &RunReport,
    candidate: &RunReport,
    comparison: ReportComparison,
    contract: ComparisonContract,
) -> Result<String, Error> {
    serde_json::to_string(&ComparisonSchema {
        schema: constants::BENCHMARK_COMPARISON_V1,
        contract: match contract {
            ComparisonContract::Diagnostics => constants::DIAGNOSTICS,
            ComparisonContract::CompleteReport => constants::COMPLETE_REPORT,
        },
        comparison: DimensionsSchema::from(comparison),
        reference_execution_metadata: ExecutionMetadataSchema::from(&reference.execution_metadata),
        candidate_execution_metadata: ExecutionMetadataSchema::from(&candidate.execution_metadata),
    })
    .map_err(|error| Error::Json(Box::new(error)))
}
