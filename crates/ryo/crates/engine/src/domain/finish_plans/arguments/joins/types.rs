//! The types of the `joins` part: what the operators over two inputs: `union`, `antijoin`, `as-of`, `ordered-match` are given. Plain data.

use crate::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::Side;

/// `union`: the rows of both inputs, each once.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::joins::types::UnionArguments;
///
/// let union = UnionArguments { left: StepNumber(0), right: StepNumber(1) };
/// assert_ne!(union.left, union.right);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnionArguments {
    /// The first input.
    pub left: StepNumber,
    /// The second input.
    pub right: StepNumber,
}

/// `antijoin`: the rows of the left input whose key no row of the right input has.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::joins::types::AntijoinArguments;
///
/// let antijoin = AntijoinArguments { left: StepNumber(0), right: StepNumber(1), left_on: Vec::new(), right_on: Vec::new() };
/// assert!(antijoin.left_on.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AntijoinArguments {
    /// The rows kept.
    pub left: StepNumber,
    /// The rows that remove them.
    pub right: StepNumber,
    /// The left key's columns.
    pub left_on: Vec<ColumnName>,
    /// The right key's columns, in the same order.
    pub right_on: Vec<ColumnName>,
}

/// `as-of`: each left row with the nearest right row of its key on the order the bound names.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::joins::types::{AsOfArguments, AsOfBound};
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let line = || ColumnName("line".into());
/// let as_of = AsOfArguments {
///     left: StepNumber(0), right: StepNumber(1), left_on: Vec::new(), right_on: Vec::new(),
///     left_order: line(), right_order: line(), bound: AsOfBound::AtOrBefore, output: Vec::new(),
/// };
/// assert_eq!(as_of.bound, AsOfBound::AtOrBefore);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AsOfArguments {
    /// The rows each answered once.
    pub left: StepNumber,
    /// The rows searched.
    pub right: StepNumber,
    /// The left key's columns.
    pub left_on: Vec<ColumnName>,
    /// The right key's columns.
    pub right_on: Vec<ColumnName>,
    /// The left order column.
    pub left_order: ColumnName,
    /// The right order column.
    pub right_order: ColumnName,
    /// Which right row is the nearest.
    pub bound: AsOfBound,
    /// The columns written, each from one side.
    pub output: Vec<OutputColumn>,
}

/// Which right row an `as-of` answers: the nearest at or before the left row's order, before it, at or after it, or
/// after it.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::joins::types::AsOfBound;
///
/// assert_ne!(AsOfBound::Before, AsOfBound::After);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AsOfBound {
    /// `at-or-before`.
    AtOrBefore,
    /// `before`.
    Before,
    /// `at-or-after`.
    AtOrAfter,
    /// `after`.
    After,
}

/// A column an ordered step writes: its name, and the side and column it is read from.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::joins::types::OutputColumn;
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::plan_parts::types::Side;
///
/// let id = OutputColumn { name: ColumnName("id".into()), side: Side::Left, column: ColumnName("id".into()) };
/// assert_eq!(id.side, Side::Left);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OutputColumn {
    /// The name it is written under.
    pub name: ColumnName,
    /// The input it is read from.
    pub side: Side,
    /// The column of that input.
    pub column: ColumnName,
}

/// `ordered-match`: left and right rows of one key paired in the order of their order columns.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::joins::types::OrderedMatchArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let ordinal = || ColumnName("ordinal".into());
/// let matched = OrderedMatchArguments {
///     left: StepNumber(0), right: StepNumber(1), left_on: Vec::new(), right_on: Vec::new(),
///     left_order: ordinal(), right_order: ordinal(), output: Vec::new(),
/// };
/// assert!(matched.output.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderedMatchArguments {
    /// The left rows.
    pub left: StepNumber,
    /// The right rows.
    pub right: StepNumber,
    /// The left key's columns.
    pub left_on: Vec<ColumnName>,
    /// The right key's columns.
    pub right_on: Vec<ColumnName>,
    /// The left order column.
    pub left_order: ColumnName,
    /// The right order column.
    pub right_order: ColumnName,
    /// The columns written, each from one side.
    pub output: Vec<OutputColumn>,
}
