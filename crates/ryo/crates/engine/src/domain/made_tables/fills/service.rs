//! The services of the `fills` part: a table's fill read from the code, and the fills that are not a pipeline's write: a
//! counted grouping, a sorted table, a group's ends and a group's places over their gathered input, and a constant table's
//! rows.
//!
//! The record is read here and nowhere else on the fill side. Its layout is the encoding's (`encoding/tables`): the width;
//! the list of the table's index sites; one parameter; then for each input the list of its pipelines' entries, the list of
//! its key columns and its order column or none; a constant table's rows as constants, row by row. Which fill a code is
//! is asked of `lowered::service::fill_code`, the one statement of the codes.

use super::types::{FillInputs, FillKind, TableFill};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::filling::service::stage_write_of;
use crate::domain::code::records::service::edge_columns_at;
use crate::domain::code::records::types::{EdgeColumns, PooledOperand, VisitColumns};
use crate::domain::code::service::list;
use crate::domain::code::types::{CodeEntry, FillCode, PoolAt, ProgramCode, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{MissingCode, UnbuiltFill};
use crate::domain::lowered::service::fill_code;
use crate::domain::lowered::types::{
    ConstantRows, FillInput, GraphFill, GroupFill, MadeFill, OrderedFill,
};
use crate::domain::made_tables::columns::types::ColumnIs;
use crate::domain::made_tables::following::service::follow_groups;
use crate::domain::made_tables::operands::service::{constant_at, file_cell, operand};
use crate::domain::made_tables::placing::service::place_groups;
use crate::domain::made_tables::reading::service::pipeline_words;
use crate::domain::made_tables::service::{
    count_groups, end_groups, file_columns, set_keys, sort_groups,
};
use crate::domain::made_tables::types::{FillFrame, FillWork, GroupRoom, TableRows};
use crate::domain::steps::types::{ColumnList, MadeColumn, MadeTableId};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};

/// The pool does not hold the record a table names.
const NO_RECORD: FilledError = FilledError::Code {
    named: MissingCode::Pool,
};

/// The codes of the fills the fill side runs with the fill each is: asked of the lowering's own statement of the codes
/// once a process, since a table's fill is asked for at every read of it.
fn known_fills() -> &'static [(FillCode, FillKind); 9] {
    static KNOWN: std::sync::OnceLock<[(FillCode, FillKind); 9]> = std::sync::OnceLock::new();
    KNOWN.get_or_init(|| {
        let input = || FillInput {
            pipelines: Box::new([]),
            keys: ColumnList(Box::new([])),
            order: None,
        };
        let no_column = EdgeColumns {
            source: MadeColumn(0),
            destination: MadeColumn(0),
        };
        [
            (
                fill_code(&MadeFill::Rows(Box::new([]), RepeatedRows::Kept)),
                FillKind::Rows,
            ),
            (
                fill_code(&MadeFill::Group(
                    GroupFill::Count { distinct: None },
                    input(),
                )),
                FillKind::Count,
            ),
            (
                fill_code(&MadeFill::Ordered(OrderedFill::Sorted(input()))),
                FillKind::Sorted,
            ),
            (
                fill_code(&MadeFill::Group(GroupFill::Ends(GroupEnd::Least), input())),
                FillKind::Ends,
            ),
            (
                fill_code(&MadeFill::Group(GroupFill::Places, input())),
                FillKind::Places,
            ),
            (
                fill_code(&MadeFill::Graph(
                    GraphFill::FixedPoints(no_column),
                    Box::new([]),
                )),
                FillKind::Follow,
            ),
            (
                fill_code(&MadeFill::Graph(
                    GraphFill::OrderedDfs(VisitColumns {
                        root: MadeColumn(0),
                        source: MadeColumn(0),
                        destination: MadeColumn(0),
                        edge: MadeColumn(0),
                    }),
                    Box::new([]),
                )),
                FillKind::Visits,
            ),
            (
                fill_code(&MadeFill::Ordered(OrderedFill::Match(input(), input()))),
                FillKind::Match,
            ),
            (
                fill_code(&MadeFill::Constant(ConstantRows(Vec::new()))),
                FillKind::Constant,
            ),
        ]
    })
}

/// Which fill a code is, of the fills the fill side runs: the code is compared with the lowering's own code of each.
pub(crate) fn kind_of(code: FillCode) -> Result<FillKind, FilledError> {
    known_fills()
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, kind)| *kind)
        .ok_or(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Fill(code),
        })
}

/// The slot of the pool at a place.
fn slot(code: &ProgramCode, at: PoolAt) -> Result<WordSlot, FilledError> {
    code.pool
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(NO_RECORD)
}

/// The place `ahead` slots after a place of the pool.
fn after(at: PoolAt, ahead: Count) -> Result<PoolAt, FilledError> {
    match u32::try_from(ahead.0) {
        Ok(slots) => at.0.checked_add(slots).map(PoolAt).ok_or(NO_RECORD),
        Err(_) => Err(NO_RECORD),
    }
}

/// The code of a made table's fill, as written: what `built::service::runs` is asked of a fill.
///
/// # Errors
///
/// The code holds no such table.
pub(crate) fn fill_code_at(
    code: &ProgramCode,
    table: MadeTableId,
) -> Result<FillCode, FilledError> {
    code.tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .map(|written| FillCode(written.fill.0))
        .ok_or(FilledError::Code {
            named: MissingCode::Table,
        })
}

/// The fill of a made table, as the code holds it: which fill, the table's width, its one parameter, and where its inputs
/// or its constant rows start.
///
/// # Errors
///
/// The code holds no such table or no record for it, or the fill is one the fill side does not run yet.
pub(crate) fn fill_at(code: &ProgramCode, table: MadeTableId) -> Result<TableFill, FilledError> {
    let written = code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .ok_or(FilledError::Code {
            named: MissingCode::Table,
        })?;
    let kind = kind_of(FillCode(written.fill.0))?;
    let at = PoolAt(written.operands.0);
    let width = slot(code, at)?;
    let sites = list(&code.pool, WordSlot(after(at, Count(1))?.0));
    let parameter = after(at, Count(sites.len().saturating_add(2)))?;
    Ok(TableFill {
        kind,
        width: Count(usize::try_from(width.0).unwrap_or(usize::MAX)),
        parameter: slot(code, parameter)?,
        inputs: after(parameter, Count(1))?,
    })
}

/// One input of a fill, read at its place in the record.
///
/// # Errors
///
/// The pool does not hold it.
pub(crate) fn input_at(code: &ProgramCode, at: PoolAt) -> Result<FillInputs<'_>, FilledError> {
    let entries = list(&code.pool, WordSlot(at.0));
    let keys_at = after(at, Count(entries.len().saturating_add(1)))?;
    let keys = list(&code.pool, WordSlot(keys_at.0));
    let order_at = after(keys_at, Count(keys.len().saturating_add(1)))?;
    let order = slot(code, order_at)?;
    Ok(FillInputs {
        entries,
        keys,
        order: (order != NO_SLOT).then_some(Count(usize::try_from(order.0).unwrap_or(usize::MAX))),
        after: after(order_at, Count(1))?,
    })
}

/// The `Constant` fill: the table's rows are the record's constants, row by row.
///
/// # Errors
///
/// The record names a constant the code does not hold.
pub(crate) fn constant_rows(
    code: &ProgramCode,
    fill: TableFill,
    out: &mut TableRows,
) -> Result<(), FilledError> {
    let rows = usize::try_from(fill.parameter.0).unwrap_or(usize::MAX);
    for place in 0..rows.saturating_mul(fill.width.0) {
        out.cells.push(constant_at(
            code,
            slot(code, after(fill.inputs, Count(place))?)?,
        )?);
    }
    out.count = Count(rows);
    Ok(())
}

/// The fills that group their gathered input: `Group(Count)` writes one row for each key, `Ordered(Sorted)` the input's
/// rows with each key's together and ascending by the order column, `Group(Ends)` the rows at one end of each key's,
/// `Group(Places)` each distinct row with its place in its key's order. The rows read and given are added to the work.
///
/// # Errors
///
/// A key or the order names no column of the input: a fault of the code, never of a file.
pub(crate) fn grouped_rows(
    code: &ProgramCode,
    fill: TableFill,
    gathered: &TableRows,
    read: FillInputs<'_>,
    room: &mut GroupRoom,
    out: &mut TableRows,
    work: &mut FillWork,
) -> Result<(), FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Column,
    };
    set_keys(room, read.keys.iter().map(|column| ColumnIndex(column.0)));
    match fill.kind {
        FillKind::Count => {
            let distinct = (fill.parameter != NO_SLOT).then_some(Count(
                usize::try_from(fill.parameter.0).unwrap_or(usize::MAX),
            ));
            count_groups(gathered, distinct, room, out).map_err(|_| wrong)?;
        }
        FillKind::Sorted => sort_groups(gathered, read.order.ok_or(wrong)?, room, out),
        FillKind::Ends => {
            // The record's parameter is the end: 0 the least, 1 the greatest.
            let end = if fill.parameter == WordSlot(0) {
                GroupEnd::Least
            } else {
                GroupEnd::Greatest
            };
            end_groups(gathered, read.order.ok_or(wrong)?, end, room, out);
        }
        FillKind::Places => place_groups(gathered, read.order.ok_or(wrong)?, room, out),
        FillKind::Follow => {
            // The record's parameter is the place of the told columns: where an edge leaves from and arrives.
            let columns = edge_columns_at(&code.pool, PoolAt(fill.parameter.0)).ok_or(wrong)?;
            let visits = follow_groups(gathered, columns, room, out);
            work.read = Count(work.read.0.saturating_add(visits.0));
        }
        FillKind::Rows | FillKind::Match | FillKind::Visits | FillKind::Constant => {
            return Err(wrong);
        }
    }
    work.read = Count(work.read.0.saturating_add(gathered.count.0));
    work.given = Count(work.given.0.saturating_add(out.count.0));
    Ok(())
}

/// Lays the columns of the emptied table an input's pipelines write into: a column every one of them writes the file
/// into is the file's own, and the table keeps no cell of it. `writes` is room for where each pipeline's row lies.
///
/// # Errors
///
/// A pipeline of the input has no write the code holds.
pub(crate) fn file_laid(
    code: &ProgramCode,
    file: CellWord,
    input: FillInputs<'_>,
    writes: &mut Vec<PoolAt>,
    into: &mut TableRows,
) -> Result<(), FilledError> {
    writes.clear();
    for entry in input.entries {
        let words = pipeline_words(code, CodeEntry(entry.0))?;
        let write = words.last().ok_or(NO_RECORD)?;
        writes.push(stage_write_of(*write).operands);
    }
    if !writes.is_empty() {
        file_columns(into, file, |column| {
            let every = writes
                .iter()
                .all(|at| matches!(operand(code, *at, column), Ok(PooledOperand::File)));
            if every {
                ColumnIs::TheFile
            } else {
                ColumnIs::Written
            }
        });
    }
    Ok(())
}

/// How many cells a row of an input's pipelines has: what its first pipeline's write writes.
///
/// # Errors
///
/// The input's first pipeline has no write the code holds.
pub(crate) fn input_width(code: &ProgramCode, input: FillInputs<'_>) -> Result<Count, FilledError> {
    let Some(entry) = input.entries.first() else {
        return Ok(Count(0));
    };
    let words = pipeline_words(code, CodeEntry(entry.0))?;
    let write = words.last().ok_or(NO_RECORD)?;
    Ok(Count(usize::from(stage_write_of(*write).columns.0)))
}

/// Lays the columns of the emptied table an input's pipelines write into, with the store's room for where each writes
/// from.
///
/// # Errors
///
/// A pipeline of the input has no write the code holds.
pub(crate) fn laid(
    frame: &mut FillFrame<'_>,
    input: FillInputs<'_>,
    into: &mut TableRows,
) -> Result<(), FilledError> {
    let mut writes = std::mem::take(&mut frame.store.writes);
    let done = file_laid(
        frame.programs.code,
        file_cell(frame.file),
        input,
        &mut writes,
        into,
    );
    frame.store.writes = writes;
    done
}
