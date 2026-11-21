//! The types of the `costing` part: how a cell stands when its atom is placed, what an access of the atom then is, and what
//! an access is taken to cost.

use crate::domain::lowering::flattening::types::RowColumn;
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::{ColumnMask, CostClass, DefinitionId};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::query_plans::types::NodeDigest;
use std::collections::HashMap;
use std::sync::Mutex;

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
pub(crate) struct UseCosts(pub(crate) Mutex<HashMap<UseAsked, Option<AccessCost>>>);

/// What a sample measured the buckets of a file at, with the language's names the sizes are read by: the costing sizes a
/// kind's, a field's or a literal's bucket by it where the sample sized one, and by the fixed prior where it did not.
#[derive(Clone, Debug)]
pub(crate) struct SampleSizes {
    /// The statistics the load was given.
    pub(crate) statistics: Statistics,
    /// The language's kinds and fields.
    pub(crate) names: KindVocabulary,
}
