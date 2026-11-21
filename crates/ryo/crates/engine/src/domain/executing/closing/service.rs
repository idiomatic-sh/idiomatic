//! The services of the `closing` part: every element reached over one or more edges from a seed, each once (the seed
//! too, when an edge gives it), staged for the `Closure` word's item. The edges of an element are the rows of the word's body, asked with the
//! element in the record's register: an inline body run through the one loop, a pattern in call form through the one
//! memo, a pattern in table form through the seam, as an `Or`'s bodies are. Each element is asked for its edges once, so
//! a cycle ends; each edge given is a row visited.

use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeEntry, CodeWord, WordSlot};
use crate::domain::code::walking::service::closure_of;
use crate::domain::executing::bodies::service::{body_of, slot};
use crate::domain::executing::staging::service::body_staged;
use crate::domain::findings::types::InstructionCount;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, set};
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Keeps, of the cells staged from `fresh` on, each element not met before and not `never` (a nearest walk's start; a
/// closure names none), in place and in order;
/// answers how many cells were staged there.
pub(crate) fn kept_new(
    state: &mut LoopState,
    seen: &mut hashbrown::HashSet<CellWord>,
    never: Option<CellWord>,
    fresh: Count,
) -> Count {
    let staged = &mut state.staged;
    let given = staged.len().saturating_sub(fresh.0);
    let mut kept = fresh.0;
    for place in fresh.0..staged.len() {
        let Some(element) = staged.get(place).copied() else {
            break;
        };
        // SPIKE (ryo-fmx3.1, fault B): the seed is an element like any other when an edge gives it (an edge of an
        // element to itself, a cycle back): today's closure has the row, and `lookup-read-events` reads it.
        if Some(element) != never
            && seen.insert(element)
            && let Some(slot) = staged.get_mut(kept)
        {
            *slot = element;
            kept = kept.saturating_add(1);
        }
    }
    staged.truncate(kept);
    Count(given)
}

/// Stages what a `Closure` word reaches from its seed, in the order the elements are first met, and leaves where they
/// are in the state for the word's item. A word whose record the pool does not hold reaches nothing.
pub(crate) fn closure(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) {
    let asking = closure_of(word);
    let code = frame.fill.programs.code;
    let first = frame.state.staged.len();
    frame.state.answer = BodyAnswer {
        first: Count(first),
        rows: Count(0),
    };
    let Some(edges) = body_of(frame, at) else {
        return;
    };
    let from = Register(slot(frame, edges.after, WordSlot(0)).0);
    // A table-form body gives its first collected column: the register its record pairs with that column.
    let outputs = match edges.record {
        BodyRecord::Table { reads, .. } => {
            let register = usize::try_from(reads.0)
                .unwrap_or(usize::MAX)
                .saturating_add(1);
            code.pool
                .get(register..register.saturating_add(1))
                .unwrap_or_default()
        }
        BodyRecord::Inline { .. } | BodyRecord::Call { .. } => &[],
    };
    let seed = cell(frame, asking.seed);
    // The elements met are this closure's own: a closure inside the edges' body finds the worker's set empty.
    let mut seen = std::mem::take(&mut frame.state.seen);
    seen.clear();
    let (mut element, mut asked, mut given) = (seed, 0_usize, 0_u64);
    loop {
        set(frame, from, element);
        let fresh = Count(frame.state.staged.len());
        let _rows: Count = body_staged(frame, edges.record, outputs);
        let edges_given = kept_new(frame.state, &mut seen, None, fresh);
        given = given.saturating_add(u64::try_from(edges_given.0).unwrap_or(u64::MAX));
        let Some(next) = frame.state.staged.get(first.saturating_add(asked)).copied() else {
            break;
        };
        element = next;
        asked = asked.saturating_add(1);
    }
    frame.state.seen = seen;
    let LoopState {
        work,
        budget,
        staged,
        answer,
        ..
    } = &mut *frame.state;
    work.visited = InstructionCount(work.visited.0.saturating_add(given));
    budget.visited = InstructionCount(budget.visited.0.saturating_add(given));
    answer.rows = Count(staged.len().saturating_sub(first));
}
