//! The `join` operator: combine the rows of two inputs whose keys are equal.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second};
use crate::domain::operating::types::{
    Emit, Feed, InputPlace, OperatorContext, Room, Supply, TreeShape,
};
use crate::domain::positions::types::KeyColumns;
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::stored_indexes::types::Acquired;
use crate::domain::table_cells::types::{RowIndex, TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    cell, column_in, count_of, index, index_stored, key_of, kinds_of, matching_stored, positions,
    read_row, rectangular, reset, row_in, table_in,
};
use crate::domain::table_model::types::{KeyIndex, On, RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::tree_join::collections::strategies::hash::item::HashItem;
use crate::domain::tree_join::service::{
    give_back, join_tree, probe, source_column, take_state, tree_finding,
};
use crate::domain::tree_join::types::{EdgeHit, TreeState};
use ryo_language::domain::relations::service::{node_file, node_position};
use ryo_language::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;

/// `join`, one item of the `operators` collection.
pub(crate) struct JoinItem;

/// The columns of the answer, as they are in the left rows and in the rows looked up: those of a tree through the view of it.
fn answer_kinds(
    left: &[CellKind],
    right: &[CellKind],
    view: Option<&[ColumnIndex]>,
    output: &[JoinColumn],
    out: &mut Table,
) -> Result<(), OperatingError> {
    reset(out, &[]);
    for column in output {
        let (kinds, place) = match (column.side, view) {
            (Side::Left, _) => (left, column.column),
            (Side::Right, Some(view)) => (right, source_column(view, column.column)),
            (Side::Right, None) => (right, column.column),
        };
        out.kinds.push(
            *column_in(kinds, place).ok_or(TableModelError::NoSuchColumn {
                index: column.column.0,
            })?,
        );
    }
    Ok(())
}

/// Writes into `into` the cells of one answer: each column of `output`, from the left row or the right row.
fn joined_row(
    output: &[JoinColumn],
    left_row: &[Value],
    right_row: &[Value],
    into: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    into.clear();
    for column in output {
        let source = match column.side {
            Side::Left => left_row,
            Side::Right => right_row,
        };
        into.push(cell(source, column.column)?);
    }
    Ok(())
}

/// The rows of `looked_up` whose node in the column `right_key` is an element of the tree of `edges` other than its root, each
/// with the edge that leads to it, in the order of the edges.
///
/// # Errors
///
/// `NoSuchColumn` when the rows lack the column.
fn edge_hits(
    edges: &TreeRows<'_>,
    looked_up: &Table,
    right_key: ColumnIndex,
) -> Result<Vec<EdgeHit>, OperatingError> {
    let mut hits = Vec::new();
    for found in 0..looked_up.count.0 {
        let Some(right_row) = row_in(looked_up, RowIndex(found)) else {
            continue;
        };
        let Value::Node(entity) = cell(right_row, right_key)? else {
            continue;
        };
        let place = usize::try_from(node_position(entity).0).unwrap_or(usize::MAX);
        if node_file(entity) == edges.file && place >= 1 && place < edges.elements.kind.len() {
            hits.push(EdgeHit {
                edge: RowIndex(place - 1),
                found: RowIndex(found),
            });
        }
    }
    hits.sort_unstable();
    Ok(hits)
}

/// Acquire an owned shared handle only after the existing provider read has succeeded.
/// Its mutable cache borrow ends before any probe can emit into another operator.
fn stored_index_for(
    right_input: Input,
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
    context: &mut OperatorContext<'_>,
    local: &mut KeyIndex,
) -> Result<Acquired, OperatingError> {
    if let Input::Relation(relation) = right_input
        && let Some(cache) = context.stored_indexes.as_deref_mut()
    {
        return Ok(crate::domain::stored_indexes::service::acquire(
            cache, relation, view, columns, local,
        )?);
    }
    index_stored(view, columns, local)?;
    Ok(Acquired {
        shared: None,
        built: true,
        eligible: false,
    })
}

/// Both generic execution paths share one dynamic tracing callsite and dispatch interest.
fn stored_join_diagnostics_enabled() -> bool {
    tracing::enabled!(target: "ryo::diagnostic::stored_join", tracing::Level::TRACE)
}

/// Emit one explicitly enabled diagnostic for a completed generic join invocation.
fn stored_join_diagnostic(
    right: &Rows<'_>,
    keys: &[ColumnIndex],
    right_input: Input,
    path: &str,
    counts: (u64, u64, u64),
    cache: (bool, u64),
    complete: bool,
) {
    tracing::event!(
        target: "ryo::diagnostic::stored_join",
        tracing::Level::TRACE,
        rhs_rows = count_of(right).0,
        rhs_width = kinds_of(right).len(),
        key_width = keys.len(),
        right_input = ?right_input,
        key_columns = ?keys,
        rhs_kinds = ?kinds_of(right),
        read_state_observed = cache.0,
        complete_relation_eligible = cache.0,
        cache_reuses = cache.1,
        builds = counts.0,
        probes = counts.1,
        hits = counts.2,
        path,
        complete,
    );
}

/// Validated generic lookup columns borrowed from the join operation.
struct LookupColumns<'a> {
    /// Logical right source used for file-cache eligibility.
    right_input: Input,
    /// Columns read from each left row.
    left_on: &'a [ColumnIndex],
    /// Ordered columns of the right lookup key.
    right_on: &'a [ColumnIndex],
    /// Cells retained in the emitted row.
    output: &'a [JoinColumn],
}

/// Scratch buffers moved out for a computed join, restored even when it refuses a row.
struct JoinBuffers {
    /// Decoded left row or materialization workspace.
    row: Vec<Value>,
    /// Output row passed to downstream operators.
    other: Vec<Value>,
    /// Current typed lookup key.
    key: Vec<Value>,
    /// Invocation-local fallback index.
    order: KeyIndex,
    /// Decoded right rows or one stored hit.
    built: Table,
}

/// Aggregate diagnostics and lazy stream index ownership for one invocation.
#[derive(Default)]
struct LookupState<'a> {
    /// Rectangular stored view selected once before a stream starts.
    stored: Option<ColumnsView<'a>>,
    /// Whether the opt-in aggregate target was enabled on entry.
    enabled: bool,
    /// Physical index builds in this invocation.
    builds: u64,
    /// Left rows probed in this invocation.
    probes: u64,
    /// Matching rows visited in this invocation.
    hits: u64,
    /// Whether the resident relation passed the owner's completeness gate.
    cache_eligible: bool,
    /// Shared acquisitions after an earlier complete build.
    cache_reuses: u64,
    /// Whether a stream's first left row has prepared its right lookup.
    indexed: bool,
    /// Owned handle whose cache borrow ended before downstream emission.
    shared: Option<std::sync::Arc<KeyIndex>>,
}

/// Probe rectangular resident columns, decoding payload only for actual hits.
fn join_stored_rows(
    columns: &LookupColumns<'_>,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    buffers: &mut JoinBuffers,
    state: &mut LookupState<'_>,
) -> Result<(), OperatingError> {
    let LookupColumns {
        right_input,
        left_on,
        right_on,
        output,
    } = columns;
    let JoinBuffers {
        row,
        other,
        key,
        order,
        built,
    } = buffers;
    let Rows::Stored(stored_view) = right else {
        return Err(OperatingError::WrongOperation);
    };
    let acquired = stored_index_for(*right_input, *stored_view, right_on, context, order)?;
    if state.enabled {
        state.builds = state.builds.saturating_add(u64::from(acquired.built));
        state.cache_reuses = state
            .cache_reuses
            .saturating_add(u64::from(!acquired.built));
        state.cache_eligible = acquired.eligible;
    }
    let lookup = acquired.shared.as_deref().unwrap_or(order);
    reset(built, stored_view.kinds);
    for position in positions(count_of(left)) {
        let Some(left_row) = read_row(left, position, row) else {
            continue;
        };
        key_of(left_row, On::Columns(left_on), key)?;
        if state.enabled {
            state.probes = state.probes.saturating_add(1);
        }
        for hit in matching_stored(*stored_view, right_on, lookup, key) {
            let Some(right_row) = read_row(right, *hit, &mut built.cells) else {
                continue;
            };
            built.count = RowCount(1);
            if state.enabled {
                state.hits = state.hits.saturating_add(1);
            }
            joined_row(output, left_row, right_row, other)?;
            emit.row(context, other)?;
        }
    }
    Ok(())
}

/// Preserve the materialized and sparse-tree paths of a computed join.
fn join_materialized_rows(
    columns: &LookupColumns<'_>,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    buffers: &mut JoinBuffers,
    state: &mut LookupState<'_>,
) -> Result<(), OperatingError> {
    let LookupColumns {
        left_on,
        right_on,
        output,
        ..
    } = columns;
    let JoinBuffers {
        row,
        other,
        key,
        order,
        built,
    } = buffers;
    let looked_up = table_in(right, built, row);
    // A few rows on the right find their edges of the tree by the child, and no pass over the edges is made.
    if let (
        Rows::Tree(
            edges @ TreeRows {
                form: TreeForm::Edges,
                ..
            },
        ),
        [ColumnIndex(1)],
        [right_key],
    ) = (left, *left_on, *right_on)
        && looked_up.count.0 * 16 < count_of(left).0
    {
        let edge_matches = edge_hits(edges, looked_up, *right_key)?;
        for EdgeHit { edge, found } in edge_matches {
            let (Some(left_row), Some(right_row)) =
                (read_row(left, edge, row), row_in(looked_up, found))
            else {
                continue;
            };
            joined_row(output, left_row, right_row, other)?;
            emit.row(context, other)?;
        }
        return Ok(());
    }
    index(looked_up, On::Columns(right_on), order)?;
    if state.enabled {
        state.builds = state.builds.saturating_add(1);
    }
    for position in positions(count_of(left)) {
        let Some(left_row) = read_row(left, position, row) else {
            continue;
        };
        if state.enabled {
            state.probes = state.probes.saturating_add(1);
        }
        for hit in HashItem::find(
            looked_up,
            order,
            KeyColumns {
                left: left_on,
                right: right_on,
            },
            left_row,
            key,
        )? {
            let right_row = row_in(looked_up, *hit).unwrap_or_default();
            if state.enabled {
                state.hits = state.hits.saturating_add(1);
            }
            joined_row(output, left_row, right_row, other)?;
            emit.row(context, other)?;
        }
    }
    Ok(())
}

/// Passes on the answer of the join of `left` and `right`, both read in place.
///
/// # Errors
///
/// Whatever the join or the one that takes the rows refuse.
pub(crate) fn join_rows(
    operation: &Operation,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Join {
        right: right_input,
        left_on,
        right_on,
        output,
        filter,
        view,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    if let Rows::Tree(tree) = right
        && let Some(finding) = tree_finding(tree, left_on, right_on, view)
    {
        return join_tree(
            tree,
            left,
            finding,
            TreeShape {
                comparisons: filter,
                view,
                output,
            },
            context,
            emit,
        );
    }
    if !filter.is_empty() || !view.is_empty() {
        return Err(OperatingError::WrongOperation);
    }
    // Nothing on either side joins with nothing: the other side is not even read.
    if count_of(left).0 == 0 || count_of(right).0 == 0 {
        return Ok(());
    }
    let mut buffers = JoinBuffers {
        row: std::mem::take(&mut context.scratch.row),
        other: std::mem::take(&mut context.scratch.other),
        key: std::mem::take(&mut context.scratch.key),
        order: std::mem::take(&mut context.scratch.order),
        built: std::mem::take(&mut context.scratch.right),
    };
    let mut state = LookupState {
        enabled: !matches!(left, Rows::Tree(_))
            && !matches!(right, Rows::Tree(_))
            && stored_join_diagnostics_enabled(),
        ..LookupState::default()
    };
    let stored = matches!(right, Rows::Stored(stored_view) if rectangular(*stored_view));
    let columns = LookupColumns {
        right_input: *right_input,
        left_on,
        right_on,
        output,
    };
    let outcome = if stored && !matches!(left, Rows::Tree(_)) {
        join_stored_rows(
            &columns,
            left,
            right,
            context,
            emit,
            &mut buffers,
            &mut state,
        )
    } else {
        join_materialized_rows(
            &columns,
            left,
            right,
            context,
            emit,
            &mut buffers,
            &mut state,
        )
    };
    if state.enabled {
        stored_join_diagnostic(
            right,
            right_on,
            *right_input,
            if stored { "stored-key" } else { "materialized" },
            (state.builds, state.probes, state.hits),
            (state.cache_eligible, state.cache_reuses),
            outcome.is_ok(),
        );
    }
    context.scratch.row = buffers.row;
    context.scratch.other = buffers.other;
    context.scratch.key = buffers.key;
    context.scratch.order = buffers.order;
    context.scratch.right = buffers.built;
    outcome
}

impl OperatorsItem for JoinItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Join { output, view, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let (left, right) = (first(inputs)?, second(inputs)?);
        let tree = matches!(right, Rows::Tree(_));
        answer_kinds(
            kinds_of(left),
            kinds_of(right),
            tree.then_some(view.as_slice()),
            output,
            out,
        )?;
        join_rows(operation, left, right, context, &mut Collect(out))
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::Join {
            left_on,
            right_on,
            output,
            filter,
            view,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Room {
            memo,
            scratch,
            held,
            ..
        } = room;
        let [left_held, right_held, _] = held;
        // A side that has to be made is made only when the other has a row: when nothing is on the left nothing joins. A left
        // that is not made as it is read, or a right that has to be made, is held whole first.
        if ins.streams(InputPlace(0)) == Supply::AtHand
            || ins.streams(InputPlace(1)) == Supply::Streamed
        {
            let left = ins.hold(InputPlace(0), context, left_held)?;
            if count_of(&left).0 == 0 {
                return Ok(());
            }
            let right = ins.hold(InputPlace(1), context, right_held)?;
            let mut own = derive(context, memo, scratch);
            return join_rows(operation, &left, &right, &mut own, emit);
        }
        // Rows as they are made on the left, the right at hand: each finds its rows on the right as it comes.
        let right = ins.hold(InputPlace(1), context, right_held)?;
        if let Rows::Tree(tree) = &right
            && let Some(finding) = tree_finding(tree, left_on, right_on, view)
        {
            let mut state: Option<TreeState> = None;
            let mut found = 0_usize;
            let outcome = ins.stream(
                InputPlace(0),
                context,
                &mut Pipe(|each: &mut OperatorContext<'_>, left_row: &[Value]| {
                    let mut counted = Pipe(|inner: &mut OperatorContext<'_>, row: &[Value]| {
                        found += 1;
                        emit.row(inner, row)
                    });
                    let mut own = derive(each, memo, scratch);
                    if state.is_none() {
                        state = Some(take_state(tree, filter, &mut own));
                    }
                    match state.as_mut() {
                        Some(state) => probe(
                            state,
                            tree,
                            finding,
                            TreeShape {
                                comparisons: filter,
                                view,
                                output,
                            },
                            left_row,
                            &mut own,
                            &mut counted,
                        ),
                        None => Ok(()),
                    }
                }),
            );
            if let Some(state) = state {
                let mut own = derive(context, memo, scratch);
                give_back(state, &mut own);
            }
            ins.found(InputPlace(1), RowCount(found));
            return outcome;
        }
        if !filter.is_empty() || !view.is_empty() {
            return Err(OperatingError::WrongOperation);
        }
        stream_lookup(operation, ins, &right, scratch, context, emit)
    }
}

/// Probe one incoming left row after the original lazy index preparation point.
fn stream_lookup_row(
    columns: &LookupColumns<'_>,
    right: &Rows<'_>,
    left_row: &[Value],
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    scratch: &mut OperatorScratch,
    state: &mut LookupState<'_>,
) -> Result<(), OperatingError> {
    let LookupColumns {
        right_input,
        left_on,
        right_on,
        output,
    } = columns;
    let OperatorScratch {
        row,
        other,
        key,
        order,
        right: built,
        ..
    } = scratch;
    let stored = state.stored;
    if let Some(view) = stored {
        if !state.indexed {
            let acquired = stored_index_for(*right_input, view, right_on, context, order)?;
            if state.enabled {
                state.builds = state.builds.saturating_add(u64::from(acquired.built));
                state.cache_reuses = state
                    .cache_reuses
                    .saturating_add(u64::from(!acquired.built));
                state.cache_eligible = acquired.eligible;
            }
            state.shared = acquired.shared;
            reset(built, view.kinds);
            state.indexed = true;
        }
        key_of(left_row, On::Columns(left_on), key)?;
        if state.enabled {
            state.probes = state.probes.saturating_add(1);
        }
        let lookup = state.shared.as_deref().unwrap_or(order);
        for hit in matching_stored(view, right_on, lookup, key) {
            let Some(right_row) = read_row(right, *hit, &mut built.cells) else {
                continue;
            };
            built.count = RowCount(1);
            if state.enabled {
                state.hits = state.hits.saturating_add(1);
            }
            joined_row(output, left_row, right_row, other)?;
            emit.row(context, other)?;
        }
        return Ok(());
    }
    let looked_up = match right {
        Rows::Computed(table) => {
            if !state.indexed {
                index(table, On::Columns(right_on), order)?;
                if state.enabled {
                    state.builds = state.builds.saturating_add(1);
                }
                state.indexed = true;
            }
            table
        }
        Rows::Stored(_) | Rows::Tree(_) => {
            if !state.indexed {
                let table = table_in(right, built, row);
                index(table, On::Columns(right_on), order)?;
                if state.enabled {
                    state.builds = state.builds.saturating_add(1);
                }
                state.indexed = true;
            }
            &*built
        }
    };
    if state.enabled {
        state.probes = state.probes.saturating_add(1);
    }
    for hit in HashItem::find(
        looked_up,
        order,
        KeyColumns {
            left: left_on,
            right: right_on,
        },
        left_row,
        key,
    )? {
        let right_row = row_in(looked_up, *hit).unwrap_or_default();
        if state.enabled {
            state.hits = state.hits.saturating_add(1);
        }
        joined_row(output, left_row, right_row, other)?;
        emit.row(context, other)?;
    }
    Ok(())
}

/// A join whose left rows come as they are made and whose right is a table at hand: each row of the left finds its rows on the
/// right as it comes, the right indexed by its key when the first row comes.
///
/// # Errors
///
/// What indexing the right, the lookup or the one that takes the rows refuse.
fn stream_lookup(
    operation: &Operation,
    ins: &mut dyn Feed<'_>,
    right: &Rows<'_>,
    scratch: &mut OperatorScratch,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Join {
        right: right_input,
        left_on,
        right_on,
        output,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let stored = match right {
        Rows::Stored(view) if rectangular(*view) => Some(*view),
        Rows::Stored(_) | Rows::Tree(_) | Rows::Computed(_) => None,
    };
    let columns = LookupColumns {
        right_input: *right_input,
        left_on,
        right_on,
        output,
    };
    let mut state = LookupState {
        stored,
        enabled: !matches!(right, Rows::Tree(_)) && stored_join_diagnostics_enabled(),
        ..LookupState::default()
    };
    let outcome = ins.stream(
        InputPlace(0),
        context,
        &mut Pipe(|each: &mut OperatorContext<'_>, left_row: &[Value]| {
            stream_lookup_row(&columns, right, left_row, each, emit, scratch, &mut state)
        }),
    );
    if state.enabled {
        stored_join_diagnostic(
            right,
            right_on,
            *right_input,
            if stored.is_some() {
                "stored-key"
            } else {
                "materialized"
            },
            (state.builds, state.probes, state.hits),
            (state.cache_eligible, state.cache_reuses),
            outcome.is_ok(),
        );
    }
    outcome
}
