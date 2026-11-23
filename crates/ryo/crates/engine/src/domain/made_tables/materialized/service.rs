//! Construct, share and return materialized column results in a worker.
use super::constants::{CLASSES, COLUMN_CELLS, HEADERS};
use super::types::{ResultColumns, TableBuffers};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;
use std::rc::Rc;

/// Obtain a cleared column of sufficient capacity.
fn take<Word, Name>(buffers: &mut TableBuffers<Word, Name>, length: Count) -> Vec<Word> {
    let length = length.0;
    let (low, high) = if length == 0 {
        (0, 8)
    } else {
        let class = usize::try_from((length - 1).bit_width()).unwrap_or_default();
        (class.saturating_sub(1), (class + 4).min(CLASSES - 1))
    };
    for class in buffers.columns.iter_mut().take(high + 1).skip(low) {
        if let Some(mut column) = class.pop() {
            buffers.capacity.0 -= column.capacity();
            column.reserve(length);
            return column;
        }
    }
    Vec::with_capacity(if length >= 16 {
        length.next_power_of_two()
    } else {
        length
    })
}

/// Make a table ready to receive its estimated population.
pub fn prepare<Word, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    names: Rc<Vec<Name>>,
    tys: Rc<Vec<CellKind>>,
    length: Count,
) -> ResultColumns<Word, Name> {
    let mut columns = buffers.headers.pop().unwrap_or_default();
    columns.reserve(names.len());
    columns.extend(std::iter::repeat_with(|| take(buffers, length)).take(names.len()));
    ResultColumns {
        names,
        tys,
        cols: columns,
        rows: Count(0),
    }
}

/// Append one row in schema order.
///
/// # Panics
///
/// Panics when the caller supplies more cells than the bound width.
pub fn append<Word: Copy, Name>(table: &mut ResultColumns<Word, Name>, cells: &[Word]) {
    table
        .cols
        .resize_with(table.cols.len().max(table.names.len()), Vec::new);
    assert!(
        cells.len() <= table.cols.len(),
        "row exceeds its bound width"
    );
    for (column, &cell) in table.cols.iter_mut().zip(cells) {
        column.push(cell);
    }
    table.rows.0 += 1;
}

/// Gather an existing row in column order.
pub fn gather<Word: Copy, Name>(
    table: &ResultColumns<Word, Name>,
    row: RowIndex,
    cells: &mut Vec<Word>,
) -> Option<()> {
    cells.clear();
    if row.0 >= table.rows.0 {
        return None;
    }
    for column in &table.cols {
        let Some(&cell) = column.get(row.0) else {
            cells.clear();
            return None;
        };
        cells.push(cell);
    }
    Some(())
}

/// Copy a table into retained capacity.
pub fn copy<Word: Copy, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    table: &ResultColumns<Word, Name>,
) -> ResultColumns<Word, Name> {
    let mut columns = buffers.headers.pop().unwrap_or_default();
    columns.reserve(table.cols.len());
    for source in &table.cols {
        let mut destination = take(buffers, Count(source.len()));
        destination.extend_from_slice(source);
        columns.push(destination);
    }
    ResultColumns {
        names: Rc::clone(&table.names),
        tys: Rc::clone(&table.tys),
        cols: columns,
        rows: table.rows,
    }
}

/// Add a computed constant column to an already sized result.
pub fn extend<Word: Copy, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    table: &mut ResultColumns<Word, Name>,
    value: Word,
    length: Count,
) {
    let mut column = take(buffers, length);
    column.resize(length.0, value);
    table.cols.push(column);
}

/// Find a named projected column.
#[must_use]
pub fn position<Word, Name: PartialEq<Query>, Query: ?Sized>(
    table: &ResultColumns<Word, Name>,
    name: &Query,
) -> Option<ColumnIndex> {
    let at = table.names.iter().position(|column| column == name)?;
    let Ok(index) = u32::try_from(at) else {
        return None;
    };
    Some(ColumnIndex(index))
}

/// Publish the result without copying its columns.
pub fn share<Word, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    table: ResultColumns<Word, Name>,
) -> Rc<ResultColumns<Word, Name>> {
    let Some(mut slot) = buffers.shared.pop() else {
        return Rc::new(table);
    };
    if let Some(old) = Rc::get_mut(&mut slot) {
        *old = table;
        return slot;
    }
    Rc::new(table)
}

/// Recover cleared columns and keep the empty header inside the worker budget.
pub fn release<Word, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    table: &mut ResultColumns<Word, Name>,
) {
    table.rows = Count(0);
    for mut column in table.cols.drain(..) {
        let capacity = column.capacity();
        if capacity == 0
            || capacity > COLUMN_CELLS / 16
            || buffers.capacity.0 + capacity > COLUMN_CELLS
        {
            continue;
        }
        column.clear();
        let class = usize::try_from(usize::BITS - 1 - capacity.leading_zeros()).unwrap_or_default();
        if let Some(bucket) = buffers.columns.get_mut(class) {
            buffers.capacity.0 += capacity;
            bucket.push(column);
        }
    }
    let header = std::mem::take(&mut table.cols);
    if header.capacity() > 0 && header.capacity() <= 64 && buffers.headers.len() < HEADERS {
        buffers.headers.push(header);
    }
}

/// A consumer may still own the answer. Reuse it only after the final read.
pub fn retire<Word, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    mut answer: Rc<ResultColumns<Word, Name>>,
) {
    if let Some(table) = Rc::get_mut(&mut answer) {
        release(buffers, table);
        if buffers.shared.len() < HEADERS {
            buffers.shared.push(answer);
        }
    }
}

/// Take a uniquely owned, cleared row selection before the operator reads its sources.
pub fn take_selection<Row: Default>(
    buffers: &mut super::types::SelectionBuffers<Row>,
) -> Rc<Vec<Row>> {
    let mut value = buffers.retained.pop().unwrap_or_default();
    buffers.capacity.0 -= value.capacity();
    if let Some(rows) = Rc::get_mut(&mut value) {
        rows.clear();
    }
    value
}

/// Reuse a selection only when all readers have released it.
pub fn retire_selection<Row>(
    buffers: &mut super::types::SelectionBuffers<Row>,
    value: Rc<Vec<Row>>,
) {
    let capacity = value.capacity();
    if Rc::strong_count(&value) == 1
        && capacity > 0
        && buffers.capacity.0 + capacity <= super::constants::SELECTION_ROWS
        && buffers.retained.len() < HEADERS
    {
        buffers.capacity.0 += capacity;
        buffers.retained.push(value);
    }
}

/// Gather a borrowed sequence of input ordinals into columns sized for the result.
/// Repeated ordinals are retained. An invalid ordinal cannot publish a partial table.
pub fn gather_rows<Word: Copy, Name>(
    buffers: &mut TableBuffers<Word, Name>,
    table: &ResultColumns<Word, Name>,
    rows: impl ExactSizeIterator<Item = RowIndex> + Clone,
) -> Option<ResultColumns<Word, Name>> {
    let count = Count(rows.len());
    if table.cols.is_empty() && rows.clone().any(|row| row.0 >= table.rows.0) {
        return None;
    }
    let mut output = prepare(buffers, table.names.clone(), table.tys.clone(), count);
    for (column, target) in table.cols.iter().zip(&mut output.cols) {
        let column = column.get(..table.rows.0)?;
        for row in rows.clone() {
            target.push(*column.get(row.0)?);
        }
    }
    output.rows = count;
    Some(output)
}
