//! The services of the `kept` part: the answer of a pattern in call form for a key: looked for, or the pattern's program
//! run and its rows kept; or, for a site whose keys do not come again, run and not kept (SPIKE).

use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::executing::asks::service::{definition_needed, site_passes, site_passing};
use crate::domain::executing::asks::types::{AskedBy, SitePasses};
use crate::domain::executing::bodies::types::CallUse;
use crate::domain::executing::budget::service::{begin_unit, end_unit, reach};
use crate::domain::executing::constants::NO_ANSWER;
use crate::domain::executing::measuring::service::{entered, evaluated, left, used};
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::types::{KeyFrom, PatternAsk};
use crate::domain::memos::service::{asked, keep, keep_passing};
use crate::domain::memos::types::{ChargedUnit, MemoId};
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::operands::service::{operand, set};
use crate::domain::scratch::types::LoopState;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The answer of a pattern in call form for the key the loop's key room holds from `start` on: the kept one, or the
/// pattern's program run once for this key, its yielded rows kept. The key's cells arrive in the formals. The key is taken
/// off the room.
fn pattern_keyed(
    frame: &mut StepFrame<'_, '_>,
    body: CallUse,
    start: Count,
    by: AskedBy,
) -> MemoId {
    used(frame.state, body.unit);
    // SPIKE: a site whose keys do not come again for the rule in the file stops keeping its answers; only for an ask
    // of the rule's own program, whose answer nothing lists.
    // A fill's ask may pass at any depth: its work goes to its table's sum. A program's only at depth nought, where
    // its steps are the rule's own.
    let passes = if by == AskedBy::Fill
        || frame.state.budget.depth.0 == 0
        || crate::domain::walk::any_depth()
    {
        site_passing(frame.state, body.site)
    } else {
        SitePasses::Kept
    };
    let kept = if passes == SitePasses::Passing {
        None
    } else {
        let kept = asked(
            &frame.state.memos,
            body.site,
            frame.state.key.get(start.0..).unwrap_or_default(),
        );
        let _later: SitePasses = site_passes(frame.state, body.site, kept);
        kept
    };
    if let Some(answer) = kept {
        frame.state.key.truncate(start.0);
        // SPIKE: an answer a table's fill asks is charged with the table (its sum), to every rule alike.
        if by == AskedBy::Program {
            reach(frame, ChargedUnit::Answer(answer));
        }
        return answer;
    }
    let first = parts_start(frame.state, body);
    for number in 0..usize::try_from(body.bound.0).unwrap_or(usize::MAX) {
        let part = first.and_then(|at| frame.state.decoded.parts.get(at.0.saturating_add(number)));
        let Some(formal) = part.map(|held| held.formal) else {
            continue;
        };
        let value = match frame.state.key.get(start.0.saturating_add(number)) {
            Some(held) => *held,
            None => CellWord(0),
        };
        set(frame, formal, value);
    }
    let base = frame.state.yielded.len();
    if passes == SitePasses::Passing {
        // SPIKE: run as an inline body is: no unit is begun and no answer is kept. A program's steps are the rule's
        // own; a fill's are taken off again and left for its table's sum (`LoopState.passed`).
        let before = frame.state.budget.visited;
        let count = frame.ports.run(frame, body.entry, RunMode::Collect);
        if by == AskedBy::Fill {
            let work = frame.state.budget.visited.0.saturating_sub(before.0);
            frame.state.budget.visited = before;
            frame.state.passed = Some(InstructionCount(work));
        }
        let LoopState {
            memos,
            key,
            yielded,
            ..
        } = &mut *frame.state;
        let answer = keep_passing(
            memos,
            body.site,
            yielded.get(base..).unwrap_or_default(),
            Count(usize::try_from(count.0).unwrap_or(usize::MAX)),
        );
        yielded.truncate(base);
        key.truncate(start.0);
        return answer;
    }
    let caller = begin_unit(frame.state);
    entered(frame.state, body.unit);
    let count = frame.ports.run(frame, body.entry, RunMode::Collect);
    left(frame.state);
    let given = Count(usize::try_from(count.0).unwrap_or(usize::MAX));
    evaluated(frame.state, body.unit, Count(1), given);
    let LoopState {
        memos,
        key,
        yielded,
        ..
    } = &mut *frame.state;
    let answer = keep(
        memos,
        body.site,
        key.get(start.0..).unwrap_or_default(),
        yielded.get(base..).unwrap_or_default(),
        Count(usize::try_from(count.0).unwrap_or(usize::MAX)),
    );
    yielded.truncate(base);
    key.truncate(start.0);
    end_unit(frame.state, caller, answer, body.unit);
    if by == AskedBy::Program {
        reach(frame, ChargedUnit::Answer(answer));
    }
    answer
}

/// The answer of a pattern in call form for the caller's values, which are the operands paired with its formals; no
/// answer, and no ask, in a file where the pattern's definition cannot have a row.
pub(crate) fn pattern(frame: &mut StepFrame<'_, '_>, body: CallUse) -> MemoId {
    if definition_needed(frame, body.need) == NeedVerdict::Empty {
        return NO_ANSWER;
    }
    let start = frame.state.key.len();
    let first = parts_start(frame.state, body);
    let mut from = KeyFrom::Constants;
    for number in 0..usize::try_from(body.bound.0).unwrap_or(usize::MAX) {
        let part = first.and_then(|at| {
            frame
                .state
                .decoded
                .parts
                .get(at.0.saturating_add(number))
                .copied()
        });
        from = part.map_or(from, |held| held.from);
        let value = match part.and_then(|held| held.value) {
            Some(held) => operand(frame, held),
            None => CellWord(0),
        };
        frame.state.key.push(value);
    }
    // A key of constants alone names one answer a file: it is the site's, found again with no hash.
    let site = usize::try_from(body.site.0).unwrap_or(usize::MAX);
    if from == KeyFrom::Constants
        && let Some(answer) = frame.state.site_answers.get(site).copied().flatten()
    {
        frame.state.key.truncate(start);
        used(frame.state, body.unit);
        reach(frame, ChargedUnit::Answer(answer));
        return answer;
    }
    let answer = pattern_keyed(frame, body, Count(start), AskedBy::Program);
    if from == KeyFrom::Constants {
        let answers = &mut frame.state.site_answers;
        if answers.len() <= site {
            answers.resize(site.saturating_add(1), None);
        }
        if let Some(slot) = answers.get_mut(site) {
            *slot = Some(answer);
        }
    }
    answer
}

/// Where the parts of a call-form body's ask start in the decoded code; none for a body it does not hold.
fn parts_start(state: &LoopState, body: CallUse) -> Option<Count> {
    let place = usize::try_from(body.formals.0).unwrap_or(usize::MAX);
    let held = state.decoded.parts_at.get(place)?;
    held.0.checked_sub(1).map(Count)
}

/// The answer of a pattern in call form for a key the fill side hands over: what a `Use` stage asks back, once per distinct
/// key of its batch. The one memo makes the pattern's program run once per distinct key of the file.
pub(crate) fn asked_pattern(frame: &mut StepFrame<'_, '_>, asked_for: PatternAsk<'_>) -> MemoId {
    let record = body_at(&frame.fill.programs.code.pool, asked_for.body).map(|body| body.record);
    if let Some(BodyRecord::Call { need, .. }) = record
        && definition_needed(frame, need) == NeedVerdict::Empty
    {
        return NO_ANSWER;
    }
    let start = frame.state.key.len();
    frame.state.key.extend_from_slice(asked_for.key);
    match record {
        Some(BodyRecord::Call {
            entry,
            memo,
            unit,
            need,
            bound,
            formals,
        }) => pattern_keyed(
            frame,
            CallUse {
                entry,
                site: memo,
                unit,
                need,
                bound,
                formals,
            },
            Count(start),
            AskedBy::Fill,
        ),
        Some(BodyRecord::Inline { .. } | BodyRecord::Table { .. }) | None => {
            // A `Use` stage's body is a pattern in call form: anything else has no row, kept so under the asked site.
            let LoopState { memos, key, .. } = &mut *frame.state;
            let answer = keep(
                memos,
                asked_for.memo,
                key.get(start..).unwrap_or_default(),
                &[],
                Count(0),
            );
            key.truncate(start);
            answer
        }
    }
}
