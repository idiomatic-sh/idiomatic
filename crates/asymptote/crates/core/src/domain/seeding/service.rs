//! The `seeding` service: the perf records `up` writes, with no IO and no ports.

use std::collections::BTreeMap;

use asymptote_values::domain::manifest::types::Percent;
use asymptote_values::domain::operations::types::Reason;
use asymptote_values::domain::operations::types::{PerfKey, PerfRecord};
use asymptote_values::domain::perf::service::standing_of;
use asymptote_values::domain::perf::types::{Measured, ToleranceKey};
use asymptote_values::domain::violations::types::ToolId;

/// The records `up` writes for a run of `measured`: a median with no record seeds one, and a run over its record's best
/// raised by its tolerance in `allowed` (the one `check` judges by) raises the best to its median and appends the reason, each of
/// a tool in `tools` (any, when none is named): a run `check` holds is never raised.
#[must_use]
pub fn seeding(
    measured: &[Measured],
    records: &[PerfRecord],
    tools: &[ToolId],
    allowed: &BTreeMap<ToleranceKey, Percent>,
    reason: &Reason,
) -> Vec<PerfRecord> {
    let recorded: BTreeMap<&PerfKey, &PerfRecord> =
        records.iter().map(|record| (&record.key, record)).collect();
    let mut written = Vec::new();
    for each in measured
        .iter()
        .filter(|each| tools.is_empty() || tools.contains(&each.key.tool))
    {
        let key = each.key.clone();
        let mut reasons = match recorded.get(&key).copied() {
            None => Vec::new(),
            Some(record)
                if standing_of(
                    each,
                    record.best,
                    allowed
                        .get(&ToleranceKey {
                            tool: each.key.tool.clone(),
                            measure: each.key.measure.clone(),
                        })
                        .copied()
                        .unwrap_or_default(),
                )
                .is_gt() =>
            {
                record.reasons.clone()
            }
            Some(_) => continue,
        };
        reasons.push(reason.clone());
        written.push(PerfRecord {
            key,
            best: each.value,
            reasons,
        });
    }
    written
}
