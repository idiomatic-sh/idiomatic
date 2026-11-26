//! The types of the `physical_plan` concept the table kernels use: rows kept once, rows keyed by the hash of their key, and the
//! place in a table's cells where the row being written starts.

pub(crate) mod keyed_rows;
pub(crate) mod row_set;

/// The place in a table's cells where the row being written starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CellStart(pub(crate) usize);
