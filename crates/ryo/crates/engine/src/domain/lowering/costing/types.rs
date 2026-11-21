//! The types of the `costing` part: how a cell stands when its atom is placed, what an access of the atom then is, and what
//! an access is taken to cost.

use crate::domain::lowering::flattening::types::RowColumn;
use crate::domain::steps::types::{ColumnMask, CostClass, DefinitionId};
use ryo_values::domain::query_plans::types::NodeDigest;
use std::cell::RefCell;
use std::collections::HashMap;

/// An estimate of the planner, in 1,024ths: rows in a file or for a candidate, or steps. A fraction says an access leaves
/// less than a row of a candidate. It orders accesses and is never a bound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PlanEstimate(pub(crate) u64);

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
    pub(crate) cells: Vec<RowColumn>,
    /// What bounds the access: a fan-out with a register among its key, the file's rows otherwise.
    pub(crate) class: CostClass,
    /// The rows it gives.
    pub(crate) rows: PlanEstimate,
}

/// What an access is taken to cost where it stands in a unit's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AccessCost {
    /// The rows it gives for each row the unit holds when it runs.
    pub(crate) rows: PlanEstimate,
    /// The steps it takes for each such row.
    pub(crate) steps: PlanEstimate,
    /// The steps it takes once in a file, whatever the rows: a table it reads is made.
    pub(crate) once: PlanEstimate,
}

/// What a definition is asked by: the columns a use binds, ascending. None bound: every row of it, its table's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct UseAsked {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// The merged plan node the definition is the rows of.
    pub(crate) node: NodeDigest,
    /// The bound columns.
    pub(crate) bound: ColumnMask,
}

/// What each use asked so far costs, from the definition's own plan for the columns bound: a use is costed once however
/// many accesses ask it. None: the definition has no program for the columns, and the use reads its table.
#[derive(Debug, Default)]
pub(crate) struct UseCosts(pub(crate) RefCell<HashMap<UseAsked, Option<AccessCost>>>);

/// The uses costed for the pack a thread is lowering: a definition's branches never change once it is made, so what a use
/// of it costs is the pack's to keep, not one branch's. `drafts` and `definitions` say which pack the costs are of.
#[derive(Debug, Default)]
pub(crate) struct PackCosts {
    /// Where the pack's drafts are.
    pub(crate) drafts: usize,
    /// How many definitions the drafts held when the costs were last read.
    pub(crate) definitions: usize,
    /// The costs.
    pub(crate) costs: UseCosts,
}
