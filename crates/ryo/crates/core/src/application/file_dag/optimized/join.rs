//! Engine-owned physical lookup for a prepared plain Join over complete held inputs.
//! Primitive output semantics remain the declared left/right projection; index
//! choice, cache admission, fallback and diagnostics stay at this application edge.

use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::resources::{JoinResources, ResourceProgram};
use crate::domain::stored_indexes::{service as stored_indexes, types::Acquired};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    errors::TableModelError,
    service::{
        cell, cell_at, check_columns, count_of, index, index_selected, index_stored,
        joined_selected, key_of, kinds_of, matching, matching_selected, matching_stored,
        materialize, positions, read_row, rectangular, row_in,
    },
    types::{KeyIndex, On, RowCount, Rows, Table},
};
use ryo_language::domain::relations::{service::kinds, types::Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, Side};

const SPARSE_PAIR_LIMIT: usize = 65_536;
const SPARSE_RIGHT_LIMIT: usize = 1_024;

mod diagnostic;
use diagnostic::Counts;
pub(crate) use diagnostic::StreamJoinStats;

struct Work {
    left_row: Vec<Value>,
    output: Vec<Value>,
    key: Vec<Value>,
    right_row: Vec<Value>,
    index: KeyIndex,
    table: Table,
}
impl Work {
    fn take(context: &mut OperatorContext<'_>) -> Self {
        Self {
            left_row: std::mem::take(&mut context.scratch.row),
            output: std::mem::take(&mut context.scratch.other),
            key: std::mem::take(&mut context.scratch.join_scratch.key),
            right_row: std::mem::take(&mut context.scratch.join_scratch.right_row),
            index: std::mem::take(&mut context.scratch.join_scratch.order),
            table: std::mem::take(&mut context.scratch.join_scratch.right),
        }
    }
    fn restore(self, context: &mut OperatorContext<'_>) {
        context.scratch.row = self.left_row;
        context.scratch.other = self.output;
        context.scratch.join_scratch.key = self.key;
        context.scratch.join_scratch.right_row = self.right_row;
        context.scratch.join_scratch.order = self.index;
        context.scratch.join_scratch.right = self.table;
    }
}

fn projected(
    output: &[JoinColumn],
    left: &Rows<'_>,
    left_at: RowIndex,
    right: &Rows<'_>,
    right_at: RowIndex,
    into: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    into.clear();
    for column in output {
        let (rows, at) = match column.side {
            Side::Left => (left, left_at),
            Side::Right => (right, right_at),
        };
        into.push(
            cell_at(rows, at, column.column).ok_or(TableModelError::NoSuchColumn {
                index: column.column.0,
            })?,
        );
    }
    Ok(())
}

/// The sparse-left candidate is a complete pair list before its first output;
/// a dense pair set falls back without changing left-major bag order.
fn sparse_left(
    join: &JoinResources,
    logical_left: Input,
    logical_right: Input,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    work: &mut Work,
    counts: &mut Counts,
) -> Option<Result<(), OperatingError>> {
    let (Input::Relation(relation), Rows::Stored(view)) = (logical_left, left) else {
        return None;
    };
    let (left_count, right_count) = (count_of(left).0, count_of(right).0);
    if !rectangular(*view)
        || view.kinds != kinds(relation)
        || left_count > u32::MAX as usize
        || right_count == 0
        || right_count > SPARSE_RIGHT_LIMIT
        || left_count < right_count.saturating_mul(16)
        || matches!(right, Rows::Tree(_) | Rows::Selected(_))
        || matches!(right,Rows::Stored(other) if !rectangular(*other))
        || matches!(right,Rows::Computed(table) if table.cells.len()!=table.count.0.saturating_mul(table.kinds.len()))
    {
        return None;
    }
    let cache = context.stored_indexes.as_deref_mut()?;
    let complete_right = match right {
        Rows::Stored(other) => matches!(logical_right, Input::Relation(source)
            if cache.eligible.contains(&source) && other.kinds==kinds(source)),
        Rows::Computed(_) => true,
        _ => false,
    };
    if !cache.eligible.contains(&relation)
        || !complete_right
        || !stored_indexes::can_retain(cache, relation, *view, &join.left_on)
    {
        return None;
    }
    if let Err(error) = check_columns(kinds_of(right), &join.right_on) {
        return Some(Err(error.into()));
    }
    let acquired =
        match stored_indexes::acquire(cache, relation, *view, &join.left_on, &mut work.index) {
            Ok(value) => value,
            Err(error) => return Some(Err(error.into())),
        };
    counts.acquired(&acquired);
    let lookup = acquired.shared.as_deref().unwrap_or(&work.index);
    let mut pairs = Vec::new();
    for right_at in positions(RowCount(right_count)) {
        work.key.clear();
        for column in &join.right_on {
            let Some(value) = cell_at(right, right_at, *column) else {
                return None;
            };
            work.key.push(value);
        }
        if counts.enabled {
            counts.probes = counts.probes.saturating_add(1);
        }
        for left_at in matching_stored(*view, &join.left_on, lookup, &work.key) {
            if pairs.len() == SPARSE_PAIR_LIMIT {
                return None;
            }
            pairs.push((*left_at, right_at));
        }
    }
    pairs.sort_unstable();
    counts.sparse_left = true;
    for (left_at, right_at) in pairs {
        if let Err(error) = projected(
            &join.output,
            left,
            left_at,
            right,
            right_at,
            &mut work.output,
        ) {
            return Some(Err(error));
        }
        if counts.enabled {
            counts.hits = counts.hits.saturating_add(1);
        }
        if let Err(error) = emit.row(context, &work.output) {
            return Some(Err(error));
        }
    }
    Some(Ok(()))
}

fn ordinary(
    join: &JoinResources,
    logical_right: Input,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    work: &mut Work,
    counts: &mut Counts,
) -> Result<(), OperatingError> {
    if let Rows::Selected(selected) = right {
        index_selected(selected, &join.right_on, &mut work.index)?;
        if counts.enabled {
            counts.builds = counts.builds.saturating_add(1);
        }
        for left_at in positions(count_of(left)) {
            let Some(left_row) = read_row(left, left_at, &mut work.left_row) else {
                continue;
            };
            key_of(left_row, On::Columns(&join.left_on), &mut work.key)?;
            if counts.enabled {
                counts.probes = counts.probes.saturating_add(1);
            }
            for right_at in matching_selected(selected, &join.right_on, &work.index, &work.key) {
                joined_selected(selected, right_at, left_row, &join.output, &mut work.output)?;
                if counts.enabled {
                    counts.hits = counts.hits.saturating_add(1);
                }
                emit.row(context, &work.output)?;
            }
        }
        return Ok(());
    }
    if let Rows::Stored(view) = right
        && rectangular(*view)
    {
        let acquired = if let (Input::Relation(relation), Some(cache)) =
            (logical_right, context.stored_indexes.as_deref_mut())
        {
            stored_indexes::acquire(cache, relation, *view, &join.right_on, &mut work.index)?
        } else {
            index_stored(*view, &join.right_on, &mut work.index)?;
            Acquired {
                shared: None,
                built: true,
                eligible: false,
            }
        };
        counts.acquired(&acquired);
        let lookup = acquired.shared.as_deref().unwrap_or(&work.index);
        for left_at in positions(count_of(left)) {
            let Some(left_row) = read_row(left, left_at, &mut work.left_row) else {
                continue;
            };
            key_of(left_row, On::Columns(&join.left_on), &mut work.key)?;
            if counts.enabled {
                counts.probes = counts.probes.saturating_add(1);
            }
            for right_at in matching_stored(*view, &join.right_on, lookup, &work.key) {
                projected(
                    &join.output,
                    left,
                    left_at,
                    right,
                    *right_at,
                    &mut work.output,
                )?;
                if counts.enabled {
                    counts.hits = counts.hits.saturating_add(1);
                }
                emit.row(context, &work.output)?;
            }
        }
        return Ok(());
    }
    // A malformed physical view retains the original permissive materialize-
    // then-index path. Its incomplete rows are omitted before indexing.
    let looked_up = if let Rows::Computed(table) = right {
        *table
    } else {
        materialize(right, &mut work.table, &mut work.right_row);
        &work.table
    };
    index(looked_up, On::Columns(&join.right_on), &mut work.index)?;
    if counts.enabled {
        counts.builds = counts.builds.saturating_add(1);
    }
    for left_at in positions(count_of(left)) {
        let Some(left_row) = read_row(left, left_at, &mut work.left_row) else {
            continue;
        };
        key_of(left_row, On::Columns(&join.left_on), &mut work.key)?;
        if counts.enabled {
            counts.probes = counts.probes.saturating_add(1);
        }
        for right_at in matching(
            looked_up,
            On::Columns(&join.right_on),
            &work.index,
            &work.key,
        ) {
            work.output.clear();
            for column in &join.output {
                let source = match column.side {
                    Side::Left => left_row,
                    Side::Right => {
                        row_in(looked_up, *right_at).ok_or(OperatingError::ShapeMismatch)?
                    }
                };
                work.output.push(cell(source, column.column)?);
            }
            if counts.enabled {
                counts.hits = counts.hits.saturating_add(1);
            }
            emit.row(context, &work.output)?;
        }
    }
    Ok(())
}

/// None leaves native, reach and opaque cases on the mandatory common path.
/// The owner calls this after static/empty/schema gates and before ordinary
/// right-index acquisition, preserving old sparse-left admission order.
pub(crate) fn supports(program: &ResourceProgram, logical: &[Input], rows: &[Rows<'_>]) -> bool {
    let ResourceProgram::Join(join) = program else {
        return false;
    };
    let [left, right] = rows else {
        return false;
    };
    logical.len() >= 2
        && join.reach.is_none()
        && !join.has_filter
        && join.view.is_empty()
        && !matches!(left, Rows::Tree(_))
        && !matches!(right, Rows::Tree(_))
}

pub(crate) fn evaluate(
    program: &ResourceProgram,
    logical: &[Input],
    rows: &[Rows<'_>],
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Option<Result<(), OperatingError>> {
    if !supports(program, logical, rows) {
        return None;
    }
    let ResourceProgram::Join(join) = program else {
        return None;
    };
    let [left, right] = rows else {
        return None;
    };
    let [logical_left, logical_right, ..] = logical else {
        return None;
    };
    let mut counts = Counts {
        enabled: tracing::enabled!(target:"ryo::diagnostic::stored_join",tracing::Level::TRACE),
        ..Counts::default()
    };
    if count_of(left).0 == 0 || count_of(right).0 == 0 {
        counts.finish(right, &join.right_on, *logical_right, "stored-key", true);
        return Some(Ok(()));
    }
    let mut work = Work::take(context);
    let sparse = sparse_left(
        join,
        *logical_left,
        *logical_right,
        left,
        right,
        context,
        emit,
        &mut work,
        &mut counts,
    );
    let result = sparse.unwrap_or_else(|| {
        ordinary(
            join,
            *logical_right,
            left,
            right,
            context,
            emit,
            &mut work,
            &mut counts,
        )
    });
    let path = if counts.sparse_left {
        "sparse-left-key"
    } else if matches!(right, Rows::Selected(_))
        || matches!(right,Rows::Stored(view) if rectangular(*view))
    {
        "stored-key"
    } else {
        "materialized"
    };
    counts.finish(right, &join.right_on, *logical_right, path, result.is_ok());
    work.restore(context);
    Some(result)
}
