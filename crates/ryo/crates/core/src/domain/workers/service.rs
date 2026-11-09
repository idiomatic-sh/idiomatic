//! The `workers` service: free functions named by verb over what each worker owns and what the workers make together, with no IO
//! and no ports.

use super::constants::MEMORY_EVERY;
use super::types::{
    Checking, FileOrder, FileTotal, FilesRead, Fresh, FreshNotes, PlacedNotes, Tables,
    VisitSetting, WorkersMet,
};
use crate::domain::render::types::Rendered;
use crate::domain::workers::types::RunDecisions;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::interface::{Governing, Mask};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::files::types::{CorpusInput, FileBytes, FileSize, Hits};
use ryo_values::domain::records::types::Record;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{ArenaBytes, FileNotes};
use {crate::domain::finish::types::LineStarts, ryo_values::domain::runs::types::RuleNumber};

/// What a worker owns before it reads a file: empty tables, and a pool big enough for any of the plans.
#[must_use]
pub(crate) fn tables_for(number: WorkerNumber) -> Tables {
    Tables {
        current_observations: None,
        observations: Vec::new(),
        number,
        elements: Elements::default(),
        symbols: Symbols::default(),
        bytes: FileBytes::default(),
        language: None,
        mask: Mask::default(),
        hits: Hits::default(),
        words: Words::default(),
        packed: PackedWords::default(),
        starts: LineStarts(Vec::new()),
        arena: ArenaBytes::default(),
        places: Vec::new(),
        rendered: Rendered::default(),
        unread: Vec::new(),
        unrecorded: Vec::new(),
        captured: Vec::new(),
        fresh: Vec::new(),
        notes: Vec::new(),
        reads: FilesRead::default(),
        partials: Vec::new(),
        covered: Vec::new(),
        unknowns: Vec::new(),
        skipped: Vec::new(),
    }
}

/// The file at `place` among the files.
#[must_use]
pub(crate) fn file_at(files: &[CorpusInput], place: FileOrder) -> Option<&CorpusInput> {
    files.get(place.0)
}

/// The bytes of the file at `place`; none for a place there is no file at.
#[must_use]
pub(crate) fn size_at(files: &[CorpusInput], place: FileOrder) -> FileSize {
    files.get(place.0).map_or(FileSize(0), |input| match input {
        CorpusInput::Available(entry) => entry.size,
        CorpusInput::ReadFailed { .. } | CorpusInput::Unreadable { .. } => FileSize(0),
    })
}

/// Decides which rules govern the file, once for it; a file no rule governs is neither read nor parsed.
pub(crate) fn govern(
    metadata: &impl RunDecisions,
    setting: &VisitSetting<'_>,
    tables: &mut Tables,
    path: &FilePath,
) -> Governing {
    metadata.place(setting.placed, path, &mut tables.mask, &mut tables.hits);
    metadata.governed(&tables.mask)
}

/// Whether the visit of the file at `place` reads the memory the process holds: the reading costs more than a file's visit when
/// the files are small, so it is made for one visit in `MEMORY_EVERY`.
#[must_use]
pub(crate) fn checking(place: FileOrder) -> Checking {
    if place.0.is_multiple_of(MEMORY_EVERY) {
        Checking::Now
    } else {
        Checking::Later
    }
}

/// Takes what one worker made into what the workers made together.
pub(crate) fn absorb(met: &mut WorkersMet, tables: Tables) {
    met.read.0 += tables.reads.0;
    met.observations.extend(tables.observations);
    met.covered.extend(tables.covered);
    met.unknowns.extend(tables.unknowns);
    met.skipped.extend(tables.skipped);
    met.unread.extend(tables.unread);
    met.unrecorded.extend(tables.unrecorded);
    met.arenas.push(tables.arena);
    met.fresh.extend(tables.fresh);
    met.notes.0.extend(tables.notes);
}

/// Puts the files that were left unread, and the records the workers made, in the order of the files.
#[must_use]
pub(crate) fn order(met: &mut WorkersMet) -> Fresh {
    met.unread.sort_by(|left, right| left.path.cmp(&right.path));
    let mut fresh = std::mem::take(&mut met.fresh);
    fresh.sort_by_key(|each| each.place.0);
    Fresh {
        places: fresh.iter().map(|each| each.place).collect(),
        records: fresh.into_iter().map(|each| each.record).collect(),
    }
}

/// Whether the rule governs the file the worker is visiting.
#[must_use]
pub(crate) fn governs_rule(
    metadata: &impl RunDecisions,
    tables: &Tables,
    rule: RuleIndex,
) -> Governing {
    metadata.governs(
        &tables.mask,
        RuleNumber(u32::try_from(rule.0).unwrap_or(u32::MAX)),
    )
}

/// What the languages wrote of each file, by the file's place: made by this run for the files it read, and nothing for the others.
#[must_use]
pub(crate) fn note_slots(fresh: &mut FreshNotes, files: FileTotal) -> Vec<Option<FileNotes>> {
    let mut slots: Vec<Option<FileNotes>> = vec![None; files.0];
    for PlacedNotes { place, notes } in std::mem::take(&mut fresh.0) {
        if let Some(slot) = slots.get_mut(place.0) {
            *slot = Some(notes);
        }
    }
    slots
}

/// The record kept of each of `files`, in their order; none for a file with no record.
pub(crate) fn records_of<'a>(files: &[CorpusInput], kept: &'a [Record]) -> Vec<Option<&'a Record>> {
    files
        .iter()
        .map(|file| {
            let CorpusInput::Available(entry) = file else {
                return None;
            };
            match kept.binary_search_by(|record| record.path.cmp(&entry.path)) {
                Ok(at) => kept.get(at),
                // The search answers where the path would go: the file has no record.
                Err(_) => None,
            }
        })
        .collect()
}

/// Whether the applicability of the rules may gate which of them run on a file: it may unless every plan has a policy and every
/// policy is to run always.
pub(crate) fn gating_of(
    applicability: &[ryo_values::domain::runs::types::RuleApplicability],
    plans: &[ryo_values::domain::plans::types::Plan],
) -> super::types::Gating {
    if applicability.len() != plans.len()
        || applicability
            .iter()
            .any(|policy| *policy != ryo_values::domain::runs::types::RuleApplicability::Always)
    {
        super::types::Gating::MayGate
    } else {
        super::types::Gating::NeverGates
    }
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
