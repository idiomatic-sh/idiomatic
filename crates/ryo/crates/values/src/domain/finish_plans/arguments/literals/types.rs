//! The types of the `literals` part: what the operators given constants: `extend`, `rows`, and an operator outside the step set are given. Plain data.

use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::expressions::types::PlanConstant;
use crate::domain::descriptions::types::{ColumnName, OperatorName};
use crate::domain::plan_parts::types::LiteralText;

/// `extend`: every input row with constant columns added.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_values::domain::finish_plans::arguments::literals::types::ExtendArguments;
///
/// assert!(ExtendArguments { from: StepNumber(0), values: Vec::new() }.values.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExtendArguments {
    /// The rows extended.
    pub from: StepNumber,
    /// The columns added, in the order the query wrote them.
    pub values: Vec<ExtendedValue>,
}

/// One constant column an `extend` adds.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::literals::types::ExtendedValue;
/// use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let code = ExtendedValue { name: ColumnName("code".into()), value: PlanConstant::Absent };
/// assert_eq!(&*code.name.0, "code");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExtendedValue {
    /// The column's name.
    pub name: ColumnName,
    /// Its value on every row.
    pub value: PlanConstant,
}

/// `rows`: a constant table.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::literals::types::RowsArguments;
///
/// assert!(RowsArguments { rows: Vec::new(), columns: Vec::new() }.rows.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowsArguments {
    /// The rows, each its cells in column order.
    pub rows: Vec<ConstantRow>,
    /// The columns.
    pub columns: Vec<ColumnName>,
}

/// One row of a constant table.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::literals::types::ConstantRow;
///
/// assert!(ConstantRow { cells: Vec::new() }.cells.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstantRow {
    /// Its cells.
    pub cells: Vec<LiteralText>,
}

/// A call of an operator outside the step set, kept as written: the executor refuses it when a run reaches it.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::literals::types::UnlistedArguments;
/// use ryo_values::domain::descriptions::types::OperatorName;
///
/// let unlisted = UnlistedArguments { operator: OperatorName("undefined".into()), arguments: Vec::new() };
/// assert!(unlisted.arguments.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnlistedArguments {
    /// The operator named.
    pub operator: OperatorName,
    /// Its arguments, in the order the query wrote them.
    pub arguments: Vec<UnlistedArgument>,
}

/// One argument of an unlisted operator.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_values::domain::finish_plans::arguments::literals::types::{ArgumentValue, StepArgumentName, UnlistedArgument};
///
/// let from = UnlistedArgument { name: StepArgumentName("from".into()), value: ArgumentValue::Step(StepNumber(0)) };
/// assert_eq!(from.value, ArgumentValue::Step(StepNumber(0)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnlistedArgument {
    /// Its name.
    pub name: StepArgumentName,
    /// Its value.
    pub value: ArgumentValue,
}

/// The name of an operator's argument, as the query writes it: `from`, `left-on`.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::literals::types::StepArgumentName;
///
/// assert_eq!(&*StepArgumentName("left-on".into()).0, "left-on");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepArgumentName(pub Box<str>);

/// What an argument holds: a step of the plan (a relation the operator reads), or a constant.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::literals::types::ArgumentValue;
/// use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
///
/// assert!(matches!(ArgumentValue::Constant(PlanConstant::Absent), ArgumentValue::Constant(_)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArgumentValue {
    /// A step of the plan.
    Step(StepNumber),
    /// A constant.
    Constant(PlanConstant),
}

