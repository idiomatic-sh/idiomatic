//! The services of the `using` part: a `Use` stage of a pattern in call form asks the loop, through the callback, for the
//! pattern's rows: once for each distinct key of the batch. The batch is grouped by the key's columns with the one index;
//! each group's rows share the answer. No step program runs here.

use crate::domain::code::filling::service::stage_use_of;
use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeWord, IndexSite};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::indexes::reading::service::{read, rows_of, runs};
use crate::domain::indexes::service::{begin, clear_indexes, finish, key_column};
use crate::domain::indexes::types::{ColumnView, FileStamp, RowRun};
use crate::domain::made_tables::operands::service::{formal_value, value_of};
use crate::domain::made_tables::pipelines::types::{BatchLevel, PipelineRoom};
use crate::domain::made_tables::types::{FillFrame, PatternAsk, PatternFound, PatternRows};
use crate::domain::stages::types::{BatchValue, KeyPart, RowKept};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Asks the loop for the rows of a `Use` stage's pattern, once for each distinct key among the rows of the batch at
/// `depth`. After it the batch holds the answers row by row, for each of its rows the stretch of the answers its key
/// holds, and whether its key's pattern has a row.
///
/// # Errors
///
/// The word names no pattern in call form; or a source the pattern's program read could not be read.
pub(crate) fn use_asked(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    asker: MadeTableId,
    word: CodeWord,
    room: &mut PipelineRoom,
    depth: Count,
) -> Result<(), FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Word,
    };
    let stage = stage_use_of(word);
    let code = &frame.programs.code;
    let (bound, formals) = match body_at(&code.pool, stage.body).map(|body| body.record) {
        Some(BodyRecord::Call { bound, formals, .. }) => {
            (usize::try_from(bound.0).unwrap_or(usize::MAX), formals)
        }
        Some(BodyRecord::Inline { .. } | BodyRecord::Table { .. }) | None => return Err(wrong),
    };
    let PipelineRoom {
        levels,
        key,
        grouping,
        ..
    } = room;
    let BatchLevel {
        columns,
        rows,
        answers,
        spans,
        found,
        ..
    } = levels.get_mut(depth.0).ok_or(wrong)?;
    key.clear();
    key.resize(bound, CellWord(0));
    let mut parts: Vec<KeyPart<'_>> = Vec::new();
    for place in 0..bound {
        match value_of(
            columns,
            code,
            frame.file,
            formal_value(code, formals, Count(place))?,
        )? {
            BatchValue::Column(column) => parts.push(KeyPart {
                place: Count(place),
                column,
            }),
            BatchValue::Constant(cell) => {
                if let Some(slot) = key.get_mut(place) {
                    *slot = cell;
                }
            }
        }
    }
    clear_indexes(grouping);
    begin(grouping, *rows, Count(parts.len()));
    for (at, part) in parts.iter().enumerate() {
        key_column(
            grouping,
            Count(at),
            ColumnView {
                cells: part.column,
                first: Count(0),
                stride: Count(1),
            },
        );
    }
    finish(grouping, IndexSite(0), FileStamp(0), None);
    let index = read(grouping, IndexSite(0));
    let width = usize::from(stage.outputs.0);
    answers.clear();
    spans.clear();
    spans.resize(rows.0, RowRun::default());
    found.clear();
    found.resize(rows.0, RowKept::No);
    for run in runs(index) {
        let held = rows_of(index, run);
        let Some(first) = held.first() else {
            continue;
        };
        let first = usize::try_from(first.0).unwrap_or(usize::MAX);
        for part in &parts {
            if let (Some(slot), Some(cell)) = (key.get_mut(part.place.0), part.column.get(first)) {
                *slot = *cell;
            }
        }
        let before = answers.len();
        let answer = uses.pattern_rows(
            frame,
            PatternAsk {
                asker,
                body: stage.body,
                memo: stage.memo,
                key,
            },
            answers,
        )?;
        let span = RowRun {
            start: Count(before.checked_div(width).unwrap_or(0)),
            end: Count(answers.len().checked_div(width).unwrap_or(0)),
        };
        let mark = match answer {
            PatternFound::Rows => RowKept::Yes,
            PatternFound::Nothing => RowKept::No,
        };
        for each in held {
            let row = usize::try_from(each.0).unwrap_or(usize::MAX);
            if let Some(slot) = spans.get_mut(row) {
                *slot = span;
            }
            if let Some(slot) = found.get_mut(row) {
                *slot = mark;
            }
        }
    }
    Ok(())
}
