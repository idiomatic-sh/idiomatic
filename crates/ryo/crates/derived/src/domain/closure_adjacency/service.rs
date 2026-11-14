//! One current closure adjacency. Partition scheduling stays outside this capability.
use super::constants::SOURCE;
use super::types::{ClosureCapacities, ClosurePartition};
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::RowCells;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// Builds the closure adjacency of one partition from its rows, reusing a recycled partition's buffers.
pub(crate) fn build(
    input: &dyn RowCells,
    positions: Option<&[RowIndex]>,
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
            .get(usize::from(source))
            .ok_or(DerivedFault::ShapeMismatch)?,
        *input
            .kinds()
            .get(usize::from(destination))
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
    for at in 0..positions.map_or(input.count().0, <[RowIndex]>::len) {
        let row = input
            .read_row(
                positions.map_or(RowIndex(at), |positions| positions[at]),
                buffer,
            )
            .ok_or(DerivedFault::ShapeMismatch)?;
        let from = *row
            .get(usize::from(source))
            .ok_or(DerivedFault::ShapeMismatch)?;
        let to = *row
            .get(usize::from(destination))
            .ok_or(DerivedFault::ShapeMismatch)?;
        part.table.cells.extend_from_slice(&[from, to]);
        part.table.count.0 += 1;
        part.sources.push(from);
    }
    part.index = crate::domain::equality::service::build(&part.table, &[SOURCE], Some(part.index))?;
    part.sources.sort_unstable();
    part.sources.dedup();
    Ok(part)
}

/// The capacity of the key and the sources of a partition.
#[must_use]
pub fn capacities(part: &ClosurePartition) -> ClosureCapacities {
    ClosureCapacities {
        key: ElementCapacity(part.key.capacity()),
        sources: ElementCapacity(part.sources.capacity()),
    }
}
