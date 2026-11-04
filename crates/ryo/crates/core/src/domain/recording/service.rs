//! The `recording` service: free functions named by verb over what a visit of a file comes to for the store, with no IO and no ports.

use super::types::{BucketWords, Flat, InterfaceBytes};
use crate::domain::coverage::service::unknown_message;
use crate::domain::coverage::types::Covered;
use crate::domain::coverage::types::{
    CoverageScope, Lacking, LackingKind, Partial, RuleIndex, Unknown,
};
use crate::domain::finish::types::RuleNumber;
use crate::domain::keeping::service::{remap_packed, rendered_of, rows_of};
use crate::domain::keeping::types::{ArenaStart, Remap};
use crate::domain::placement::service::governs;
use crate::domain::placement::types::Governing;
use crate::domain::render::types::Answering;
use crate::domain::rules::service::id_at;
use crate::domain::table_words::service::{pack, unpacked};
use crate::domain::table_words::types::WordSlice;
use crate::domain::trust::service::{blocked, matters};
use crate::domain::trust::types::{Bearing, Blocking};
use crate::domain::visit::types::{Keeping, Noting};
use crate::domain::workers::types::{FileOrder, FreshRecord, PlacedNotes, Tables, VisitSetting};
use ryo_language::domain::elements::service::{notes_of, take_notes};
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::service::{name, relation_named};
use ryo_relations::domain::relations::types::NameRef;
use ryo_relations::domain::relations::types::{FileId, Words, WorkerNumber};
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::files::types::{CorpusEntry, FileNumber};
use ryo_values::domain::records::types::HandedNotes;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::records::types::{
    FileOutcome, GoverningRules, Handed, HandedWords, Handing, Ran, Record,
};
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::stored_facts::types::{
    ArenaBase, ArenaBytes, Digest, FileNotes, NoteBytes, PartialAt, PartialCause, PartialKind,
    PartialScope, StoredPartial,
};

/// The relations a record says were read in part, as the coverage steps take them.
#[must_use]
pub(crate) fn partials_of(stored: &[StoredPartial]) -> Vec<Partial> {
    stored
        .iter()
        .filter_map(|each| {
            relation_named(NameRef(&each.relation_name.0)).map(|relation| Partial {
                relation,
                cause: PartialCause(each.cause.0.clone()),
                at: match each.at {
                    PartialScope::OneFile(file) => Some(FileId(file.0)),
                    PartialScope::Repository => None,
                },
                lacking: if each.lacking_kind.0.is_empty() {
                    Lacking::Any
                } else {
                    Lacking::Kind(LackingKind(each.lacking_kind.0.clone()))
                },
            })
        })
        .collect()
}

/// The relations read in part, as a record keeps them.
#[must_use]
pub(crate) fn stored_of(partials: &[Partial]) -> Vec<StoredPartial> {
    partials
        .iter()
        .map(|each| StoredPartial {
            relation_name: RelationName(name(each.relation).0.into()),
            cause: PartialCause(each.cause.0.clone()),
            at: each.at.map_or(PartialScope::Repository, |at| {
                PartialScope::OneFile(PartialAt(at.0))
            }),
            lacking_kind: PartialKind(match &each.lacking {
                Lacking::Any => Box::default(),
                Lacking::Kind(kind) => kind.0.clone(),
            }),
        })
        .collect()
}

/// What the file handed to the cross-file steps with the numbers that differ from run to run taken out, to say whether they are
/// as they were, normalized and packed one bucket at a time. Words that point at another file or at another worker's text
/// cannot be used by another run: none then, before the application makes any digest call.
#[must_use]
pub(crate) fn moved_words(
    tables: &Tables,
    number: FileId,
    base: ArenaBase,
) -> Option<Vec<BucketWords>> {
    let plain = Remap {
        from_file: number,
        to_file: FileId(0),
        from_worker: tables.number,
        to_worker: WorkerNumber(0),
        from_base: ArenaStart(base.0),
        to_base: ArenaStart(0),
    };
    let mut moved = Vec::with_capacity(tables.captured.len());
    // Only the largest single normalized u64 bucket is live; all retained buckets are packed.
    // Every remap still completes before the application calls the digest port.
    let mut words = Words::default();
    for each in &tables.captured {
        words.0.clear();
        remap_packed(&each.words.0, &plain, &mut words)?;
        moved.push(BucketWords {
            bucket: each.bucket,
            word_count: words.0.len(),
            words: pack(WordSlice(&words.0)),
        });
    }
    Some(moved)
}

/// What the file handed to one rule, apart: the buckets of the rule's frontier with their words, one after the other. An edit that
/// changes only what one rule reads leaves the others' steps as they were.
pub(crate) fn frontier_flat(
    moved: &[BucketWords],
    observations: Option<ryo_values::domain::observations::types::ValidatedFrontiers<'_>>,
    slots: &[crate::domain::file_program::types::ObservedFrontierRoot],
    setting: &VisitSetting<'_>,
    rule: RuleIndex,
    flat: &mut Flat,
) {
    let first = setting
        .first_bucket
        .get(rule.0)
        .map_or(u32::MAX, |bucket| bucket.0);
    let places = setting
        .metadata
        .get(rule.0)
        .map_or(0, |rule| rule.frontier_capture.len());
    flat.0.clear();
    for each in moved.iter().filter(|each| {
        each.bucket.0 >= first
            && usize::try_from(each.bucket.0 - first).is_ok_and(|place| place < places)
    }) {
        flat.0.extend([
            u64::from(each.bucket.0),
            u64::try_from(each.word_count).unwrap_or(u64::MAX),
        ]);
        flat.0.extend(unpacked(&each.words).map(|word| word.0));
    }
    if let Some(observations) = observations {
        let start = slots.partition_point(|slot| slot.producer.0 < rule.0);
        let end = slots.partition_point(|slot| slot.producer.0 <= rule.0);
        if start != end {
            use ryo_values::domain::observations::types::{
                ObservationReadinessCount, ObservationReadinessRange, ObservationReadinessSlot,
            };
            let bytes = ryo_values::domain::observations::service::frontier_identity(
                observations,
                ObservationReadinessRange {
                    start: ObservationReadinessSlot(
                        u32::try_from(start).expect("prepared status slot fits u32"),
                    ),
                    length: ObservationReadinessCount(
                        u32::try_from(end - start).expect("prepared status range fits u32"),
                    ),
                },
            )
            .expect("validated status range follows the prepared root layout")
            .0;
            flat.0.extend([
                u64::MAX,
                u64::try_from(bytes.len()).expect("readiness identity length fits u64"),
            ]);
            for chunk in bytes.chunks(8) {
                let mut word = [0; 8];
                word[..chunk.len()].copy_from_slice(chunk);
                flat.0.push(u64::from_le_bytes(word));
            }
        }
    }
}

/// What the rules gave of the file: its findings, what it shows other files, and the words and the text it hands the steps that
/// read across files, moved as `moved_words` moves them, its text from `base` on.
#[must_use]
pub(crate) fn made_ran(
    tables: &Tables,
    number: FileId,
    interface: Digest,
    moved: Vec<BucketWords>,
    base: ArenaBase,
) -> Ran {
    Ran {
        rows: rows_of(&tables.rendered),
        number: FileNumber(number.0),
        interface,
        frontier: Vec::new(),
        noted: Digest([0; 16]),
        handed: Handing::Made(Handed {
            observations: tables.current_observations.clone(),
            buckets: moved
                .into_iter()
                .map(|each| HandedWords {
                    bucket: each.bucket,
                    words: each.words.into(),
                })
                .collect(),
            text: ArenaBytes(
                tables
                    .arena
                    .0
                    .get(usize::try_from(base.0).unwrap_or(usize::MAX)..)
                    .unwrap_or_default()
                    .to_vec(),
            ),
            ..Handed::default()
        }),
    }
}

/// What a file read now hands the project and the steps that read across files besides its words: its notes and, when a relation
/// of it is read in part, the rules that govern it and those relations.
fn hand_the_rest(ran: &mut Ran, notes: Option<&FileNotes>, covered: Option<&Covered>) {
    let Handing::Made(handed) = &mut ran.handed else {
        return;
    };
    if let Some(notes) = notes {
        handed.notes = HandedNotes::Noted(notes.clone());
    }
    if let Some(covered) = covered {
        handed.governing = GoverningRules(
            covered
                .rules
                .iter()
                .map(|rule| u32::try_from(rule.0).unwrap_or(u32::MAX))
                .collect(),
        );
        handed.partials = stored_of(&covered.partial);
    }
}

/// What a file hands the project, as the bytes its interface is the digest of: the language that noted it and what it noted, then
/// the relations it is read in part.
#[must_use]
pub(crate) fn interface_bytes(tables: &Tables) -> InterfaceBytes {
    let mut bytes = Vec::new();
    let notes = &notes_of(&tables.elements).0;
    if let (Some(language), false) = (tables.language, notes.is_empty()) {
        bytes.extend_from_slice(&language.0.to_le_bytes());
        bytes.extend_from_slice(notes);
    }
    for each in stored_of(&tables.partials) {
        bytes.extend_from_slice(format!("{each:?}").as_bytes());
    }
    InterfaceBytes(bytes)
}

/// Notes what a file read in part costs the rules: what each could not decide in the file, and the file's coverage for the
/// cross-file steps that trust the absence of rows of what is partial.
pub(crate) fn cover(
    tables: &mut Tables,
    setting: &VisitSetting<'_>,
    at: FileId,
    path: &FilePath,
    acceptance: SyntaxAcceptance,
) {
    if tables.partials.is_empty() {
        return;
    }
    let mut rules = Vec::new();
    for (rule, needs) in setting.needs.iter().enumerate() {
        let applicable = if setting.metadata.is_empty() || !setting.applicability_can_gate {
            true
        } else {
            match setting.applicability.get(rule).copied() {
                Some(RuleApplicability::Always) => true,
                Some(RuleApplicability::AcceptedSyntax) => acceptance == SyntaxAcceptance::Accepted,
                None => false,
            }
        };
        if !applicable {
            continue;
        }
        if governs(
            &tables.mask,
            RuleNumber(u32::try_from(rule).unwrap_or(u32::MAX)),
        ) == Governing::DoesNotGovern
        {
            continue;
        }
        for partial in &tables.partials {
            if needs
                .reads
                .iter()
                .any(|read| matters(read, partial) == Bearing::Matters)
            {
                tables.unknowns.push(Unknown {
                    rule: id_at(setting.ids, RuleIndex(rule)),
                    path: path.clone(),
                    message: unknown_message(partial, CoverageScope::File(path)),
                });
            }
        }
        if blocked(&needs.cross, &tables.partials) == Blocking::Blocked {
            rules.push(RuleIndex(rule));
        }
    }
    if !rules.is_empty() {
        tables.covered.push(Covered {
            file: at,
            rules,
            partial: tables.partials.clone(),
        });
    }
}

/// Puts the findings the store kept of a file where the report takes them from.
fn restore(tables: &mut Tables, ran: &Ran) -> Ran {
    tables.rendered = rendered_of(&ran.rows);
    ran.clone()
}

/// Uses what the store kept of a file again, instead of reading it: its findings leave as they were, renumbered for this run in the
/// record the run keeps. What the rules could not decide in it is what the store kept of that.
pub(crate) fn replay(
    tables: &mut Tables,
    number: FileId,
    at: FileOrder,
    record: &Record,
    file: &CorpusEntry,
    digest: Digest,
) {
    let renewed = |outcome: FileOutcome| Record {
        executable: file.executable,
        path: record.path.clone(),
        size: file.size,
        modified: file.modified,
        digest,
        number: FileNumber(number.0),
        outcome,
    };
    tables.partials.clear();
    let outcome = match &record.outcome {
        FileOutcome::Unread { reason, ran } => {
            let renewed_ran = ran.as_ref().map(|ran| restore(tables, ran));
            tables.unread.push(Unread {
                path: file.path.clone(),
                reason: reason.clone(),
            });
            FileOutcome::Unread {
                reason: reason.clone(),
                ran: renewed_ran,
            }
        }
        FileOutcome::Read(ran) => FileOutcome::Read(restore(tables, ran)),
    };
    tables.fresh.push(FreshRecord {
        place: at,
        record: renewed(outcome),
    });
}

/// Ends the visit of a file: the file's notes wait for the project, and its record for the store, when the run keeps one.
pub(crate) fn finish_file(
    tables: &mut Tables,
    setting: &VisitSetting<'_>,
    at: FileOrder,
    file: &CorpusEntry,
    digest: Digest,
    number: FileNumber,
    outcome: Option<FileOutcome>,
) {
    if setting.noted == Noting::Wanted
        && !notes_of(&tables.elements).0.is_empty()
        && let Some(language) = tables.language
    {
        tables.notes.push(PlacedNotes {
            place: at,
            notes: FileNotes {
                language,
                notes: NoteBytes(take_notes(&mut tables.elements).0),
            },
        });
    }
    match (setting.answering, outcome) {
        (Answering::Findings, Some(mut outcome)) => {
            let notes = tables
                .notes
                .last()
                .filter(|each| each.place == at)
                .map(|each| &each.notes);
            let covered = tables.covered.last().filter(|each| each.file.0 == number.0);
            match &mut outcome {
                FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. }
                    if setting.keeping == Keeping::Keeps =>
                {
                    hand_the_rest(ran, notes, covered);
                }
                FileOutcome::Read(_) | FileOutcome::Unread { .. } => {}
            }
            tables.fresh.push(FreshRecord {
                place: at,
                record: Record {
                    executable: file.executable,
                    path: file.path.clone(),
                    size: file.size,
                    modified: file.modified,
                    digest,
                    number,
                    outcome,
                },
            });
        }
        // What this file handed cannot be moved to another run: it keeps no record, and the next run reads it again.
        (Answering::Findings, None) => tables.unrecorded.push(file.path.clone()),
        (Answering::Rows(_), _) => {}
    }
}

/// How the words a file handed the last run move to this run, read by the worker of `tables` as the file `number`: as if the run
/// had read it, its text after what the worker's arena holds now. Taken before the file's text joins the arena.
#[must_use]
pub(crate) fn remap_to(tables: &Tables, number: FileId) -> Remap {
    Remap {
        from_file: FileId(0),
        to_file: number,
        from_worker: WorkerNumber(0),
        to_worker: tables.number,
        from_base: ArenaStart(0),
        to_base: ArenaStart(u32::try_from(tables.arena.0.len()).unwrap_or(u32::MAX)),
    }
}

/// Hands what a file the run does not read handed the last run but its words to the steps that read across files and the project,
/// as if the run had read it: its text joins the worker's arena, and its notes and what of it is read in part join the worker's.
pub(crate) fn hand_rest_again(
    tables: &mut Tables,
    at: FileOrder,
    number: FileId,
    text: &ArenaBytes,
    notes: Option<&FileNotes>,
    governing: &GoverningRules,
    partials: &[StoredPartial],
    observations: Option<&std::sync::Arc<ryo_values::domain::records::types::FrontierReadiness>>,
) {
    if let Some(root) = observations {
        tables
            .observations
            .push(crate::domain::coverage::types::FileObservationReadiness {
                file: number,
                root: std::sync::Arc::clone(root),
            });
    }
    tables.arena.0.extend_from_slice(&text.0);
    if let Some(notes) = notes {
        tables.notes.push(PlacedNotes {
            place: at,
            notes: notes.clone(),
        });
    }
    if !governing.0.is_empty() {
        tables.covered.push(Covered {
            file: number,
            rules: governing
                .0
                .iter()
                .map(|rule| RuleIndex(usize::try_from(*rule).unwrap_or(usize::MAX)))
                .collect(),
            partial: partials_of(partials),
        });
    }
}
