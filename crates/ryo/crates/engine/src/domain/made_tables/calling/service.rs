//! The services of the `calling` part: a `Call` stage of a pipeline. Its word is read through the code's readers, each
//! argument is a column of the batch or one cell, and the meaning's loop in `stages::calling` is asked once for the
//! batch; the rows it gives lie on the batch as the rows of a `Use` do, and the same loop spreads them.

use crate::domain::code::filling::service::{stage_call_of, stage_render_of};
use crate::domain::code::small_codes::service::{context_of, meaning_of};
use crate::domain::code::types::CodeWord;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::made_tables::operands::service::{
    call_arguments, column_mut, column_of, operand, value_of,
};
use crate::domain::made_tables::pipelines::types::{BatchLevel, PipelineRoom};
use crate::domain::made_tables::types::FillFrame;
use crate::domain::stages::calling::service::{call_batch, rendered};
use crate::domain::stages::types::{BatchRow, CallMeans, CallOut};
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

/// Asks a `Render` stage's rendering for every row of the batch at `depth`: the rows whose node has a text in the role are
/// marked kept and the texts, in their order, are the column of the stage's output, as long as the rows kept.
///
/// # Errors
///
/// The word names no role the code holds, or the language cannot prove a node's text in the role.
pub(crate) fn render_asked(
    frame: &mut FillFrame<'_>,
    word: CodeWord,
    room: &mut PipelineRoom,
    depth: Count,
) -> Result<(), FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Word,
    };
    let stage = stage_render_of(word);
    let context = context_of(stage.context).ok_or(wrong.clone())?;
    let BatchLevel {
        columns,
        rows,
        keep,
        answers,
        ..
    } = room.levels.get_mut(depth.0).ok_or(wrong.clone())?;
    keep.clear();
    answers.clear();
    for row in 0..rows.0 {
        let node = column_of(columns, stage.node)
            .get(row)
            .copied()
            .ok_or(wrong.clone())?;
        if let Some(text) = rendered(&mut *frame.facts, &mut *frame.words, node, context)? {
            keep.push(BatchRow(u32::try_from(row).unwrap_or(u32::MAX)));
            answers.push(text);
        }
    }
    let out = column_mut(columns, stage.out).ok_or(wrong)?;
    out.clear();
    out.extend_from_slice(answers);
    Ok(())
}
