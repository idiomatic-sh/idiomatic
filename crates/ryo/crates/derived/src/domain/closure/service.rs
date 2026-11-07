//! One current closure adjacency. Partition scheduling stays outside this capability.
use crate::domain::contract::types::{ClosurePartition, DerivedFault, RowCells};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn build(
    input: &dyn RowCells,
    positions: Option<&[ryo_values::domain::rows::types::RowIndex]>,
    key: &[Value],
    source: ColumnIndex,
    destination: ColumnIndex,
    recycled: Option<ClosurePartition>,
    buffer: &mut Vec<Value>,
) -> Result<ClosurePartition, DerivedFault> {
    for column in [source, destination] {
        if usize::try_from(column.0).map_or(true, |at| at >= input.kinds().len()) {
            return Err(DerivedFault::NoSuchColumn { column });
        }
    }
    let kinds = vec![
        *input
            .kinds()
            .get(source.0 as usize)
            .ok_or(DerivedFault::ShapeMismatch)?,
        *input
            .kinds()
            .get(destination.0 as usize)
            .ok_or(DerivedFault::ShapeMismatch)?,
    ];
    let mut part = recycled.unwrap_or_default();
    part.key.clear();
    part.key.extend_from_slice(key);
    part.table.kinds.clear();
    part.table.kinds.extend(kinds);
    part.table.cells.clear();
    part.table.count.0 = 0;
    part.sources.clear();
    for at in 0..positions.map_or(
        input.count(),
        <[ryo_values::domain::rows::types::RowIndex]>::len,
    ) {
        let row = input
            .read_row(positions.map_or(at, |positions| positions[at].0), buffer)
            .ok_or(DerivedFault::ShapeMismatch)?;
        let from = *row
            .get(source.0 as usize)
            .ok_or(DerivedFault::ShapeMismatch)?;
        let to = *row
            .get(destination.0 as usize)
            .ok_or(DerivedFault::ShapeMismatch)?;
        part.table.cells.extend_from_slice(&[from, to]);
        part.table.count.0 += 1;
        part.sources.push(from);
    }
    part.index =
        crate::domain::equality::service::build(&part.table, &[ColumnIndex(0)], Some(part.index))?;
    part.sources.sort_unstable();
    part.sources.dedup();
    Ok(part)
}
