//! The services of the `measuring` part: what a measuring worker counts (the runs of each plan point; each unit's uses,
//! evaluations and rows; each table's fill; the file's elements by kind) and times (each unit's own thread time, what it
//! called left out, and the part of it spent in files where the unit gave no row). Every function here does nothing in
//! a worker that does not measure, and none is called once per word but `word_run`, which the plain loop does not hold.
//! The clock is the thread's own, read when the unit running changes: on entering and on leaving a unit.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{CodeEntry, WordSlot};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::scratch::types::{LoopState, Measuring, UnitInFile};
use cpu_time::ThreadTime;
use ryo_language::domain::node_table::service::ids_of_kind;
use ryo_relations::domain::relations::types::{Count, KindBits};
use std::time::Duration;

/// The slot of a list grown to hold place `at`.
fn slot_mut<T: Clone + Default>(all: &mut Vec<T>, at: Count) -> Option<&mut T> {
    if all.len() <= at.0 {
        all.resize(at.0.saturating_add(1), T::default());
    }
    all.get_mut(at.0)
}

/// Adds `by` to a unit's count in a column of counts by unit place; a unit no code names counts nothing.
fn added(column: &mut Vec<Count>, unit: WordSlot, by: Count) {
    let place = Count(usize::try_from(unit.0).unwrap_or(usize::MAX));
    if unit != NO_SLOT
        && let Some(held) = slot_mut(column, place)
    {
        *held = Count(held.0.saturating_add(by.0));
    }
}

/// A unit's time and rows in the file being scanned.
fn in_file_mut(measure: &mut Measuring, unit: WordSlot) -> Option<&mut UnitInFile> {
    let place = usize::try_from(unit.0).unwrap_or(usize::MAX);
    (unit != NO_SLOT)
        .then(|| slot_mut(&mut measure.file, Count(place)))
        .flatten()
}

/// The thread's own time; nought where the system has no such clock.
fn thread_time() -> Duration {
    ThreadTime::try_now().map_or(Duration::ZERO, |now| now.as_duration())
}

/// Gives the time since the unit running last changed to the innermost unit running.
fn switched(measure: &mut Measuring) {
    let now = thread_time();
    let spent = usize::try_from(now.saturating_sub(measure.since).as_nanos()).unwrap_or(usize::MAX);
    measure.since = now;
    if let Some(unit) = measure.running.last().copied()
        && let Some(held) = in_file_mut(measure, unit)
    {
        held.nanos = Count(held.nanos.0.saturating_add(spent));
    }
}

/// A unit starts to run: the time so far is its caller's.
pub(crate) fn entered(state: &mut LoopState, unit: WordSlot) {
    if let Some(measure) = state.measure.as_deref_mut() {
        switched(measure);
        measure.running.push(unit);
    }
}

/// The innermost unit running ends: the time since it last ran is its own, and its caller runs again.
pub(crate) fn left(state: &mut LoopState) {
    if let Some(measure) = state.measure.as_deref_mut() {
        switched(measure);
        let _ended: Option<WordSlot> = measure.running.pop();
    }
}

/// A use reached a unit: a rule dispatched, a pattern asked, a table read.
pub(crate) fn used(state: &mut LoopState, unit: WordSlot) {
    if let Some(measure) = state.measure.as_deref_mut() {
        added(&mut measure.calls, unit, Count(1));
    }
}

/// A unit was evaluated for `keys` keys and gave `rows` rows: a rule for its candidates, a pattern for one key.
pub(crate) fn evaluated(state: &mut LoopState, unit: WordSlot, keys: Count, rows: Count) {
    let Some(measure) = state.measure.as_deref_mut() else {
        return;
    };
    added(&mut measure.keys, unit, keys);
    added(&mut measure.rows, unit, rows);
    if let Some(held) = in_file_mut(measure, unit) {
        held.rows = Count(held.rows.0.saturating_add(rows.0));
    }
}

/// The word at `at` is about to run in a measuring worker: its plan point has one run more.
#[inline]
pub(crate) fn word_run(frame: &mut StepFrame<'_, '_>, at: CodeEntry) {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    let point = frame.fill.programs.code.points.get(place).copied();
    if let Some(point) = point.filter(|point| *point != NO_SLOT)
        && let Some(measure) = frame.state.measure.as_deref_mut()
        && let Some(runs) = slot_mut(
            &mut measure.points,
            Count(usize::try_from(point.0).unwrap_or(usize::MAX)),
        )
    {
        *runs = Count(runs.0.saturating_add(1));
    }
}

/// The file's scan ends in a measuring worker. Every table made in the file is counted once, with its rows and its
/// fill's rows, whoever made it (a fill makes the tables it reads itself); the file's elements are counted by kind; and
/// each unit's time in the file is added to its time, and to its idle time when it gave no row in the file.
pub(crate) fn file_ended(frame: &mut StepFrame<'_, '_>) {
    let (store, code, nodes) = (
        &*frame.fill.store,
        frame.fill.programs.code,
        frame.fill.nodes,
    );
    let kinds = frame.fill.programs.kinds.len();
    let Some(measure) = frame.state.measure.as_deref_mut() else {
        return;
    };
    measure.files = Count(measure.files.0.saturating_add(1));
    let made = store.tables.iter().zip(&code.tables);
    for (slot, table) in made.filter(|(slot, _)| slot.stamp == store.stamp) {
        let build = slot.work.read.0.saturating_add(slot.work.given.0);
        added(&mut measure.keys, table.unit, Count(1));
        added(&mut measure.rows, table.unit, slot.work.made);
        added(&mut measure.build, table.unit, Count(build));
        if let Some(held) = in_file_mut(measure, table.unit) {
            held.rows = Count(held.rows.0.saturating_add(slot.work.made.0));
        }
    }
    for kind in 0..kinds {
        let ids = ids_of_kind(nodes, KindBits(u16::try_from(kind).unwrap_or(u16::MAX))).len();
        if let Some(held) = slot_mut(&mut measure.kinds, Count(kind)) {
            *held = Count(held.0.saturating_add(ids));
        }
    }
    let Measuring {
        nanos,
        idle_nanos,
        file,
        ..
    } = measure;
    for (place, spent) in file.iter_mut().enumerate() {
        let unit = WordSlot(u32::try_from(place).unwrap_or(u32::MAX));
        added(nanos, unit, spent.nanos);
        if spent.rows.0 == 0 {
            added(idle_nanos, unit, spent.nanos);
        }
        *spent = UnitInFile::default();
    }
}
