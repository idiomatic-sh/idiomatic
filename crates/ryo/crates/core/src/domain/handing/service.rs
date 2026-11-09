//! The `handing` service: free functions named by verb over what a file handed the steps that read across files and the rows the
//! store keeps of it, with no IO and no ports.

use crate::domain::keeping::service::rendered_of;
use crate::domain::keeping::types::{ArenaStart, Remap};
use crate::domain::workers::types::{FileOrder, FreshRecord, PlacedNotes, Tables};
use ryo_execution::inbound::use_cases::metadata::interface::{
    Covered, Lacking, LackingKind, Partial, RuleIndex,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service::name;
use ryo_relations::domain::relations::service::relation_named;
use ryo_relations::domain::relations::types::{FileId, WorkerNumber};
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Ran;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::records::types::{GoverningRules, Handed, HandedNotes, RecordWords};
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::PartialAt;
use ryo_values::domain::stored_facts::types::PartialKind;
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, FileNotes, PartialCause, PartialScope, StoredPartial,
};
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::{
    BucketsWanted, Stored, StoredCovered, StoredHanded, StoredNotes, StoredText, StoredWords,
};

/// The rows of `rows` of the stored file `file`: the tables keep each file's rows together, in the order of the files.
pub(crate) fn rows_of_file<T>(
    rows: &[T],
    file: StoredFileNumber,
    of: impl Fn(&T) -> StoredFileNumber,
) -> &[T] {
    let start = rows.partition_point(|each| of(each) < file);
    let end = rows.partition_point(|each| of(each) <= file);
    rows.get(start..end).unwrap_or_default()
}

/// What the file at `path` handed the steps that read across files when the store was kept but its words, which the store keeps
/// where they are; nothing when the store has no row of it.
#[must_use]
pub(crate) fn handed_in<W>(
    stored: &Stored,
    handed: &ryo_values::domain::stores::types::HandedRows<W>,
    path: &FilePath,
) -> Handed {
    let Ok(place) = stored.files.binary_search_by(|file| file.path.cmp(path)) else {
        return Handed::default();
    };
    let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
    let covered = rows_of_file(&handed.covered, file, |each| each.file).first();
    Handed {
        observations: match rows_of_file(&handed.observations, file, |each| each.file) {
            [one] => ryo_values::domain::records::types::HandedReadiness::Recorded(
                std::sync::Arc::clone(&one.root),
            ),
            _ => ryo_values::domain::records::types::HandedReadiness::Unrecorded, // missing/ambiguous readiness remains unavailable to demanded roots
        },
        buckets: Vec::new(),
        text: rows_of_file(&handed.texts, file, |each| each.file)
            .first()
            .map(|each| each.text.clone())
            .unwrap_or_default(),
        notes: rows_of_file(&handed.notes, file, |each| each.file)
            .first()
            .map_or(HandedNotes::Nothing, |each| {
                HandedNotes::Noted(each.notes.clone())
            }),
        governing: covered
            .map(|each| each.governing.clone())
            .unwrap_or_default(),
        partials: covered
            .map(|each| each.partials.clone())
            .unwrap_or_default(),
    }
}

/// Puts what the file `file` handed the steps that read across files in the store's tables, a row only where there is something.
pub(crate) fn hand_into(
    stored: &mut StoredHanded,
    file: StoredFileNumber,
    handed: Handed,
    rebuilt: &BucketsWanted,
    staged: &mut Vec<ryo_values::domain::stores::types::StagedRow>,
) {
    let wanted = |bucket: Bucket| match rebuilt {
        BucketsWanted::All => true,
        BucketsWanted::These(buckets) => buckets.binary_search(&bucket).is_ok(),
    };
    for each in handed
        .buckets
        .into_iter()
        .filter(|each| wanted(each.bucket))
    {
        let inline = u64::try_from(stored.frontier.len()).unwrap_or(u64::MAX);
        let count = u64::try_from(staged.len()).unwrap_or(u64::MAX);
        let ordinal = inline.saturating_add(count);
        stored
            .proofs
            .push(ryo_values::domain::stores::types::StoredFrontierProof {
                file,
                bucket: each.bucket,
                proof: each.proof.clone(),
            });
        match each.handed {
            RecordWords::Inline(words) => stored.frontier.push(StoredWords {
                file,
                bucket: each.bucket,
                proof: each.proof,
                words,
            }),
            RecordWords::Staged(words) => {
                staged.push(ryo_values::domain::stores::types::StagedRow {
                    file,
                    bucket: each.bucket,
                    ordinal: ryo_values::domain::records::types::StagedOrdinal(ordinal),
                    words,
                })
            }
        }
    }
    if let ryo_values::domain::records::types::HandedReadiness::Recorded(root) = handed.observations
    {
        stored
            .observations
            .push(ryo_values::domain::stores::types::StoredFrontierReadiness { file, root });
    }
    if !handed.text.0.is_empty() {
        stored.texts.push(StoredText {
            file,
            text: handed.text,
        });
    }
    if let HandedNotes::Noted(notes) = handed.notes {
        stored.notes.push(StoredNotes { file, notes });
    }
    if !handed.governing.0.is_empty() {
        stored.covered.push(StoredCovered {
            file,
            governing: handed.governing,
            partials: handed.partials,
        });
    }
}

/// The relations a record says were read in part, as the coverage steps take them, for the file `own` the run numbers it: a part
/// kept of one file is of the file the record is of (see `stored_of`), whatever number the file had in the run that kept it.
#[must_use]
pub(crate) fn partials_of(stored: &[StoredPartial], own: FileId) -> Vec<Partial> {
    stored
        .iter()
        .filter_map(|each| {
            relation_named(NameRef(&each.relation_name.0)).map(|relation| Partial {
                relation,
                cause: PartialCause(each.cause.0.clone()),
                at: match each.at {
                    PartialScope::OneFile(_) => {
                        ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(
                            own,
                        )
                    }
                    PartialScope::Repository => {
                        ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::Repository
                    }
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
        tables.observations.push(
            ryo_execution::inbound::use_cases::metadata::interface::FileObservationReadiness {
                file: number,
                root: std::sync::Arc::clone(root),
            },
        );
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
            partial: partials_of(partials, number),
        });
    }
}

/// The relations of the file `own` read in part, as a record keeps them. The number a file has differs from run to run, so a part
/// read of the file itself is kept with no number, as `moved_words` keeps the file's own words at file 0: place 0 is the file the record
/// is of, and `partials_of` gives it the number the reading run has for it. A part said of another file has no place in this
/// file's record that a later run could number, so it is kept as the repository's: wider, never on the wrong file.
#[must_use]
pub(crate) fn stored_of(partials: &[Partial], own: FileId) -> Vec<StoredPartial> {
    partials
        .iter()
        .map(|each| StoredPartial {
            relation_name: RelationName(name(each.relation).0.into()),
            cause: PartialCause(each.cause.0.clone()),
            at: match each.at {
                ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(at)
                    if at == own =>
                {
                    PartialScope::OneFile(PartialAt(0))
                }
                ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(_)
                | ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::Repository => {
                    PartialScope::Repository
                }
            },
            lacking_kind: PartialKind(match &each.lacking {
                Lacking::Any => Box::default(),
                Lacking::Kind(kind) => kind.0.clone(),
            }),
        })
        .collect()
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
