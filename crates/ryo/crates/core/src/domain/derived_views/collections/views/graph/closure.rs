//! One current closure adjacency. Partition scheduling and the ordered cursor stay with the application.
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::{graph::ClosurePartition, views::BorrowedRows},
    table_model::{
        service::{check_columns, index, push_row},
        types::{KeyIndex, On},
    },
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(super) fn prepare(
    input: BorrowedRows<'_>,
    positions: Option<&[RowIndex]>,
    key: &[Value],
    source: ColumnIndex,
    destination: ColumnIndex,
    previous: Option<ClosurePartition>,
    buffer: &mut Vec<Value>,
) -> Result<ClosurePartition, OperatingError> {
    check_columns(input.kinds(), &[source, destination])?;
    let kinds = vec![
        *input
            .kinds()
            .get(source.0 as usize)
            .ok_or(OperatingError::ShapeMismatch)?,
        *input
            .kinds()
            .get(destination.0 as usize)
            .ok_or(OperatingError::ShapeMismatch)?,
    ];
    let mut part = previous.unwrap_or_else(|| ClosurePartition {
        key: Vec::new(),
        table: Table::default(),
        index: KeyIndex::default(),
        sources: Vec::new(),
    });
    part.key.clear();
    part.key.extend_from_slice(key);
    part.table.kinds.clear();
    part.table.kinds.extend(kinds);
    part.table.cells.clear();
    part.table.count.0 = 0;
    part.sources.clear();
    for at in 0..positions.map_or(input.count(), <[RowIndex]>::len) {
        let row = input
            .read(
                positions.map_or(RowIndex(at), |positions| positions[at]),
                buffer,
            )
            .ok_or(OperatingError::ShapeMismatch)?;
        let from = *row
            .get(source.0 as usize)
            .ok_or(OperatingError::ShapeMismatch)?;
        let to = *row
            .get(destination.0 as usize)
            .ok_or(OperatingError::ShapeMismatch)?;
        push_row(&mut part.table, &[from, to]);
        part.sources.push(from);
    }
    index(&part.table, On::Columns(&[ColumnIndex(0)]), &mut part.index)?;
    part.sources.sort_unstable();
    part.sources.dedup();
    Ok(part)
}

use ryo_values::domain::rows::types::{RowIndex, Table};
