//! Comparison evidence accompanies timing output without becoming a fabricated BMF metric.
use crate::outbound::evidence::adapters::comparison::constants;
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
            constants::SCHEMA: constants::BENCHMARK_COMPARISON_V1,
            constants::CONTRACT: contract_name(contract),
            constants::COMPARISON: comparison_value(comparison),
            constants::REFERENCE_EXECUTION_METADATA: crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&reference.execution_metadata),
            constants::CANDIDATE_EXECUTION_METADATA: crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&candidate.execution_metadata),
        })
    );
}

fn contract_name(contract: ComparisonContract) -> &'static str {
    match contract {
        ComparisonContract::Diagnostics => constants::DIAGNOSTICS,
        ComparisonContract::CompleteReport => constants::COMPLETE_REPORT,
    }
}
fn comparison_value(comparison: ReportComparison) -> serde_json::Value {
    use ryo_benchmark_contracts::domain::reports::types::Comparison;
    let name = |value| match value {
        Comparison::Exact => constants::EXACT,
        Comparison::Different => constants::DIFFERENT,
        Comparison::Incomparable => constants::INCOMPARABLE,
    };
    serde_json::json!({constants::DIAGNOSTICS:name(comparison.diagnostics),constants::EXECUTION_METADATA:name(comparison.execution_metadata)})
}
