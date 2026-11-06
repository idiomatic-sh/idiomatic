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
            crate::outbound::evidence::adapters::comparison::constants::SCHEMA: crate::outbound::evidence::adapters::comparison::constants::BENCHMARK_COMPARISON_V1,
            crate::outbound::evidence::adapters::comparison::constants::CONTRACT: contract_name(contract),
            crate::outbound::evidence::adapters::comparison::constants::COMPARISON: comparison_value(comparison),
            crate::outbound::evidence::adapters::comparison::constants::REFERENCE_EXECUTION_METADATA: crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&reference.execution_metadata),
            crate::outbound::evidence::adapters::comparison::constants::CANDIDATE_EXECUTION_METADATA: crate::outbound::worker::schemas::ExecutionMetadataSchema::from(&candidate.execution_metadata),
        })
    );
}

fn contract_name(contract: ComparisonContract) -> &'static str {
    match contract {
        ComparisonContract::Diagnostics => {
            crate::outbound::evidence::adapters::comparison::constants::DIAGNOSTICS
        }
        ComparisonContract::CompleteReport => {
            crate::outbound::evidence::adapters::comparison::constants::COMPLETE_REPORT
        }
    }
}
fn comparison_value(comparison: ReportComparison) -> serde_json::Value {
    use ryo_benchmark_contracts::domain::reports::types::Comparison;
    let name = |value| match value {
        Comparison::Exact => crate::outbound::evidence::adapters::comparison::constants::EXACT,
        Comparison::Different => {
            crate::outbound::evidence::adapters::comparison::constants::DIFFERENT
        }
        Comparison::Incomparable => {
            crate::outbound::evidence::adapters::comparison::constants::INCOMPARABLE
        }
    };
    serde_json::json!({crate::outbound::evidence::adapters::comparison::constants::DIAGNOSTICS:name(comparison.diagnostics),crate::outbound::evidence::adapters::comparison::constants::EXECUTION_METADATA:name(comparison.execution_metadata)})
}
