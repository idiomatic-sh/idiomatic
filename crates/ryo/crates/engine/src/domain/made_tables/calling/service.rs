//! The services of the `calling` part: a `Call` stage of a pipeline. Its word is read through the code's readers, each
//! argument is a column of the batch or one cell, and the meaning's loop in `stages::calling` is asked once for the
//! batch; the rows it gives lie on the batch as the rows of a `Use` do, and the same loop spreads them.

use crate::domain::code::filling::service::stage_call_of;
use crate::domain::code::small_codes::service::meaning_of;
use crate::domain::code::types::CodeWord;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::made_tables::operands::service::{call_arguments, operand, value_of};
use crate::domain::made_tables::pipelines::types::{BatchLevel, PipelineRoom};
use crate::domain::made_tables::types::FillFrame;
use crate::domain::stages::calling::service::call_batch;
use crate::domain::stages::types::{CallMeans, CallOut};
use ryo_relations::domain::relations::types::Count;

/// Asks a `Call` stage's meaning for every row of the batch at `depth`. After it the batch holds the cells of the rows
/// the meaning gave, row by row, and for each of its rows the stretch of those rows it gave.
///
/// # Errors
///
/// The word names no meaning, or an operand the code does not hold; the meaning faulted on a row; or it is one the fill
/// side does not run yet.
pub(crate) fn call_asked(
    frame: &mut FillFrame<'_>,
    word: CodeWord,
    room: &mut PipelineRoom,
    depth: Count,
) -> Result<(), FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Word,
    };
    let stage = stage_call_of(word);
    let code = frame.programs.code;
    let meaning = meaning_of(stage.meaning, stage.regex, stage.flag).ok_or(wrong.clone())?;
    let run = call_arguments(code, stage.arguments)?;
    let PipelineRoom {
        levels,
        compiled,
        calls,
        ..
    } = room;
    let BatchLevel {
        columns,
        rows,
        answers,
        spans,
        ..
    } = levels.get_mut(depth.0).ok_or(wrong)?;
    let mut arguments = Vec::with_capacity(run.count.0);
    for place in 0..run.count.0 {
        arguments.push(value_of(
            columns,
            code,
            frame.file,
            operand(code, run.first, Count(place))?,
        )?);
    }
    call_batch(
        CallMeans {
            matcher: frame.matcher,
            regexes: frame.programs.regexes,
            compiled,
            source: frame.text,
        },
        frame.words,
        meaning,
        &arguments,
        *rows,
        CallOut {
            answers,
            spans,
            scratch: calls,
        },
    )
}
