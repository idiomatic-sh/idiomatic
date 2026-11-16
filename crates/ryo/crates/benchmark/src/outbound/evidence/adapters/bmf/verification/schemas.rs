//! Only this boundary knows the BMF names of the verification times.
use super::super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Serialize;
use std::collections::BTreeMap;

/// One verification time as a BMF metric: a single observation.
#[derive(Serialize)]
struct MeasureSchema {
    /// The time in milliseconds.
    value: f64,
    /// The smallest observation, the same one.
    lower_value: f64,
    /// The largest observation, the same one.
    upper_value: f64,
}
impl From<std::time::Duration> for MeasureSchema {
    fn from(elapsed: std::time::Duration) -> Self {
        let milliseconds = elapsed.as_secs_f64() * constants::MILLISECONDS_PER_SECOND;
        Self {
            value: milliseconds,
            lower_value: milliseconds,
            upper_value: milliseconds,
        }
    }
}

/// The verification times of every rule as one BMF document: one benchmark for each rule, `verify/<linter>/<rule>`, with the
/// measures `total-ms`, `max-case-ms` and `compile-ms`, so a tool states one tolerance for each measure across its rules.
pub(in super::super) fn encode(
    linter: &ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName,
    timings: &BTreeMap<ryo::linter::RuleId, crate::outbound::evidence::types::RuleTiming>,
) -> Result<Vec<u8>, Error> {
    let mut document = BTreeMap::new();
    for (rule, timing) in timings {
        let _previous = document.insert(
            format!("{}/{}/{rule}", constants::VERIFY, linter.0),
            BTreeMap::from([
                (constants::TOTAL_MS, MeasureSchema::from(timing.total)),
                (constants::MAX_CASE_MS, MeasureSchema::from(timing.slowest)),
                (constants::COMPILE_MS, MeasureSchema::from(timing.compile)),
            ]),
        );
    }
    serde_json::to_vec(&document).map_err(|error| Error::Json(Box::new(error)))
}
