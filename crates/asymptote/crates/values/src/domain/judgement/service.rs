//! The `judgement` service: what a run's violations mean against the accepted records, with no IO and no ports.

use std::collections::BTreeMap;

use super::types::{
    Content, Evidence, Inventory, Judgement, Position, Relocated, Resolution, Resolved, Run, Short,
    Sightings, Slot, Stale, StaleReason, Status, Verdict,
};
use crate::domain::operations::types::{Count, Record};
use crate::domain::perf::service::holding;
use crate::domain::perf::types::{Holding, PerfOutcome};
use crate::domain::violations::constants::WHOLE_FILE;
use crate::domain::violations::service::key;
use crate::domain::violations::types::{Key, Keyed, Violation};

/// Compares the violations `runs` reported against the `accepted` records.
///
/// A violation with no key is new: failing towards reporting is the safe direction for a gate. A record's count is
/// summed over every row of its key first, so one row cannot consume every sighting and leave the rest resolved; a
/// key seen more often than accepted is new by the surplus, which closes the fix-one-add-one hole. Then a record its
/// own path reports fewer times than it accepts takes, up to what it is short, the unaccepted sightings of the same
/// content at another path: code that moved, as a file renamed or code carried to another file. A whole-file
/// violation has no content but its file, so it moves only with a rename git records.
#[must_use]
pub fn compare(violations: &[Violation], accepted: &[Record], runs: &[Run]) -> Judgement {
    let mut outcome = Judgement::default();
    let mut observed: BTreeMap<Key, Vec<&Violation>> = BTreeMap::new();
    for violation in violations {
        match key(violation) {
            Keyed::Key(key) => observed.entry(key).or_default().push(violation),
            Keyed::Unkeyed => outcome.new.push(violation.clone()),
        }
    }
    // Each key's allowance, summed over its records, and the first record that accepts it.
    let mut allowance: BTreeMap<&Key, (u32, &Record)> = BTreeMap::new();
    for record in accepted {
        allowance.entry(&record.key).or_insert((0_u32, record)).0 += record.count.0;
    }
    // First at the record's own path: the allowance a key's sightings use, the sightings beyond it, and the records
    // reported fewer times than they accept.
    let mut pending: Vec<Sightings<'_>> = Vec::new();
    let mut short: Vec<Short<'_>> = Vec::new();
    for (key, (allowed, record)) in allowance {
        let seen = observed.remove(key).unwrap_or_default();
        let count = u32::try_from(seen.len()).unwrap_or(u32::MAX);
        let covered = seen
            .len()
            .min(usize::try_from(allowed).unwrap_or(usize::MAX));
        let mut sightings = seen.into_iter();
        outcome
            .accepted
            .extend(sightings.by_ref().take(covered).cloned());
        if count >= allowed {
            pending.push(Sightings {
                key,
                violations: sightings.rev().map(Some).collect(),
            });
            continue;
        }
        short.push(Short {
            record,
            accepted: Count(allowed),
            seen: Count(count),
        });
    }
    let unaccepted: Vec<(Key, Vec<&Violation>)> = observed.into_iter().collect();
    pending.extend(unaccepted.iter().map(|(key, seen)| Sightings {
        key,
        violations: seen.iter().map(|violation| Some(*violation)).collect(),
    }));
    claim_moved(
        &mut pending,
        &mut short,
        &mut outcome.accepted,
        &mut outcome.relocated,
    );
    outcome.new.extend(
        pending
            .into_iter()
            .flat_map(|sightings| sightings.violations.into_iter().flatten())
            .cloned(),
    );
    for Short {
        record,
        accepted: ceiling,
        seen,
    } in short
    {
        if seen >= ceiling {
            continue;
        }
        let (allowed, sighted) = (ceiling.0, seen.0);
        let resolved = |resolution| Resolved {
            record: record.clone(),
            count: Count(allowed - sighted),
            remaining: Count(sighted),
            resolution,
        };
        match evidence(record, runs) {
            Evidence::Evaluated => outcome.resolved.push(resolved(Resolution::NotReported)),
            Evidence::Retired => outcome.resolved.push(resolved(Resolution::RuleRetired)),
            Evidence::Missing(reason) => outcome.stale.push(Stale {
                record: record.clone(),
                reason,
            }),
        }
    }
    outcome.new.sort();
    outcome.accepted.sort();
    outcome
}

/// A record short at its own path takes, up to what it is short, the unaccepted sightings of the same content at
/// another path: code that moved, `relocated` under the key it is reported at now. A whole-file violation has no
/// content but its file, so it moves only with a rename.
fn claim_moved(
    pending: &mut [Sightings<'_>],
    short: &mut [Short<'_>],
    accepted: &mut Vec<Violation>,
    relocated: &mut Vec<Relocated>,
) {
    let mut elsewhere: BTreeMap<Content<'_>, Vec<Slot>> = BTreeMap::new();
    for (group, sightings) in pending.iter().enumerate() {
        if sightings.key.fingerprint != WHOLE_FILE {
            elsewhere.entry(content(sightings.key)).or_default().extend(
                (0..sightings.violations.len()).map(|index| Slot {
                    group: Position(group),
                    index: Position(index),
                }),
            );
        }
    }
    for item in short.iter_mut() {
        if item.record.key.fingerprint == WHOLE_FILE {
            continue;
        }
        if let Some(candidates) = elsewhere.get_mut(&content(&item.record.key)) {
            while item.seen < item.accepted
                && let Some(Slot { group, index }) = candidates.pop()
            {
                if let Some(sightings) = pending.get_mut(group.0)
                    && let Some(moved) =
                        sightings.violations.get_mut(index.0).and_then(Option::take)
                {
                    accepted.push(moved.clone());
                    let to = sightings.key;
                    match relocated.iter_mut().find(|relocated| {
                        relocated.record.id == item.record.id && &relocated.to == to
                    }) {
                        Some(relocated) => relocated.count.0 += 1,
                        None => relocated.push(Relocated {
                            record: item.record.clone(),
                            to: to.clone(),
                            count: Count(1),
                        }),
                    }
                }
                item.seen.0 += 1;
            }
        }
    }
}

/// What identifies a violation wherever it is: its key without its path.
fn content(key: &Key) -> Content<'_> {
    Content {
        tool: &key.tool,
        rule: &key.rule,
        level: key.level,
        standing: key.standing,
        fingerprint: key.fingerprint,
    }
}

/// What `runs` show of the rule `record` accepts a violation of. A completed run of the tool evaluated the rule unless
/// it declared its rules and left this one out. Retirement needs every run to declare its rules without it, since a
/// failed run or one that reported the rule anyway could each have evaluated it. A tool that did not run, or failed,
/// shows nothing: its records stay unjudged.
#[must_use]
pub(crate) fn evidence(record: &Record, runs: &[Run]) -> Evidence {
    let mut any = false;
    let mut completed = false;
    let mut retire = true;
    for run in runs.iter().filter(|run| run.tool == record.key.tool) {
        any = true;
        if run.status == Status::Completed {
            completed = true;
            let evaluated = match &run.inventory {
                Inventory::Declared(rules) => rules.contains(&record.key.rule),
                // A tool that ran to completion evaluated everything it was asked: a rule it does not list is no
                // proof it did not run, and a record it no longer reports is resolved.
                Inventory::Undeclared => true,
            };
            if evaluated {
                return Evidence::Evaluated;
            }
        }
        retire &= run.status == Status::Completed
            && matches!(run.inventory, Inventory::Declared(_))
            && !run.reported.contains(&record.key.rule);
    }
    if !any {
        Evidence::Missing(StaleReason::ToolNotRun)
    } else if !completed {
        Evidence::Missing(StaleReason::ToolFailed)
    } else if retire {
        Evidence::Retired
    } else {
        Evidence::Missing(StaleReason::RuleNotEvaluated)
    }
}

/// The record a partly resolved record is lowered to: its count the violations that remain; none when none does.
#[must_use]
pub fn lowered(resolved: &Resolved) -> Option<Record> {
    (resolved.remaining.0 > 0).then(|| Record {
        count: resolved.remaining,
        ..resolved.record.clone()
    })
}

/// What a run comes to: a failed tool outranks anything found, since "could not determine" is never "fine".
#[must_use]
pub fn verdict(runs: &[Run], outcome: &Judgement) -> Verdict {
    let failed = runs.iter().any(|run| run.status != Status::Completed);
    let unrun = outcome
        .stale
        .iter()
        .any(|stale| stale.reason == StaleReason::ToolNotRun);
    if failed || unrun {
        Verdict::Undetermined
    } else if outcome.new.is_empty() && outcome.resolved.is_empty() {
        Verdict::Pass
    } else {
        Verdict::Changed
    }
}

/// The last lines of what a tool wrote to stderr: enough to say why it failed, never the whole of a long log.
#[must_use]
pub fn tail(stderr: &super::types::Tail) -> super::types::Tail {
    let lines: Vec<&str> = stderr.0.trim_end().lines().collect();
    let start = lines.len().saturating_sub(super::constants::TAIL_LINES);
    let last = lines.get(start..).unwrap_or_default();
    super::types::Tail(last.join(super::constants::NEWLINE).into())
}

/// The verdict of a check once its perf outcome is counted: a pass is changed by a perf outcome that breaks it, and
/// anything else stands.
#[must_use]
pub fn with_perf(judged: Verdict, outcome: &PerfOutcome) -> Verdict {
    if judged == Verdict::Pass && holding(outcome) == Holding::Breaks {
        Verdict::Changed
    } else {
        judged
    }
}
