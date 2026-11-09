//! The `recording` service: free functions named by verb over what a visit of a file comes to for the store, with no IO and no ports.

use super::constants::PHYSICAL_FACTS_INCOMPLETE;
use super::types::{BucketWords, Flat, InterfaceBytes, WordCount};
use crate::domain::handing::service::stored_of;
use crate::domain::keeping::service::{remap_packed, rows_of};
use crate::domain::keeping::types::{ArenaStart, Remap};
use crate::domain::render::types::Answering;
use crate::domain::visit::types::{Keeping, Noting};
use crate::domain::workers::types::{
    FileOrder, FreshRecord, Gating, PlacedNotes, Tables, VisitSetting,
};
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::Governing;
use ryo_execution::inbound::use_cases::metadata::interface::{Bearing, Blocking};
use ryo_execution::inbound::use_cases::metadata::interface::{
    CoverageScope, Partial, RuleIndex, Unknown,
};
use ryo_language::domain::elements::service::{notes_of, take_notes};
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{FileId, Words, WorkerNumber};
use ryo_values::domain::files::types::{CorpusEntry, FileNumber};
use ryo_values::domain::records::types::HandedNotes;
use ryo_values::domain::records::types::{
    FileOutcome, GoverningRules, Handed, HandedWords, Handing, Ran, Record,
};
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleNumber;
use ryo_values::domain::stored_facts::types::{
    ArenaBase, ArenaBytes, Digest, FileNotes, NoteBytes,
};
use ryo_values::domain::table_words::service::{pack, unpacked};
use ryo_values::domain::table_words::types::WordSlice;

/// Whether [`cover`] names a rule that reads as `needs` at a file read in part as `partials`: a relation of the file the rule reads
/// is read in part there, so the rule's unknown at the file is already given with the relation and why.
pub(crate) fn names_at_file(
    metadata: &impl RunDecisions,
    partials: &[Partial],
    needs: Option<&ryo_execution::inbound::use_cases::metadata::interface::Needs>,
) -> Bearing {
    let named = needs.is_some_and(|needs| {
        partials.iter().any(|partial| {
            needs
                .reads
                .iter()
                .any(|read| metadata.matters(read, partial) == Bearing::Matters)
        })
    });
    if named {
        Bearing::Matters
    } else {
        Bearing::DoesNotMatter
    }
}

/// What a rule could not decide because the engine answered its root in the file `at` only in part, the partitions still uncertain
/// being `affected`. When they are this file alone and [`cover`] names the rule at the file, a second unknown without the reason
/// would report the same uncertainty twice; any other partition (another file, the repository) is uncertainty `cover` does not see
/// from this file, and the rule says so here.
pub(crate) fn partial_root(
    metadata: &impl RunDecisions,
    unknowns: &mut Vec<Unknown>,
    partials: &[Partial],
    setting: &VisitSetting<'_>,
    rule: RuleIndex,
    at: FileId,
    path: &FilePath,
    affected: &[Option<FileId>],
) {
    let own = !affected.is_empty() && affected.iter().all(|file| *file == Some(at));
    if own && names_at_file(metadata, partials, setting.needs.get(rule.0)) == Bearing::Matters {
        return;
    }
    unknowns.push(Unknown {
        rule: metadata.rule_id(setting.ids, rule),
        path: path.clone(),
        message: ryo_execution::inbound::use_cases::metadata::interface::Message(
            PHYSICAL_FACTS_INCOMPLETE.into(),
        ),
        cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
    });
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
            proof: each.proof.clone(),
            word_count: WordCount(words.0.len()),
            words: pack(WordSlice(&words.0)),
        });
    }
    Some(moved)
}

/// What the file handed to one rule, apart: the buckets of the rule's frontier with their words, one after the other. An edit that
/// changes only what one rule reads leaves the others' steps as they were.
pub(crate) fn frontier_flat(
    moved: &[BucketWords],
    observations: Option<ryo_values::domain::observations::validated::ValidatedFrontiers<'_>>,
    slots: &[ryo_execution::inbound::use_cases::execution::interface::ObservedFrontierRoot],
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
            u64::try_from(each.word_count.0).unwrap_or(u64::MAX),
        ]);
        flat.0.extend(unpacked(&each.words).map(|word| word.0));
    }
    if let Some(observations) = observations {
        let start = slots.partition_point(|slot| slot.producer.0 < rule.0);
        let end = slots.partition_point(|slot| slot.producer.0 <= rule.0);
        if start != end {
            use ryo_values::domain::observations::types::ObservationReadinessRange;
            use ryo_values::domain::records::types::ObservationReadinessCount;
            use ryo_values::domain::records::types::ObservationReadinessSlot;
            let identity = ryo_values::domain::observations::service::frontier_identity(
                observations,
                ObservationReadinessRange {
                    start: ObservationReadinessSlot(u32::try_from(start).unwrap_or(u32::MAX)),
                    length: ObservationReadinessCount(
                        u32::try_from(end - start).unwrap_or(u32::MAX),
                    ),
                },
            );
            // A range the prepared layout does not hold has no identity: it is written as an identity of no bytes, which no
            // valid range has, so it never passes for one.
            let bytes = identity.map_or_else(|_| Vec::new(), |identity| identity.0);
            flat.0
                .extend([u64::MAX, u64::try_from(bytes.len()).unwrap_or(u64::MAX)]);
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
            observations: match tables.current_observations.clone() {
                Some(root) => ryo_values::domain::records::types::HandedReadiness::Recorded(root),
                None => ryo_values::domain::records::types::HandedReadiness::Unrecorded,
            },
            buckets: moved
                .into_iter()
                .map(|each| HandedWords {
                    bucket: each.bucket,
                    proof: each.proof,
                    handed: each.words.into(),
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
    metadata: &impl RunDecisions,
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
        let applicable = if setting.metadata.is_empty()
            || setting.applicability_can_gate == Gating::NeverGates
        {
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
        if metadata.governs(
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
                .any(|read| metadata.matters(read, partial) == Bearing::Matters)
            {
                tables.unknowns.push(Unknown {
                    rule: metadata.rule_id(setting.ids, RuleIndex(rule)),
                    path: path.clone(),
                    message: metadata.unknown_message(partial, CoverageScope::File(path)),
                    cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
                });
            }
        }
        if metadata.blocked(&needs.cross, &tables.partials) == Blocking::Blocked {
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

use crate::domain::workers::types::RunDecisions;
