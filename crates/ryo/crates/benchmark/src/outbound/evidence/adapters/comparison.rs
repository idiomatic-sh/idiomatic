//! Comparison evidence accompanies timing output without becoming a fabricated BMF metric.
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::ReportComparison;
use ryo_benchmark_contracts::domain::reports::types::RunReport;

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
            "contract": contract_name(contract),
            "comparison": comparison_value(comparison),
            "reference_execution_metadata": crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&reference.execution_metadata),
            "candidate_execution_metadata": crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&candidate.execution_metadata),
        })
    );
}

fn contract_name(contract: ComparisonContract) -> &'static str {
    match contract {
        ComparisonContract::Diagnostics => "diagnostics",
        ComparisonContract::CompleteReport => "complete-report",
    }
}
fn comparison_value(comparison: ReportComparison) -> serde_json::Value {
    use ryo_benchmark_contracts::domain::reports::types::Comparison;
    let name = |value| match value {
        Comparison::Exact => "exact",
        Comparison::Different => "different",
        Comparison::Incomparable => "incomparable",
    };
    serde_json::json!({"diagnostics":name(comparison.diagnostics),"execution_metadata":name(comparison.execution_metadata)})
}
