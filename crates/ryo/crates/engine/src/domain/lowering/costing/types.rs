//! The types of the `costing` part: how a cell stands when its atom is placed, and what an access of the atom then is.

use crate::domain::lowering::definitions::types::RowEstimate;
use crate::domain::lowering::flattening::types::ColumnPlace;
use crate::domain::steps::types::CostClass;

/// How a cell of an atom stands when the atom is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellState {
    /// A variable some earlier access gave: a register's value.
    Register,
    /// A constant of the query.
    Constant,
    /// A variable nothing gave yet: the access gives it.
    Open,
    /// The scanned file: never a key.
    File,
}

/// The access of an atom's rows chosen for the cells that stand bound: the cells it is keyed by, what bounds it, and the rows
/// it is taken to give each time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyChoice {
    /// The cells that key the access, by place; none: every row of the source.
    pub(crate) cells: Vec<ColumnPlace>,
    /// What bounds the access: a fan-out with a register among its key, the file's rows otherwise.
    pub(crate) class: CostClass,
    /// The rows it gives.
    pub(crate) rows: RowEstimate,
}
