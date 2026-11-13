//! What a measurement is admitted against.
use crate::domain::suite::types::audit::value::Audit;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use ryo_benchmark_contracts::domain::workloads::types::Tool;

/// The audit, the tool it audited and the inventory the tool declared before the audit narrowed it.
#[derive(Debug)]
pub struct MeasurementAdmission<'a> {
    /// Whether the run is held to the accounted inventory or only to the rules it selects.
    pub scope: BenchmarkCoverage,
    /// The audit of the tool's authored rules.
    pub audit: &'a Audit,
    /// The tool whose defaults, deferrals and pending rules the audit accounts for.
    pub tool: &'a Tool,
    /// The complete default inventory the tool declared.
    pub declared_inventory: Vec<ryo::linter::RuleId>,
}
