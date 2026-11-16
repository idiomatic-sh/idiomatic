//! The verification times as one BMF document.
use super::constants;
use super::schemas::MetricSchema;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;

/// The verification times of every rule as one BMF document: benchmark `verify`, and for each rule the measures
/// `verify/<linter>/<rule>-total-ms`, `-max-case-ms` and `-compile-ms`.
pub(super) fn encode(
    linter: &ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName,
    timings: &BTreeMap<ryo::linter::RuleId, crate::outbound::evidence::types::RuleTiming>,
) -> Result<Vec<u8>, Error> {
    let measure = |elapsed: std::time::Duration| {
        let milliseconds = elapsed.as_secs_f64() * 1000.0;
        MetricSchema {
            value: milliseconds,
            lower_value: milliseconds,
            upper_value: milliseconds,
        }
    };
    let mut measures = BTreeMap::new();
    for (rule, timing) in timings {
        let prefix = format!("{}/{}/{rule}", constants::VERIFY, linter.0);
        let _total = measures.insert(format!("{prefix}-total-ms"), measure(timing.total));
        let _slowest = measures.insert(format!("{prefix}-max-case-ms"), measure(timing.slowest));
        let _compile = measures.insert(format!("{prefix}-compile-ms"), measure(timing.compile));
    }
    serde_json::to_vec(&BTreeMap::from([(constants::VERIFY, measures)]))
        .map_err(|error| Error::Json(Box::new(error)))
}
