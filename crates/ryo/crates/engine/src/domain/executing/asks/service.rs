//! The services of the `asks` part: a definition's own need read at an ask of one of its patterns in call form, so that
//! an ask in a file where the definition cannot have a row is not made; and what an `Exists` word that keeps no answer
//! of its own asks where it stands: an inline body, and a pattern in call form whose program is a short straight line,
//! which is run as an inline body is and keeps no answer (the budget counts its steps, and no key).

use super::types::QuickAsk;
use super::types::SitePasses;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeEntry, MemoSite, NeedSite};
use crate::domain::executing::constants::{PASS_AFTER, PASS_UNDER};
use crate::domain::executing::measuring::service::used;
use crate::domain::executing::presence::service::consulted;
use crate::domain::executing::sources::types::FileNeeds;
use crate::domain::memos::types::{MemoId, RuleStamp};
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{operand, set};
use crate::domain::scratch::kept::types::{AskedNeed, QuickBody, SiteAsks};
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Whether the definition of a pattern in call form may have a row in the file: its own need, read from what it
/// answered this rule's first ask of it in the file: one load and one compare of the rule's stamp.
#[inline]
pub(crate) fn definition_needed(frame: &mut StepFrame<'_, '_>, need: NeedSite) -> NeedVerdict {
    // Inside a table's fill nothing is decided and nothing skipped: the fill may be a need's own way of deciding, and a
    // formula that reads a table still being made would find it empty (ERA001, SIM300 and S307 lost their rows so).
    if frame.state.filling.0 > 0 {
        return NeedVerdict::Holds;
    }
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
    // Every operand is read before any formal is written, as an evaluation the memo keeps reads its whole key first: an
    // operand may be one of the pattern's own formal registers (a pattern asked from inside itself, or from a program
    // that shares its registers), and writing a formal before reading it gave the ask the wrong cells (ERA001, SIM300
    // and S307 lost their rows so).
    let count = usize::try_from(bound.0).unwrap_or(usize::MAX);
    let part_at = |frame: &StepFrame<'_, '_>, number: usize| {
        first.and_then(|start| {
            frame
                .state
                .decoded
                .parts
                .get(start.saturating_add(number))
                .copied()
        })
    };
    let start = frame.state.key.len();
    for number in 0..count {
        let value = match part_at(frame, number).and_then(|part| part.value) {
            Some(held) => operand(frame, held),
            None => CellWord(0),
        };
        frame.state.key.push(value);
    }
    for number in 0..count {
        let Some(part) = part_at(frame, number) else {
            continue;
        };
        let value = match frame.state.key.get(start.saturating_add(number)) {
            Some(held) => *held,
            None => CellWord(0),
        };
        set(frame, part.formal, value);
    }
    frame.state.key.truncate(start);
    QuickAsk::Collect(entry)
}

/// SPIKE: counts an ask the running rule makes of a memo site in the file, found kept or not, and answers whether the
/// site passes from now on: after `PASS_AFTER` asks of which fewer than one in `PASS_UNDER` found a kept answer, the
/// rule's later asks of the site in the file are evaluated without looking for a kept answer and without keeping one
/// (the memo's table costs more than it saves when keys do not come again). Each rule decides from its own asks, so
/// the order of the rules changes nothing.
pub(crate) fn site_passes(
    state: &mut LoopState,
    site: MemoSite,
    found: Option<MemoId>,
) -> SitePasses {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    let stamp = state.budget.stamp;
    if state.site_asks.len() <= place && place < usize::MAX {
        state
            .site_asks
            .resize(place.saturating_add(1), SiteAsks::default());
    }
    let Some(asks) = state.site_asks.get_mut(place) else {
        return SitePasses::Kept;
    };
    if asks.rule != stamp {
        *asks = SiteAsks {
            rule: stamp,
            ..SiteAsks::default()
        };
    }
    asks.asks = Count(asks.asks.0.saturating_add(1));
    asks.hits = Count(asks.hits.0.saturating_add(usize::from(found.is_some())));
    passing(*asks)
}

/// Whether a site with these asks passes.
#[inline]
fn passing(asks: SiteAsks) -> SitePasses {
    if asks.asks.0 >= PASS_AFTER.0 && asks.hits.0.saturating_mul(PASS_UNDER.0) < asks.asks.0 {
        SitePasses::Passing
    } else {
        SitePasses::Kept
    }
}

/// SPIKE: whether the running rule's asks of a memo site already pass in the file, before an ask is made.
#[inline]
pub(crate) fn site_passing(state: &LoopState, site: MemoSite) -> SitePasses {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    match state.site_asks.get(place) {
        Some(asks) if asks.rule == state.budget.stamp => passing(*asks),
        Some(_) | None => SitePasses::Kept,
    }
}
