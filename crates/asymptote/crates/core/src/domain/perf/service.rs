//! The `perf` service: what a run's measurements mean against the lock's perf records, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    Holding, Improvement, Measured, PerfOutcome, Regression, Spread, ToleranceKey, Unseeded,
};
use crate::domain::manifest::types::{Percent, Tool};
use crate::domain::operations::types::{Count, Number, PerfKey, PerfRecord};
use crate::domain::violations::types::ToolId;

/// How the run stands against `best`: `Greater` when even the low end of its spread is above the best raised by the
/// `tolerance` (the best itself when it is none), `Less` when even the high end is below the best, `Equal` otherwise; a bound
/// the tool did not report is the median.
pub(crate) fn standing_of(
    measurement: &Measured,
    best: Number,
    tolerance: Percent,
) -> std::cmp::Ordering {
    let ceiling = best.0 * (1.0_f64 + f64::from(tolerance.0) / 100.0_f64);
    let bound = |spread: Spread| match spread {
        Spread::Given(number) => number.0,
        Spread::Absent => measurement.value.0,
    };
    let (low, high) = (bound(measurement.lower), bound(measurement.upper));
    if low > ceiling {
        std::cmp::Ordering::Greater
    } else if high < best.0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Equal
    }
}

/// The tolerance each tool's manifest states for each of its measures: what `check` judges by and `up` raises by.
#[must_use]
pub fn allowances(tools: &[Tool]) -> BTreeMap<ToleranceKey, Percent> {
    tools
        .iter()
        .flat_map(|tool| {
            tool.tolerance.iter().map(|(measure, percent)| {
                (
                    ToleranceKey {
                        tool: tool.id.clone(),
                        measure: measure.clone(),
                    },
                    *percent,
                )
            })
        })
        .collect()
}

/// The records whose tool completed (`ran`) and no longer reports them.
#[must_use]
pub fn resolved(measured: &[Measured], records: &[PerfRecord], ran: &[ToolId]) -> Vec<PerfRecord> {
    let seen: BTreeSet<&PerfKey> = measured.iter().map(|each| &each.key).collect();
    records
        .iter()
        .filter(|record| ran.contains(&record.key.tool) && !seen.contains(&record.key))
        .cloned()
        .collect()
}

/// Judges `measured` against `records`: a run whose low end is above the best raised by its measure's tolerance in `allowed`
/// (none, for a measure it does not name) is regressed, one whose high end is below the best improved, one with no record
/// unseeded; a record of a tool in `ran` that reports it no longer is resolved.
#[must_use]
pub fn judge(
    measured: &[Measured],
    records: &[PerfRecord],
    ran: &[ToolId],
    allowed: &BTreeMap<ToleranceKey, Percent>,
) -> PerfOutcome {
    let recorded: BTreeMap<&PerfKey, &PerfRecord> =
        records.iter().map(|record| (&record.key, record)).collect();
    let mut outcome = PerfOutcome::default();
    for each in measured {
        let value = each.value;
        let Some(record) = recorded.get(&each.key).copied() else {
            outcome.unseeded.push(Unseeded {
                key: each.key.clone(),
                value,
            });
            continue;
        };
        // Only `up` raises a best: whatever its spread says, a check rewrites a best only to a lower value.
        let tolerance = allowed
            .get(&ToleranceKey {
                tool: each.key.tool.clone(),
                measure: each.key.measure.clone(),
            })
            .copied()
            .unwrap_or_default();
        let spread = standing_of(each, record.best, tolerance);
        let standing = if value.0 < record.best.0 {
            spread
        } else {
            spread.max(std::cmp::Ordering::Equal)
        };
        match standing {
            std::cmp::Ordering::Greater => outcome.regressed.push(Regression {
                record: record.clone(),
                value,
            }),
            std::cmp::Ordering::Less => outcome.improved.push(Improvement {
                record: record.clone(),
                value,
            }),
            std::cmp::Ordering::Equal => outcome.held = Count(outcome.held.0 + 1),
        }
    }
    outcome.resolved = resolved(measured, records, ran);
    outcome
}

/// What the outcome means for the run: it breaks it on a regression, a median nothing seeds, or a record to drop.
#[must_use]
pub fn holding(outcome: &PerfOutcome) -> Holding {
    if outcome.regressed.is_empty() && outcome.unseeded.is_empty() && outcome.resolved.is_empty() {
        Holding::Holds
    } else {
        Holding::Breaks
    }
}

/// The records an improvement rewrites: each at its new best, keeping its reasons.
#[must_use]
pub fn rewritten(outcome: &PerfOutcome) -> Vec<PerfRecord> {
    outcome
        .improved
        .iter()
        .map(|improved| PerfRecord {
            best: improved.value,
            ..improved.record.clone()
        })
        .collect()
}
