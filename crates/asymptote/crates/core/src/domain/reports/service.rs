//! The `reports` service: whether a kept report or run still holds, with no IO and no ports.

use super::constants::{
    ABSENT, BMF, EXIT_STATUS, HEAD, OPTION_VALUE, RACY, RANGE_DOTS, SARIF, SHELL_PUNCTUATION,
    SYMMETRIC_DOT,
};
use super::types::{
    Digest, FileDigest, FileRecord, FileState, HashParts, Keeping, Listed, NamedInput, Observed,
    Recorded, Report, Scanned, Stamped, Stats,
};
use crate::domain::manifest::types::{Output, Pathspec, Tool};
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::judgement::types::{Failure, Inventory, Run, Status};
use crate::domain::operations::types::{PerfKey, Record, Revision};
use crate::domain::perf::types::{Measured, Measurement};
use crate::domain::violations::types::{RepoPath, ToolId, Violation};

/// The files whose digest now differs from the one `kept` recorded.
#[must_use]
pub fn moved(kept: &Recorded, now: &[FileDigest]) -> Vec<RepoPath> {
    now.iter()
        .filter(|now| {
            kept.files
                .get(&now.path)
                .is_none_or(|record| record.digest.0 != now.digest.0)
        })
        .map(|now| now.path.clone())
        .collect()
}

/// The files of `stats` whose digest `known` cannot vouch for, which must be read: a present file with no record, one
/// of another size or modification time, or one recorded so soon after its modification that a write in the same tick
/// could hide.
#[must_use]
pub fn unread(known: &Recorded, stats: &Stats) -> Vec<RepoPath> {
    stats
        .files
        .iter()
        .filter(|stat| match stat.state {
            FileState::Absent => false,
            FileState::Present { size, modified } => {
                known.files.get(&stat.path).is_none_or(|record| {
                    record.size != size
                        || record.modified != modified
                        || modified.0.saturating_add(RACY.0) >= known.taken.0
                })
            }
        })
        .map(|stat| stat.path.clone())
        .collect()
}

/// The record of every file of `stats`, taken when they were looked at: an absent file's is the absent digest, a file
/// just read takes its digest from `read`, and any other keeps the digest `known` recorded for it.
#[must_use]
pub fn recorded(known: &Recorded, stats: &Stats, read: &[FileDigest]) -> Recorded {
    let read: BTreeMap<&RepoPath, &Digest> =
        read.iter().map(|file| (&file.path, &file.digest)).collect();
    let files = stats
        .files
        .iter()
        .map(|stat| {
            let record = match stat.state {
                FileState::Absent => FileRecord {
                    digest: Digest(ABSENT.into()),
                    ..FileRecord::default()
                },
                FileState::Present { size, modified } => FileRecord {
                    size,
                    modified,
                    digest: read
                        .get(&stat.path)
                        .map(|digest| (*digest).clone())
                        .or_else(|| {
                            known
                                .files
                                .get(&stat.path)
                                .map(|record| record.digest.clone())
                        })
                        .unwrap_or_else(|| Digest(ABSENT.into())),
                },
            };
            (stat.path.clone(), record)
        })
        .collect();
    Recorded {
        files,
        taken: stats.taken,
    }
}

/// The run of `tool` a report kept under `stamp`, when it completed: inputs that did not move give the same answer.
#[must_use]
pub fn reusable<'a>(report: &'a Report, tool: &ToolId, stamp: &Digest) -> Option<&'a Scanned> {
    report.scanned.iter().find(|scanned| {
        &scanned.run.tool == tool
            && scanned.run.status == Status::Completed
            && scanned.stamp == Stamped::Digested(stamp.clone())
    })
}

/// The files a judgement rests on beyond the tools' listed inputs: each a violation is in, and each a record judged
/// is of, sorted.
#[must_use]
pub fn unlisted(listed: &Listed, violations: &[Violation], accepted: &[Record]) -> Vec<RepoPath> {
    let touched: BTreeSet<&RepoPath> = violations
        .iter()
        .map(|found| &found.path)
        .chain(accepted.iter().map(|record| &record.key.path))
        .filter(|path| !listed.stamps.contains_key(*path))
        .collect();
    touched.into_iter().cloned().collect()
}

/// The runs of a report that failed.
#[must_use]
pub fn failures(report: &Report) -> Vec<&Run> {
    report
        .scanned
        .iter()
        .filter(|scanned| scanned.keeping == Keeping::Current)
        .map(|scanned| &scanned.run)
        .filter(|run| run.status != Status::Completed)
        .collect()
}

/// Every violation a report's runs reported, and every run.
#[must_use]
pub fn observed(report: &Report) -> Observed {
    Observed {
        violations: report
            .scanned
            .iter()
            .filter(|scanned| scanned.keeping == Keeping::Current)
            .flat_map(|scanned| scanned.violations.iter().cloned())
            .collect(),
        runs: report
            .scanned
            .iter()
            .filter(|scanned| scanned.keeping == Keeping::Current)
            .map(|scanned| scanned.run.clone())
            .collect(),
    }
}

/// The run of `tool` that failed: what it would have reported is unknown, and it is never reused.
#[must_use]
pub fn failed(tool: &ToolId, failure: Failure) -> Scanned {
    Scanned {
        run: Run {
            tool: tool.clone(),
            status: Status::Failed(failure),
            inventory: Inventory::Undeclared,
            reported: BTreeSet::new(),
        },
        violations: Vec::new(),
        stamp: Stamped::Unstamped,
        inputs: Vec::new(),
        measurements: Vec::new(),
        keeping: Keeping::Current,
    }
}

/// The completed run of `tool`, under `stamp`: the rules it reported are those of its violations.
#[must_use]
pub fn completed(
    tool: &ToolId,
    violations: Vec<Violation>,
    inventory: Inventory,
    measurements: Vec<Measurement>,
    stamp: Stamped,
    inputs: Vec<NamedInput>,
) -> Scanned {
    Scanned {
        run: Run {
            tool: tool.clone(),
            status: Status::Completed,
            inventory,
            reported: violations
                .iter()
                .map(|violation| violation.rule.clone())
                .collect(),
        },
        violations,
        stamp,
        inputs,
        measurements,
        keeping: Keeping::Current,
    }
}

/// Every measurement `scanned` runs made, with the tool that made it.
#[must_use]
pub fn measured(scanned: &[Scanned]) -> Vec<Measured> {
    scanned
        .iter()
        .filter(|scanned| scanned.keeping == Keeping::Current)
        .flat_map(|scanned| {
            scanned.measurements.iter().map(|measurement| Measured {
                key: PerfKey {
                    tool: scanned.run.tool.clone(),
                    benchmark: measurement.benchmark.clone(),
                    measure: measurement.measure.clone(),
                },
                value: measurement.value,
                lower: measurement.lower,
                upper: measurement.upper,
            })
        })
        .collect()
}

/// The two ends of the git range `word` writes (`a..b`, `a...b`), when both are written as revisions and not as paths that
/// climb (`../x`, `a/../b`).
fn range_of(word: &Revision) -> Option<[Revision; 2]> {
    let (left, right) = word.0.split_once(RANGE_DOTS)?;
    let right = right.strip_prefix(SYMMETRIC_DOT).unwrap_or(right);
    // Git reads an end left out as `HEAD`: `main..` is `main..HEAD`.
    let ends = [left, right].map(|end| if end.is_empty() { HEAD } else { end });
    (left.len() + right.len() > 0
        && ends.iter().all(|end| {
            !end.contains(RANGE_DOTS) && !end.starts_with(['/', '.']) && !end.ends_with(['/', '.'])
        }))
    .then(|| ends.map(|end| Revision(end.into())))
}

/// The revisions `tool`'s command line names as the ends of a git range, each once, in order: a tool that reads a range
/// reads git's history, which is in none of its inputs, so the commits these name are part of its stamp.
#[must_use]
pub fn range_ends(tool: &Tool) -> Vec<Revision> {
    let mut ends: Vec<Revision> = Vec::new();
    for argument in &tool.run.0 {
        let words = argument
            .0
            .split_whitespace()
            .flat_map(|word| word.split(OPTION_VALUE))
            .map(|word| Revision(word.trim_matches(SHELL_PUNCTUATION).into()));
        for end in words.filter_map(|word| range_of(&word)).flatten() {
            if !ends.contains(&end) {
                ends.push(end);
            }
        }
    }
    ends
}

/// What `tool`'s stamp is made of: its command line, how its run is read, the commits of the git range it reads
/// (`commits`, resolved by the caller), and each of its pathspecs with the digest of what it names in the input tree
/// (`named`); none when one of its pathspecs could not be listed, so the run is never reused.
#[must_use]
pub fn stamp_parts<'a>(
    tool: &'a Tool,
    named: &'a BTreeMap<Pathspec, Option<Digest>>,
    commits: &'a [Revision],
) -> Option<HashParts<'a>> {
    let mut inputs = Vec::with_capacity(tool.inputs.len());
    for spec in &tool.inputs {
        inputs.push((spec, named.get(spec)?.as_ref()?));
    }
    let mut parts: Vec<&[u8]> = tool
        .run
        .0
        .iter()
        .map(|argument| argument.0.as_bytes())
        .collect();
    parts.push(match tool.output {
        Output::Sarif => SARIF,
        Output::ExitStatus => EXIT_STATUS,
        Output::Bmf => BMF,
    });
    parts.extend(commits.iter().map(|commit| commit.0.as_bytes()));
    for (spec, digest) in inputs {
        parts.push(spec.0.as_bytes());
        parts.push(digest.0.as_bytes());
    }
    Some(HashParts(parts))
}

/// The runs of the check that kept `previous` that the next one keeps beside its own: a completed, stamped run of a
/// tool in `configured` that is not in `selected`, so a check of some groups forgets nothing of the others.
#[must_use]
pub fn retained(previous: &[Scanned], configured: &[ToolId], selected: &[ToolId]) -> Vec<Scanned> {
    previous
        .iter()
        .filter(|scanned| {
            let tool = &scanned.run.tool;
            configured.contains(tool)
                && !selected.contains(tool)
                && scanned.run.status == Status::Completed
                && matches!(scanned.stamp, Stamped::Digested(_))
        })
        .map(|scanned| Scanned {
            keeping: Keeping::Retained,
            ..scanned.clone()
        })
        .collect()
}

/// The completed, stamped runs of `previous` of the tools in `failed`, each kept only for a later check to reuse: a tool
/// whose run failed this time keeps the answer of the last run that completed.
#[must_use]
pub fn replaced(previous: &[Scanned], failed: &[ToolId]) -> Vec<Scanned> {
    previous
        .iter()
        .filter(|scanned| {
            failed.contains(&scanned.run.tool)
                && scanned.run.status == Status::Completed
                && matches!(scanned.stamp, Stamped::Digested(_))
        })
        .map(|scanned| Scanned {
            keeping: Keeping::Retained,
            ..scanned.clone()
        })
        .collect()
}
