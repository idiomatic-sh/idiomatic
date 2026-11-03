//! The `workers` service: free functions named by verb over what each worker owns and what the workers make together, with no IO
//! and no ports.

use super::constants::MEMORY_EVERY;
use super::types::{
    Checking, FileOrder, FileTotal, FilesRead, Fresh, FreshNotes, PlacedNotes, Tables,
    VisitSetting, WorkersMet,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::finish::types::{LineStarts, RuleNumber};
use crate::domain::lowering::types::Counting;
use crate::domain::operator::types::{PlanIdentity, PreparedPool, RowsFed, TreeSteps};
use crate::domain::placement::service::{governed, governs, place};
use crate::domain::placement::types::{Governing, Mask};
use crate::domain::render::types::Rendered;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::files::types::{CorpusEntry, FileBytes, FileSize, Hits};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{ArenaBytes, FileNotes};

/// What a worker owns before it reads a file: empty tables, and a pool big enough for any of the plans.
#[must_use]
pub(crate) fn tables_for(
    number: WorkerNumber,
    plans: &[Plan],
    unlowered: TreeSteps,
    counting: Counting,
) -> Tables {
    Tables {
        number,
        elements: Elements::default(),
        symbols: Symbols::default(),
        bytes: FileBytes::default(),
        language: None,
        mask: Mask::default(),
        hits: Hits::default(),
        words: Words::default(),
        packed: PackedWords::default(),
        pool: PreparedPool {
            matchers: std::sync::Arc::new(
                crate::domain::matching::types::prepared::value::PreparedMatchers::new(plans),
            ),
            layouts: Vec::with_capacity(plans.len()),
            fed: Vec::with_capacity(plans.len()),
            unlowered,
            counting,
            ..PreparedPool::default()
        },
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
        shared: crate::domain::shared_walk::types::MergedWalk::default(),
    }
}

/// The file at `place` among the files.
#[must_use]
pub(crate) fn file_at(files: &[CorpusEntry], place: FileOrder) -> Option<&CorpusEntry> {
    files.get(place.0)
}

/// The bytes of the file at `place`; none for a place there is no file at.
#[must_use]
pub(crate) fn size_at(files: &[CorpusEntry], place: FileOrder) -> FileSize {
    files.get(place.0).map_or(FileSize(0), |entry| entry.size)
}

/// Decides which rules govern the file, once for it; a file no rule governs is neither read nor parsed.
pub(crate) fn govern(
    setting: &VisitSetting<'_>,
    tables: &mut Tables,
    path: &FilePath,
) -> Governing {
    place(setting.placed, path, &mut tables.mask, &mut tables.hits);
    governed(&tables.mask)
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

/// Adds the rows `more` fed a step to `total`.
pub(crate) fn add_fed(total: &mut RowsFed, more: &RowsFed) {
    for (slot, each) in total.0.iter_mut().zip(more.0) {
        *slot = slot.saturating_add(each);
    }
}

/// Nothing met yet, with room for the rows each step of each plan is fed.
#[must_use]
pub(crate) fn gathering(plans: &[Plan]) -> WorkersMet {
    WorkersMet {
        fed: plans
            .iter()
            .map(|plan| vec![RowsFed::default(); plan.steps.len()])
            .collect(),
        ..WorkersMet::default()
    }
}

/// Takes what one worker made into what the workers made together.
pub(crate) fn absorb(met: &mut WorkersMet, tables: Tables, plans: &[Plan]) {
    for (layout, fed) in tables.pool.layouts.iter().zip(&tables.pool.fed) {
        let Some(rule) = plans
            .iter()
            .position(|plan| PlanIdentity(std::ptr::from_ref(plan).addr()) == layout.plan)
        else {
            continue;
        };
        if let Some(slots) = met.fed.get_mut(rule) {
            for (slot, more) in slots.iter_mut().zip(fed) {
                add_fed(slot, more);
            }
        }
    }
    met.read.0 += tables.reads.0;
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
pub(crate) fn governs_rule(tables: &Tables, rule: RuleIndex) -> Governing {
    governs(
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
