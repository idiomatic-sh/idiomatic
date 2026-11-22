//! The types of the `walks` part: what the operators that walk a tree or an edge relation: `axis`, `closure`, `follow`, `ordered-dfs` are given. Plain data.

use crate::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::Direction;

/// `axis`: each start with the relatives the tree relation's direction reaches, kept to those of the domain.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::walks::types::AxisArguments;
/// use ryo_values::domain::plan_parts::types::Direction;
///
/// let axis = AxisArguments { from: StepNumber(2), relation: StepNumber(0), direction: Direction::Ancestor, domain: StepNumber(1) };
/// assert_eq!(axis.direction, Direction::Ancestor);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AxisArguments {
    /// The starts.
    pub from: StepNumber,
    /// The tree relation walked.
    pub relation: StepNumber,
    /// Which way.
    pub direction: Direction,
    /// The relatives admitted.
    pub domain: StepNumber,
}

/// `closure` and `follow`: the pairs an edge relation's source and destination columns join, transitively (`closure`) or
/// along one key (`follow`).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::walks::types::{EdgeArguments, EdgeKeys};
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let name = |text: &str| ColumnName(text.into());
/// let edges = EdgeArguments { edges: StepNumber(0), source: name("parent"), destination: name("child"), keys: EdgeKeys::Unwritten };
/// assert_eq!(edges.keys, EdgeKeys::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EdgeArguments {
    /// The edges.
    pub edges: StepNumber,
    /// The source column.
    pub source: ColumnName,
    /// The destination column.
    pub destination: ColumnName,
    /// The columns every pair of one walk shares.
    pub keys: EdgeKeys,
}

/// The key columns of an edge walk: written, or not.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::walks::types::EdgeKeys;
///
/// assert_ne!(EdgeKeys::Written(Vec::new()), EdgeKeys::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EdgeKeys {
    /// The query names none.
    Unwritten,
    /// The query names these.
    Written(Vec<ColumnName>),
}

/// `ordered-dfs`: a depth-first walk of an edge relation from each root, in the order column's order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::walks::types::OrderedDfsArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let name = |text: &str| ColumnName(text.into());
/// let walk = OrderedDfsArguments {
///     from: StepNumber(0), root: name("root"), edges: StepNumber(1), source: name("parent"), destination: name("child"),
///     order: name("ordinal"), edge: name("edge"), keys: Vec::new(), output: name("visit"),
/// };
/// assert_eq!(&*walk.output.0, "visit");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderedDfsArguments {
    /// The roots.
    pub from: StepNumber,
    /// The root column.
    pub root: ColumnName,
    /// The edges.
    pub edges: StepNumber,
    /// The source column.
    pub source: ColumnName,
    /// The destination column.
    pub destination: ColumnName,
    /// The order column.
    pub order: ColumnName,
    /// The edge column.
    pub edge: ColumnName,
    /// The key columns.
    pub keys: Vec<ColumnName>,
    /// The column the visit's place is written to.
    pub output: ColumnName,
}
