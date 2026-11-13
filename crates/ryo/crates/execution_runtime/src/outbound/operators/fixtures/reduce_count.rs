//! The table and group output the operators contract counts through `ReduceCount`, the call the engine makes.
use ryo_operators::domain::reduction::types::{GroupDigest, GroupOutput};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as CellValue};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The groups a reduction wrote: each group's first row and count, digests ignored.
#[derive(Debug, Default)]
pub struct Groups(pub Vec<(usize, usize)>);

impl GroupOutput for Groups {
    fn clear(&mut self) {
        self.0.clear();
    }
    fn push(&mut self, first: RowIndex, counted: RowCount, _digest: GroupDigest) {
        self.0.push((first.0, counted.0));
    }
    fn sort_first(&mut self) {
        self.0.sort_unstable();
    }
}

/// A table of two number columns holding `rows`.
#[must_use]
pub fn table(rows: &[[i64; 2]]) -> Table {
    Table {
        kinds: vec![CellKind::Number; 2],
        cells: rows
            .iter()
            .flatten()
            .map(|&number| CellValue::Number(Integer(number)))
            .collect(),
        count: RowCount(rows.len()),
    }
}
