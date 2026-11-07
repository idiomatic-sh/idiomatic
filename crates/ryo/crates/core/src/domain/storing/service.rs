//! The `storing` service: free functions named by verb over what a run keeps in its store and what the next uses again, with no IO
//! and no ports.

use super::types::{Decision, ToKeep, ToStore};
use crate::domain::handing::service::{hand_into, handed_in, rows_of_file};
use crate::domain::keeping::service::{findings_of, rows_from};
use crate::domain::workers::types::FileOrder;
use ryo_execution::inbound::use_cases::metadata::interface::{Message, Unknown};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, FileNumber, Nanoseconds};
use ryo_values::domain::records::types::{
    FileOutcome, Freshness, Handing, Ran, Record, RuleFrontier,
};
use ryo_values::domain::row_texts::types::{Cell, FindingRows, PlacedRule};
use ryo_values::domain::runs::types::{RuleId, Skipped};
use ryo_values::domain::stored_facts::types::{Bucket, Digest};
use ryo_values::domain::stored_findings::types::{
    FindingMade, StoredCell, StoredFileNumber, StoredFinding, StoredUnknown, StoredUnread,
    UnknownText,
};
use ryo_values::domain::stores::types::MadeAgain;
use ryo_values::domain::stores::types::{
    BucketsWanted, Crossing, FileAgain, KeptRows, Stored, StoredFile, StoredFrontier, StoredRule,
};

/// The stored file at `path`: the files are kept in the order of their paths.
fn stored_at<'a>(stored: &'a Stored, path: &FilePath) -> Option<&'a StoredFile> {
    match stored.files.binary_search_by(|file| file.path.cmp(path)) {
        Ok(place) => stored.files.get(place),
        // The search answers where the path would go: the path is not kept.
        Err(_) => None,
    }
}

/// Whether the system says the file is as the store kept it.
#[must_use]
pub(crate) fn as_kept(kept: &StoredFile, file: &CorpusEntry) -> Freshness {
    // A time the system does not know says nothing of the file.
    if kept.size == file.size
        && kept.modified == file.modified
        && kept.executable == file.executable
        && file.modified != Nanoseconds(0)
    {
        Freshness::AsItWas
    } else {
        Freshness::Changed
    }
}

/// What a run does with what the last kept, before a file is read: everything again when the store holds nothing, or was kept for
/// other rules, or for other files while a rule reads across files; the files as they were again when nothing changed that a rule
/// reading across files could see; a check of what the changed files show otherwise.
#[must_use]
pub(crate) fn decide(stored: &Stored, rules: &[StoredRule], files: &[CorpusInput]) -> Decision {
    if *stored == Stored::default() || stored.rules != rules {
        return Decision::Whole;
    }
    let across = rules.iter().any(|rule| rule.crossing == Crossing::Across);
    // A failed primary input has no reusable fresh record to compare with its old interface.
    // Recompute current cross-file rows/coverage rather than infer an unchanged empty delta.
    if across
        && files
            .iter()
            .any(|input| !matches!(input, CorpusInput::Available(_)))
    {
        return Decision::Whole;
    }
    let same_files = files.len() == stored.files.len()
        && files
            .iter()
            .zip(&stored.files)
            .all(|(file, kept)| ryo_values::domain::files::service::input_path(file) == &kept.path);
    if across && !same_files {
        return Decision::Cross;
    }
    let changed: Vec<FileOrder> = files
        .iter()
        .enumerate()
        .filter(|(_, input)| match input {
            CorpusInput::Available(file) => stored_at(stored, &file.path)
                .is_none_or(|kept| as_kept(kept, file) == Freshness::Changed),
            CorpusInput::ReadFailed { .. } | CorpusInput::Unreadable { .. } => true,
        })
        .map(|(place, _)| FileOrder(place))
        .collect();
    if !across || changed.is_empty() {
        Decision::Again
    } else {
        Decision::Check(changed)
    }
}

/// What the rules gave of a file, when they gave anything.
const fn ran_of(outcome: &FileOutcome) -> Option<&Ran> {
    match outcome {
        FileOutcome::Read(ran) => Some(ran),
        FileOutcome::Unread { ran, .. } => ran.as_ref(),
    }
}

/// Which rules that read across files are made again, after the files `changed` were read alone: every one for a file the store
/// has no row of; for a changed file, the rules whose part of its frontier changed, the rules that read the project (`project`, by
/// their place) when its notes or partly read relations changed, it is one of `manifests` (in the order of their paths) or the tools
/// refused it, and the rules that found something about it across files or could not decide in it. The rules are numbered by their
/// place among `ids`.
#[must_use]
pub(crate) fn made_again(
    stored: &Stored,
    changed: &[Record],
    manifests: &[FilePath],
    project: &[PlacedRule],
    ids: &[RuleId],
) -> MadeAgain {
    let place_of = |rule: &RuleId| {
        ids.iter()
            .position(|id| id == rule)
            .map(|place| PlacedRule(u32::try_from(place).unwrap_or(u32::MAX)))
    };
    // What the rules found about each file across files or could not decide in it, by the file: read once, so that many changed
    // files cost one pass over the findings.
    // A finding is about a file when its subject is a path: a file cell, or a text that spells one.
    let mut held: Vec<(FilePath, Option<PlacedRule>)> = stored
        .findings
        .iter()
        .filter(|finding| finding.made == FindingMade::AcrossFiles)
        .filter_map(|finding| match &finding.subject {
            StoredCell::Value(Cell::File(path) | Cell::Text(path)) => {
                Some((FilePath(path.clone()), place_of(&finding.rule)))
            }
            StoredCell::Value(Cell::Number(_) | Cell::Span(_)) | StoredCell::Nothing => None,
        })
        .chain(
            stored
                .unknowns
                .iter()
                .map(|each| (each.path.clone(), place_of(&each.rule))),
        )
        .chain(
            stored
                .skipped
                .iter()
                .map(|each| (each.path.clone(), place_of(&each.rule))),
        )
        .collect();
    held.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let mut rules = Vec::new();
    for record in changed {
        let Ok(place) = stored
            .files
            .binary_search_by(|file| file.path.cmp(&record.path))
        else {
            return MadeAgain::All;
        };
        let Some(kept) = stored.files.get(place) else {
            return MadeAgain::All;
        };
        if record.executable != kept.executable {
            return MadeAgain::All;
        }
        if record.digest == kept.digest {
            continue;
        }
        let Some(ran) = ran_of(&record.outcome) else {
            return MadeAgain::All;
        };
        if ran.interface == kept.interface {
            continue;
        }
        if ran.noted != kept.noted
            || manifests.binary_search(&record.path).is_ok()
            || stored.refused.iter().any(|each| each.path == record.path)
        {
            rules.extend_from_slice(project);
        }
        let start = held.partition_point(|each| each.0 < record.path);
        for (path, rule) in held.get(start..).unwrap_or_default() {
            if *path != record.path {
                break;
            }
            match rule {
                Some(rule) => rules.push(*rule),
                None => return MadeAgain::All,
            }
        }
        let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
        let before = rows_of_file(&stored.frontiers, file, |each| each.file);
        for rule in before
            .iter()
            .map(|each| each.rule)
            .chain(ran.frontier.iter().map(|each| each.rule))
        {
            let old = before
                .iter()
                .find(|each| each.rule == rule)
                .map(|each| each.digest);
            let new = ran
                .frontier
                .iter()
                .find(|each| each.rule == rule)
                .map(|each| each.digest);
            if old != new {
                rules.push(rule);
            }
        }
    }
    rules.sort_unstable_by_key(|rule| rule.0);
    rules.dedup();
    if rules.is_empty() {
        MadeAgain::Nothing
    } else {
        MadeAgain::These(rules)
    }
}

/// What a run found of each file the store kept, as the run would have found it, in the order of the stored files: its findings
/// and why no language read it; none when a finding's rule is not one of `ids`.
#[must_use]
pub(crate) fn records_from(stored: &Stored, ids: &[RuleId]) -> Option<Vec<Record>> {
    let mut owned: Vec<Vec<&StoredFinding>> = vec![Vec::new(); stored.files.len()];
    for finding in &stored.findings {
        if let FindingMade::InFile(file) = finding.made {
            owned
                .get_mut(usize::try_from(file.0).unwrap_or(usize::MAX))?
                .push(finding);
        }
    }
    let mut unread: Vec<Option<&StoredUnread>> = vec![None; stored.files.len()];
    for each in &stored.unread {
        *unread.get_mut(usize::try_from(each.file.0).unwrap_or(usize::MAX))? = Some(each);
    }
    stored
        .files
        .iter()
        .zip(owned)
        .zip(unread)
        .enumerate()
        .map(|(place, ((file, findings), unread))| {
            let number = FileNumber(u32::try_from(place).unwrap_or(u32::MAX));
            let ran = Ran {
                rows: rows_from(&findings, ids)?,
                number,
                interface: file.interface,
                handed: Handing::Kept,
                frontier: rows_of_file(&stored.frontiers, StoredFileNumber(number.0), |each| {
                    each.file
                })
                .iter()
                .map(|each| RuleFrontier {
                    rule: each.rule,
                    digest: each.digest,
                })
                .collect(),
                noted: file.noted,
            };
            Some(Record {
                executable: file.executable,
                path: file.path.clone(),
                size: file.size,
                modified: file.modified,
                digest: file.digest,
                number,
                outcome: match unread {
                    Some(each) => FileOutcome::Unread {
                        reason: each.reason.clone(),
                        ran: Some(ran),
                    },
                    None => FileOutcome::Read(ran),
                },
            })
        })
        .collect()
}

/// The findings of the rules that read across files, as the store kept them; none when a finding's rule is not one of `ids`.
#[must_use]
pub(crate) fn later_from(
    stored: &Stored,
    ids: &[RuleId],
    made_again: &[RuleId],
) -> Option<FindingRows> {
    let across: Vec<&StoredFinding> = stored
        .findings
        .iter()
        .filter(|finding| {
            finding.made == FindingMade::AcrossFiles
                && made_again.iter().all(|rule| *rule.0 != *finding.rule.0)
        })
        .collect();
    rows_from(&across, ids)
}

/// What the rules could not decide, as the store kept it, but in the files read again, whose own say now.
#[must_use]
pub(crate) fn unknowns_from(stored: &Stored, read_again: &[FilePath]) -> Vec<Unknown> {
    stored
        .unknowns
        .iter()
        .filter(|each| read_again.binary_search(&each.path).is_err())
        .map(|each| Unknown {
            rule: RuleId(each.rule.0.clone()),
            path: each.path.clone(),
            message: Message(each.message.0.clone()),
        })
        .collect()
}

/// The files read again, and with them the files the store kept that the run no longer lists: they are gone, so what the store kept
/// of them answers nothing about this run. Sorted, once each, as `unknowns_from` and `skipped_from` search them.
#[must_use]
pub(crate) fn with_gone(
    mut read_again: Vec<FilePath>,
    stored: &Stored,
    listed: &[&FilePath],
) -> Vec<FilePath> {
    let listed: std::collections::BTreeSet<&FilePath> = listed.iter().copied().collect();
    read_again.extend(
        stored
            .files
            .iter()
            .filter(|kept| !listed.contains(&kept.path))
            .map(|kept| kept.path.clone()),
    );
    read_again.sort();
    read_again.dedup();
    read_again
}

/// What the rules explicitly skipped, as the store kept it, but not for files read again.
#[must_use]
pub(crate) fn skipped_from(stored: &Stored, read_again: &[FilePath]) -> Vec<Skipped> {
    stored
        .skipped
        .iter()
        .filter(|each| read_again.binary_search(&each.path).is_err())
        .cloned()
        .collect()
}

/// The records the store kept, with those of `fresh` in place of the ones of the same path and among them by path: both in the order
/// of their paths.
#[must_use]
pub(crate) fn with_fresh(kept: Vec<Record>, fresh: Vec<Record>) -> Vec<Record> {
    let mut merged = Vec::with_capacity(kept.len() + fresh.len());
    let mut fresh = fresh.into_iter().peekable();
    for record in kept {
        while let Some(new) = fresh.next_if(|new| new.path < record.path) {
            merged.push(new);
        }
        match fresh.next_if(|new| new.path == record.path) {
            Some(new) => merged.push(new),
            None => merged.push(record),
        }
    }
    merged.extend(fresh);
    merged
}

/// What a run keeps in its store: its rules, a row for each file, the findings, why a file was not read, the manifests refused, what
/// the rules could not decide, the files the lookup read, and the words as they were: the buckets `unchanged` and the files used again.
#[must_use]
pub(crate) fn stored_of<W>(
    keeping: ToKeep<'_>,
    last: &Stored,
    last_handed: &ryo_values::domain::stores::types::HandedRows<W>,
    rebuilt: &BucketsWanted,
    unchanged: Vec<Bucket>,
) -> ToStore {
    let ToKeep {
        rules,
        records,
        later,
        refused,
        unknowns,
        skipped,
        consulted,
        consulted_absent,
        project_observations,
    } = keeping;
    let mut stored = Stored {
        rules,
        refused: refused.to_vec(),
        consulted: consulted.to_vec(),
        consulted_absent: consulted_absent.to_vec(),
        project_observations: project_observations.to_vec(),
        unknowns: unknowns
            .iter()
            .map(|each| StoredUnknown {
                rule: RuleId(each.rule.0.clone()),
                path: each.path.clone(),
                message: UnknownText(each.message.0.clone()),
            })
            .collect(),
        skipped: skipped.to_vec(),
        ..Stored::default()
    };
    let mut kept = KeptRows {
        unchanged,
        ..KeptRows::default()
    };
    stored.files.reserve_exact(records.len());
    // Each record is let go once it is a row of the store, so the findings are never held twice over.
    for (place, mut record) in records.into_iter().enumerate() {
        let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
        // What a file read now handed moves into the store; what a file used again handed is taken from the last store.
        let handing = match &mut record.outcome {
            FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. } => {
                Some(std::mem::replace(&mut ran.handed, Handing::Kept))
            }
            FileOutcome::Unread { ran: None, .. } => None,
        };
        match handing {
            Some(Handing::Made(handed)) => {
                hand_into(&mut stored.handed, file, handed, rebuilt, &mut kept.staged)
            }
            Some(Handing::Kept) => {
                // Its words stay where the store keeps them, moved to its number now.
                if let Ok(was) = last
                    .files
                    .binary_search_by(|each| each.path.cmp(&record.path))
                {
                    kept.file_again.push(FileAgain {
                        was: StoredFileNumber(u32::try_from(was).unwrap_or(u32::MAX)),
                        now: file,
                    });
                }
                hand_into(
                    &mut stored.handed,
                    file,
                    handed_in(last, last_handed, &record.path),
                    rebuilt,
                    &mut kept.staged,
                );
            }
            None => {}
        }
        let ran = ran_of(&record.outcome);
        if let Some(ran) = ran {
            stored
                .frontiers
                .extend(ran.frontier.iter().map(|each| StoredFrontier {
                    file,
                    rule: each.rule,
                    digest: each.digest,
                }));
        }
        stored.files.push(StoredFile {
            executable: record.executable,
            path: record.path,
            digest: record.digest,
            noted: ran.map_or(Digest([0; 16]), |ran| ran.noted),
            interface: ran.map_or(Digest([0; 16]), |ran| ran.interface),
            size: record.size,
            modified: record.modified,
        });
        if let Some(ran) = ran {
            stored
                .findings
                .extend(findings_of(&ran.rows, FindingMade::InFile(file)));
        }
        if let FileOutcome::Unread { reason, .. } = record.outcome {
            stored.unread.push(StoredUnread { file, reason });
        }
    }
    stored
        .findings
        .extend(findings_of(later, FindingMade::AcrossFiles));
    ToStore { stored, kept }
}
