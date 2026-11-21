//! The services of the `meanings` part: a `Call` word asks a per-row meaning for the rows it gives the candidate. The
//! meaning is the fill side's kernel (`stages::calling::service::call_batch`, the operators crate's meanings), asked for
//! a batch of one row whose arguments are one cell each: a call word of the loop and a call stage of a pipeline have
//! one definition of every meaning. The rows given are staged, each as the cells of the word's outputs in order, for the
//! word's item. A meaning's fault on the row, and a meaning the kernel does not run, are handed on as a fill's are.

use crate::domain::code::deciding::service::call_of;
use crate::domain::code::records::service::operand_at;
use crate::domain::code::small_codes::service::meaning_of;
use crate::domain::code::types::{CodeWord, PoolAt};
use crate::domain::executing::sources::service::faulted;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::operand;
use crate::domain::scratch::types::{BodyAnswer, CallRoom, LoopState};
use crate::domain::stages::calling::service::call_batch;
use crate::domain::stages::types::{BatchValue, CallMeans, CallOut};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Stages the rows a `Call` word's meaning gives for the candidate, and leaves where they are in the state for the
/// word's item: no row when the meaning drops the candidate, faults on it, or is one the kernel does not run.
pub(crate) fn call(frame: &mut StepFrame<'_, '_>, word: CodeWord) {
    let asking = call_of(word);
    let code = frame.fill.programs.code;
    let first = frame.state.staged.len();
    frame.state.answer = BodyAnswer {
        first: Count(first),
        rows: Count(0),
    };
    let Some(meaning) = meaning_of(asking.meaning, asking.regex, asking.flag) else {
        faulted(
            frame.state,
            FilledError::Code {
                named: MissingCode::Word,
            },
        );
        return;
    };
    // The arguments' record is how many, then that many operands.
    let count = code
        .pool
        .get(usize::try_from(asking.arguments.0).unwrap_or(usize::MAX))
        .map_or(0, |held| held.0);
    frame.state.calls.arguments.clear();
    for number in 0..count {
        let at = PoolAt(
            asking
                .arguments
                .0
                .wrapping_add(1)
                .wrapping_add(number.wrapping_mul(2)),
        );
        let value = operand_at(&code.pool, at).map_or(CellWord(0), |held| operand(frame, held));
        frame
            .state
            .calls
            .arguments
            .push(BatchValue::Constant(value));
    }
    let LoopState {
        patterns,
        calls,
        staged,
        answer,
        ..
    } = &mut *frame.state;
    let CallRoom {
        arguments,
        answers,
        spans,
        scratch,
    } = calls;
    let asked = call_batch(
        CallMeans {
            matcher: frame.fill.matcher,
            regexes: frame.fill.programs.regexes,
            compiled: patterns,
            source: frame.fill.text,
        },
        &mut *frame.fill.words,
        meaning,
        arguments,
        Count(1),
        CallOut {
            answers,
            spans,
            scratch,
        },
    );
    match asked {
        Ok(()) => {
            let given = spans.first().copied().unwrap_or_default();
            staged.extend_from_slice(answers);
            answer.rows = Count(given.end.0.saturating_sub(given.start.0));
        }
        Err(fault) => faulted(frame.state, fault),
    }
}
