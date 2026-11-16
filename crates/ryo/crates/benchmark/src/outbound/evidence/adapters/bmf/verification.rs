//! The verification times as one BMF document.
use super::constants;
use super::schemas::MetricSchema;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;

/// The verification times of every rule as one BMF document: one benchmark for each rule, `verify/<linter>/<rule>`, with the
/// measures `total-ms`, `max-case-ms` and `compile-ms`, so a tool states one tolerance for each measure across its rules.
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
    let mut document = BTreeMap::new();
    for (rule, timing) in timings {
        let _previous = document.insert(
            format!("{}/{}/{rule}", constants::VERIFY, linter.0),
            BTreeMap::from([
                (constants::TOTAL_MS, measure(timing.total)),
                (constants::MAX_CASE_MS, measure(timing.slowest)),
                (constants::COMPILE_MS, measure(timing.compile)),
            ]),
        );
    }
    serde_json::to_vec(&document).map_err(|error| Error::Json(Box::new(error)))
}
