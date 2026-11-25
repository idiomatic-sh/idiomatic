//! The `probe` item of the `kernels` collection: each left row, in order, probes a `hash_index` of the right rows by its key.
//! An inner probe answers the left row with each right row of its key, ascending; a semi probe keeps the left row when its key
//! has a right row, an anti probe when it has none, and an ordered probe when none of them orders strictly past it toward
//! the probe's extremum (each key's extremum found once, as its first row).

use super::super::distinct::item::counted;
use super::super::filter::item::kept_nodes;
use super::super::hash_index::item::{find_cells, indexed, next_of};
use super::super::probe_right::item::{
    LeftRows, ProbeReads, ProbeScratch, number_at, probe_prepared, probe_reads, walked_tree,
};
use super::super::scan_facts::item::{
    cell_of, chunk_read, count_of, facts_cell, kinds_of, row_of, table_cell, table_row, facts_cell_at,
};
use super::super::scan_roots::item::{keyed_roots, next_found, tree_cell};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::service::answer_kinds;
use crate::domain::equality_join::types::{Existence, PartitionExtremum, ProbeMode, ProbeParts};
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, KeyedRun, NO_ROW};
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::shared_index::value::IndexRows;
use crate::domain::physical_plan::types::stream_scratch::value::{
    CellBound, ChunkRead, ChunkStop, RowChunk,
};
use crate::domain::physical_plan::types::{
    CellStart, InputRows, KernelRun, KeyedRowAt, KeyedSide, NodeFed, TreeRead,
};
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};



/// The right rows one left row matches: a key's run of the index, whether a row set holds the row, or the tree rows its walk
/// found.
#[derive(Clone, Copy, Debug)]
enum Matched<'a> {
    /// The keyed rows and the key's rows in them; none when it has none.
    Indexed(&'a KeyedRows, Option<KeyedRun>),
    /// Whether a whole-row index holds the left row.
    Member(SetMembership),
    /// The rows of this tree read the walk found, in the probe's `found`.
    Walked(TreeRead<'a>),
}

/// Appends the answers of the rows of `left` from its first unread row, probing the right rows readied by `probe_prepared`
/// from `right`, to `output`, until every row is read, a row faults, or `output` holds `bound` cells or more after a row: a
/// row's answers are never split. Both sides are read in place, whatever their kind (a chain's chunk, or a seeded closure's
/// facts, tree or table left); each side's kind is matched once for the call, and each answer cell is read by that kind's
/// own reader. A probe that walks the tree finds each row's rows from its node and counts them as the rows of `right` it
/// was fed.
///
/// # Errors
///
/// `ShapeMismatch` for an ordered probe's order that is no number, `NoSuchColumn` for one past its row.
#[expect(
    clippy::too_many_arguments,
    reason = "the left rows, the right rows, the run, the probe's own scratch, its feed, its answer and its bound"
)]
pub(crate) fn probe_rows(
    probe: &ProbeParts,
    left: RowChunk<'_>,
    right: InputRows<'_>,
    run: &KernelRun<'_>,
    scratch: &mut ProbeScratch<'_>,
    fed: &mut NodeFed,
    output: &mut Table,
    bound: CellBound,
) -> Result<ChunkRead, PortableOperatorFault> {
    let mut reads = probe_reads(probe, right, run, scratch, fed)?;
    // A chain's chunk is a table, read by the table's reader; a held join's left of another kind by `cell_of`.
    match left.rows {
        InputRows::Table(table) => {
            by_right(probe, &mut reads, left, output, bound, |at, column| {
                table_cell(table, at, column)
            })
        }
        // Spike (ryo-fmx3): a relation's left read by the relation's own reader.
        InputRows::Facts(view) if reads.fast_key => {
            by_right(probe, &mut reads, left, output, bound, |at, column| {
                facts_cell(view, at, column)
            })
        }
        InputRows::Facts(_) | InputRows::Tree(_) | InputRows::Row(..) => {
            by_right(probe, &mut reads, left, output, bound, |at, column| {
                cell_of(left.rows, at, column)
            })
        }
    }
}

/// `probe_rows` past its left rows' kind: the right rows' kind matched once, each left row probed with both kinds' readers.
fn by_right(
    probe: &ProbeParts,
    reads: &mut ProbeReads<'_>,
    left: RowChunk<'_>,
    output: &mut Table,
    bound: CellBound,
    left_cell: impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> Result<ChunkRead, PortableOperatorFault> {
    let (right, rows, cells) = (reads.right, left.rows, &left_cell);
    match right {
        InputRows::Facts(view) => chunk_read(left, output, bound, |at, output| {
            let right_cell = |row, column| facts_cell(view, row, column);
            row_probed(probe, reads, rows, at, output, cells, &right_cell)
        }),
        InputRows::Tree(tree) => chunk_read(left, output, bound, |at, output| {
            let right_cell = |row, column| tree_cell(tree, row, column);
            row_probed(probe, reads, rows, at, output, cells, &right_cell)
        }),
        InputRows::Table(table) => chunk_read(left, output, bound, |at, output| {
            let right_cell = |row, column| table_cell(table, row, column);
            row_probed(probe, reads, rows, at, output, cells, &right_cell)
        }),
        InputRows::Row(..) => chunk_read(left, output, bound, |at, output| {
            let right_cell = |row, column| cell_of(right, row, column);
            row_probed(probe, reads, rows, at, output, cells, &right_cell)
        }),
    }
}

/// Appends the answer of row `at` of `left` to `output` and answers the row's fault, reading the left's cells with
/// `left_cell` and the right's with `right_cell` (a walked probe reads the rows it found from the tree).
///
/// # Errors
///
/// `ShapeMismatch` for an ordered probe's order that is no number, `NoSuchColumn` for one past its row.
fn row_probed(
    probe: &ProbeParts,
    reads: &mut ProbeReads<'_>,
    left: InputRows<'_>,
    at: RowIndex,
    output: &mut Table,
    left_cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    right_cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let ProbeReads {
        right,
        walked,
        keys: [left_on, right_on],
        index,
        row,
        repeats,
        decisions,
        found: nodes,
        kept,
        matcher,
        fed,
        left: reading,
        fast_key,
    } = reads;
    let (right, index) = (*right, *index);
    if *reading == LeftRows::Kept {
        let start = CellStart(output.cells.len());
        output.cells.extend_from_slice(row_of(left, at, row));
        counted(output, start, probe.repeated, repeats);
        return Ok(None);
    }
    let matched = match (*walked, index) {
        // Spike (ryo-fmx3): the left key read by the left's reader, the index's rows by the right's.
        (None, IndexRows::Keyed(rows)) if *fast_key => {
            match super::super::hash_index::item::find_read(rows, right_on, right_cell, left_on, at, left_cell) {
                Ok(found) => Matched::Indexed(rows, found),
                Err(fault) => return Ok(Some(fault)),
            }
        }
        (None, IndexRows::Keyed(rows)) => match find_cells(
            rows,
            KeyedSide {
                rows: right,
                on: right_on,
            },
            KeyedRowAt {
                side: KeyedSide {
                    rows: left,
                    on: left_on,
                },
                row: at,
            },
        ) {
            Ok(found) => Matched::Indexed(rows, found),
            Err(fault) => return Ok(Some(fault)),
        },
        // A whole-row index is asked only whether it holds the left row.
        (None, IndexRows::Rows(set)) => {
            Matched::Member(set.holds(kinds_of(right), right_cell, row_of(left, at, row)))
        }
        (Some(tree), _) => {
            let (Some(left_key), Some(right_key)) = (left_on.first(), right_on.first()) else {
                return Err(PortableOperatorFault::WrongOperation);
            };
            let Some(key) = left_cell(at, *left_key) else {
                return Ok(Some(PortableOperatorFault::NoSuchColumn {
                    index: *left_key,
                }));
            };
            nodes.clear();
            keyed_roots(&tree, *right_key, key, nodes);
            if let Some(slot) = fed.0.get_mut(1) {
                *slot = RowCount(slot.0.saturating_add(nodes.len()));
            }
            if let Some(kernel) = *kept
                && let Some(fault) = kept_nodes(tree, kernel, *matcher, row, decisions, nodes)
            {
                return Ok(Some(fault));
            }
            Matched::Walked(tree)
        }
    };
    if probe.mode == ProbeMode::Inner {
        match matched {
            // Spike (ryo-fmx3): a table left's row read once, each answer cell read off the row's slice; a table or a
            // relation right read by its own reader, with no closure per cell.
            Matched::Indexed(rows, key) if matches!(left, InputRows::Table(_)) => {
                let InputRows::Table(left_table) = left else {
                    return Err(PortableOperatorFault::WrongOperation);
                };
                let left_row = row_cells(left_table, at);
                let mut matched_row = key.map_or(NO_ROW, |key| key.first);
                while matched_row != NO_ROW {
                    let right_at = indexed(matched_row);
                    let fault = match right {
                        InputRows::Table(right_table) => {
                            let right_row = row_cells(right_table, right_at);
                            answered_rows(probe, left_row, output, repeats, |column| {
                                right_row.get(column).copied()
                            })
                        }
                        InputRows::Facts(view) => {
                            answered_rows(probe, left_row, output, repeats, |column| {
                                facts_cell_at(view, right_at, column)
                            })
                        }
                        InputRows::Tree(_) | InputRows::Row(..) => answered(
                            probe,
                            at,
                            right_at,
                            output,
                            repeats,
                            left_cell,
                            right_cell,
                        ),
                    };
                    if let Some(fault) = fault {
                        return Ok(Some(fault));
                    }
                    matched_row = next_of(rows, matched_row);
                }
            }
            Matched::Indexed(rows, key) => {
                let mut matched_row = key.map_or(NO_ROW, |key| key.first);
                while matched_row != NO_ROW {
                    if let Some(fault) = answered(
                        probe,
                        at,
                        indexed(matched_row),
                        output,
                        repeats,
                        left_cell,
                        right_cell,
                    ) {
                        return Ok(Some(fault));
                    }
                    matched_row = next_of(rows, matched_row);
                }
            }
            Matched::Member(_) => return Err(PortableOperatorFault::WrongOperation),
            Matched::Walked(tree) => {
                let walked_rows = TreeRead {
                    candidates: nodes.as_slice(),
                    ..tree
                };
                let found = RowCount(nodes.len());
                let mut matched_row = next_found(None, found);
                while matched_row != NO_ROW {
                    if let Some(fault) = answered(
                        probe,
                        at,
                        indexed(matched_row),
                        output,
                        repeats,
                        left_cell,
                        &|row, column| tree_cell(&walked_rows, row, column),
                    ) {
                        return Ok(Some(fault));
                    }
                    matched_row = next_found(Some(matched_row), found);
                }
            }
        }
        return Ok(None);
    }
    if let ProbeMode::Ordered(order) = probe.mode {
        // An ordered probe keeps its index (`tree_probe`): it never walks.
        let Matched::Indexed(_, found) = matched else {
            return Err(PortableOperatorFault::WrongOperation);
        };
        let passed = match found {
            None => false,
            Some(key) => {
                let bound = number_at(right, indexed(key.first), order.right_order)?;
                let own = number_at(left, at, order.left_order)?;
                match order.extremum {
                    PartitionExtremum::Maximum => bound > own,
                    PartitionExtremum::Minimum => bound < own,
                }
            }
        };
        if !passed {
            let start = CellStart(output.cells.len());
            output.cells.extend_from_slice(row_of(left, at, row));
            counted(output, start, probe.repeated, repeats);
        }
        return Ok(None);
    }
    let exists = match (probe.existence, matched) {
        (_, Matched::Walked(_)) => !nodes.is_empty(),
        (_, Matched::Member(member)) => member == SetMembership::In,
        (_, Matched::Indexed(_, None)) => false,
        (Existence::First, Matched::Indexed(_, Some(_))) => true,
        (Existence::Every, Matched::Indexed(rows, Some(key))) => {
            let mut count = RowCount(0);
            let mut matched_row = key.first;
            while matched_row != NO_ROW {
                count = RowCount(count.0.saturating_add(1));
                matched_row = next_of(rows, matched_row);
            }
            count.0 > 0
        }
    };
    if exists == (probe.mode == ProbeMode::Semi) {
        let start = CellStart(output.cells.len());
        output.cells.extend_from_slice(row_of(left, at, row));
        counted(output, start, probe.repeated, repeats);
    }
    Ok(None)
}

/// Appends an inner probe's answer of left row `at` and right row `right_row` to `output`, each column read from its side by
/// its reader, and drops it when it repeats; answers the fault of a column past its side's row, writing nothing of it.
fn answered(
    probe: &ProbeParts,
    at: RowIndex,
    right_row: RowIndex,
    output: &mut Table,
    repeats: &mut RowSet,
    left_cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    right_cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> Option<PortableOperatorFault> {
    let start = CellStart(output.cells.len());
    for column in &probe.output {
        let cell = match column.side {
            Side::Left => left_cell(at, column.column),
            Side::Right => right_cell(right_row, column.column),
        };
        let Some(cell) = cell else {
            output.cells.truncate(start.0);
            return Some(PortableOperatorFault::NoSuchColumn {
                index: column.column,
            });
        };
        output.cells.push(cell);
    }
    counted(output, start, probe.repeated, repeats);
    None
}

/// Spike (ryo-fmx3): the cells of row `row` of a table, exactly its width, as `table_cell` reads them: none past its rows.
#[inline]
fn row_cells(table: &Table, row: RowIndex) -> &[Value] {
    if row.0 >= table.count.0 || table.kinds.is_empty() {
        return &[];
    }
    table_row(table, row)
}


/// Spike (ryo-fmx3): `answered` with the left row's cells in hand and the right row's read by `right_cell`.
#[inline]
fn answered_rows(
    probe: &ProbeParts,
    left_row: &[Value],
    output: &mut Table,
    repeats: &mut RowSet,
    right_cell: impl Fn(usize) -> Option<Value>,
) -> Option<PortableOperatorFault> {
    let start = CellStart(output.cells.len());
    for column in &probe.output {
        let at = usize::try_from(column.column.0).unwrap_or(usize::MAX);
        let cell = match column.side {
            Side::Left => left_row.get(at).copied(),
            Side::Right => right_cell(at),
        };
        let Some(cell) = cell else {
            output.cells.truncate(start.0);
            return Some(PortableOperatorFault::NoSuchColumn {
                index: column.column,
            });
        };
        output.cells.push(cell);
    }
    counted(output, start, probe.repeated, repeats);
    None
}

/// Appends the answer of each row of `left` probing the rows of `right` to `output`, and answers the first row's fault: the
/// right filter's, before any answer, or a left row's, after the answers of the rows before it. A left of no row reaches no
/// right row: it answers its layout before any right key or filter is read. A blocking kernel's join over rows it made (a
/// seeded closure's), its left read in place, unbound, through `probe_rows`, as the chain of streamed stages reads a
/// streamed probe's chunks.
///
/// # Errors
///
/// `NoSuchColumn` for an answer column, or a right key column, past its rows.
pub(crate) fn probed(
    probe: &ProbeParts,
    left: InputRows<'_>,
    right: InputRows<'_>,
    run: &KernelRun<'_>,
    mut scratch: ProbeScratch<'_>,
    fed: &mut NodeFed,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    output.kinds = answer_kinds(probe, kinds_of(left), kinds_of(right))?;
    let walked = walked_tree(scratch.build, right)?.is_some();
    if walked && let Some(slot) = fed.0.get_mut(1) {
        *slot = RowCount(0);
    }
    if count_of(left).0 == 0 {
        return Ok(None);
    }
    if let Some(fault) = probe_prepared(probe, kinds_of(left), right, run, &mut scratch)? {
        return Ok(Some(fault));
    }
    scratch.repeats.cleared();
    let read = probe_rows(
        probe,
        RowChunk {
            rows: left,
            from: RowIndex(0),
        },
        right,
        run,
        &mut scratch,
        fed,
        output,
        CellBound(usize::MAX),
    )?;
    // Unbound, it reads every row in one call, or stops at a row's fault.
    match read.stop {
        ChunkStop::Faulted(fault) => Ok(Some(fault)),
        ChunkStop::Hard(fault) => Err(fault),
        ChunkStop::Ended | ChunkStop::Full => Ok(None),
    }
}
