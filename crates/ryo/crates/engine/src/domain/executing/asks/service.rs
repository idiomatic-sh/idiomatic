//! The services of the `asks` part: a definition's own need read at an ask of one of its patterns in call form, so that
//! an ask in a file where the definition cannot have a row is not made; and what an `Exists` word that keeps no answer
//! of its own asks where it stands: an inline body, and a pattern in call form whose program is a short straight line,
//! which is run as an inline body is and keeps no answer (the budget counts its steps, and no key).

use super::types::QuickAsk;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeEntry, NeedSite};
use crate::domain::executing::measuring::service::used;
use crate::domain::executing::presence::service::consulted;
use crate::domain::executing::sources::types::FileNeeds;
use crate::domain::memos::types::RuleStamp;
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{operand, set};
use crate::domain::scratch::kept::types::{AskedNeed, QuickBody};
use crate::domain::scratch::types::BodyAnswer;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Whether the definition of a pattern in call form may have a row in the file: its own need, read from what it
/// answered this rule's first ask of it in the file: one load and one compare of the rule's stamp.
#[inline]
pub(crate) fn definition_needed(frame: &mut StepFrame<'_, '_>, need: NeedSite) -> NeedVerdict {
    let place = usize::try_from(need.0).unwrap_or(usize::MAX);
    match frame.state.need_asks.get(place) {
        Some(asked) if asked.rule == frame.state.budget.stamp => asked.verdict,
        Some(_) | None => first_asked(frame, need),
    }
}

/// A rule's first ask of a definition's need in a file: decided, or found decided by an earlier rule; the rule is told
/// the node table the decision read, whoever decided it; and what it said is kept for the rule's later asks.
#[inline(never)]
fn first_asked(frame: &mut StepFrame<'_, '_>, need: NeedSite) -> NeedVerdict {
    let place = usize::try_from(need.0).unwrap_or(usize::MAX);
    let code = frame.fill.programs.code;
    let verdict = decided(&mut FileNeeds(frame), code, need);
    consulted(frame, need);
    let asked = AskedNeed {
        rule: frame.state.budget.stamp,
        verdict,
    };
    let asks = &mut frame.state.need_asks;
    if asks.len() <= place && place < usize::MAX {
        asks.resize(
            place.saturating_add(1),
            AskedNeed {
                rule: RuleStamp::default(),
                verdict: NeedVerdict::Holds,
            },
        );
    }
    if let Some(kept) = asks.get_mut(place) {
        *kept = asked;
    }
    verdict
}

/// What the `Exists` word at `at` asks of its body where it stands: nothing, when the body has a row whatever the
/// candidate or its definition cannot have one in the file (the answer is left in the state); the program to run, for
/// an inline body and for a straight pattern, whose formals are written first; or that the body is asked as any is.
#[inline]
pub(crate) fn quick_exists(frame: &mut StepFrame<'_, '_>, at: CodeEntry) -> QuickAsk {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    match frame.state.decoded.quick.get(place).copied() {
        Some(QuickBody::Row) => {
            frame.state.answer = BodyAnswer {
                first: Count(0),
                rows: Count(1),
            };
            QuickAsk::Answered
        }
        Some(QuickBody::Inline(entry)) => QuickAsk::Run(entry),
        Some(QuickBody::Straight(entry)) => straight_asked(frame, at, entry),
        Some(QuickBody::Kept) | None => QuickAsk::Kept,
    }
}

/// An ask of a straight pattern: not made where its definition cannot have a row; else the caller's cells are written
/// to its formals and its program is to be run in place.
fn straight_asked(frame: &mut StepFrame<'_, '_>, at: CodeEntry, entry: CodeEntry) -> QuickAsk {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    let body = frame.state.decoded.bodies.get(place).copied().flatten();
    let Some(BodyRecord::Call {
        unit,
        need,
        bound,
        formals,
        ..
    }) = body.map(|read| read.record)
    else {
        return QuickAsk::Kept;
    };
    if definition_needed(frame, need) == NeedVerdict::Empty {
        frame.state.answer = BodyAnswer::default();
        return QuickAsk::Answered;
    }
    used(frame.state, unit);
    let first = frame
        .state
        .decoded
        .parts_at
        .get(usize::try_from(formals.0).unwrap_or(usize::MAX))
        .and_then(|held| held.0.checked_sub(1));
    for number in 0..usize::try_from(bound.0).unwrap_or(usize::MAX) {
        let part = first.and_then(|start| {
            frame
                .state
                .decoded
                .parts
                .get(start.saturating_add(number))
                .copied()
        });
        let Some(part) = part else {
            continue;
        };
        let value = match part.value {
            Some(held) => operand(frame, held),
            None => CellWord(0),
        };
        set(frame, part.formal, value);
    }
    QuickAsk::Run(entry)
}
