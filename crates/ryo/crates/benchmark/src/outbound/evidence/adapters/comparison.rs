//! Comparison evidence accompanies timing output without becoming a fabricated BMF metric.
use crate::domain::diagnostics::types::{ComparisonContract, ReportComparison, RunReport};

/// Retain the declared contract, both dimensions and actual producer capabilities on the benchmark evidence stream.
pub(crate) fn write(
    reference: &RunReport,
    candidate: &RunReport,
    comparison: ReportComparison,
    contract: ComparisonContract,
) {
    eprintln!(
        "{}",
        serde_json::json!({
            "schema": "benchmark-comparison/v1",
            "contract": contract,
            "comparison": comparison,
            "reference_execution_metadata": &reference.execution_metadata,
            "candidate_execution_metadata": &candidate.execution_metadata,
        })
    );
}
