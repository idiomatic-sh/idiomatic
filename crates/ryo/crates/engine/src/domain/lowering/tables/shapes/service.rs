//! The services of the `shapes` part: what a row of a step holds, kind by kind, and whether a comparison or a join can be
//! written over it. Pure functions of shapes, read by the admission of a rule.

use crate::domain::lowering::flattening::service::function_call;
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::types::SelectedRule;
use crate::domain::steps::types::TestOperation;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, Operand, PlanComparison, QueryLiteral, Side, Test,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The comparison a plan's test is, as a step's or a stage's test; `Matches` takes its expression's number when written.
/// None for a test no step makes.
#[must_use]
pub(crate) const fn test_of(test: Test) -> Option<TestOperation> {
    Some(match test {
        Test::Equal => TestOperation::Equal,
        Test::NotEqual => TestOperation::NotEqual,
        Test::LessThan => TestOperation::Less,
        Test::LessThanOrEqual => TestOperation::LessOrEqual,
        Test::GreaterThan => TestOperation::Greater,
        Test::GreaterThanOrEqual => TestOperation::GreaterOrEqual,
        Test::In => TestOperation::In,
        Test::NotIn => TestOperation::NotIn,
        Test::GlobAny => TestOperation::Glob,
        Test::NotGlobAny => TestOperation::NotGlob,
        Test::StartsWith => TestOperation::StartsWith,
        Test::EndsWith => TestOperation::EndsWith,
        Test::Matches => TestOperation::Matches(crate::domain::steps::types::RegexId(0)),
        Test::NumericGreaterThanOrEqual => TestOperation::NumericAtLeast,
        Test::IntegerSuccessor => TestOperation::Successor,
        Test::NaturalLessThan => TestOperation::NaturalLess,
        Test::FloatTextGreaterThanOrEqual => return None,
    })
}

/// What an operand holds, over a row of `shape`: a column's kind, a literal's, a bound parameter's. None for an operand the
/// lowering writes no operand for: a function applied, a parameter bound to a list, a column past the row.
pub(crate) fn operand_kind(
    rule: &SelectedRule<'_>,
    shape: &StepShape,
    operand: &Operand,
) -> Option<CellKind> {
    match operand {
        Operand::Column(column) => shape.0.get(usize::from(*column)).copied(),
        Operand::Literal(QueryLiteral::Number(_)) => Some(CellKind::Number),
        Operand::Literal(QueryLiteral::Text(_)) => Some(CellKind::Symbol),
        Operand::Parameter(parameter) => {
            let place = usize::try_from(parameter.0).unwrap_or(usize::MAX);
            match rule.bindings.0.get(place)? {
                ParameterBinding::Number(_) => Some(CellKind::Number),
                ParameterBinding::Text(_) => Some(CellKind::Symbol),
                ParameterBinding::Flag(_)
                | ParameterBinding::Numbers(_)
                | ParameterBinding::Texts(_)
                | ParameterBinding::TextSets(_)
                | ParameterBinding::Tables(_) => None,
            }
        }
        // A function is text made from texts, when the engine's contract names its meaning.
        Operand::Function(applied) => {
            let _meaning = function_call(applied.function)?;
            let mut arguments = applied.arguments.iter();
            arguments
                .all(|argument| operand_kind(rule, shape, argument) == Some(CellKind::Symbol))
                .then_some(CellKind::Symbol)
        }
    }
}

/// The shape, when every comparison can be written over a row of it; none when one cannot.
pub(crate) fn comparable<'s>(
    rule: &SelectedRule<'_>,
    shape: &'s StepShape,
    comparisons: &[PlanComparison],
) -> Option<&'s StepShape> {
    let mut each = comparisons.iter();
    let written = each.all(|comparison| {
        let test = match (comparison.test, &comparison.right) {
            (Test::Matches, Operand::Literal(QueryLiteral::Text(_))) => true,
            (Test::Matches, _) => false,
            (test, _) => test_of(test).is_some(),
        };
        test && operand_kind(rule, shape, &comparison.left).is_some()
            && operand_kind(rule, shape, &comparison.right).is_some()
    });
    written.then_some(shape)
}

/// The kinds of a row at the given columns; none when a column is past the row.
pub(crate) fn picked(shape: &StepShape, columns: &[ColumnIndex]) -> Option<StepShape> {
    let kinds: Option<Vec<CellKind>> = columns
        .iter()
        .map(|column| shape.0.get(usize::from(*column)).copied())
        .collect();
    kinds.map(StepShape)
}

/// A side's shape as its view shows it: the columns the view names, in order; all of them when it names none.
pub(crate) fn viewed(shape: &StepShape, view: &[ColumnIndex]) -> Option<StepShape> {
    if view.is_empty() {
        Some(shape.clone())
    } else {
        picked(shape, view)
    }
}

/// The kinds of a join's answer.
pub(crate) fn joined(
    left: &StepShape,
    right: &StepShape,
    output: &[JoinColumn],
) -> Option<StepShape> {
    let kinds: Option<Vec<CellKind>> = output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            side.0.get(usize::from(column.column)).copied()
        })
        .collect();
    kinds.map(StepShape)
}

/// The left side's shape, when each key column is within its side's row and the two sides have as many keys.
pub(crate) fn keyed<'s>(
    left: &'s StepShape,
    right: &StepShape,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
) -> Option<&'s StepShape> {
    let held = left_on.len() == right_on.len()
        && picked(left, left_on).is_some()
        && picked(right, right_on).is_some();
    held.then_some(left)
}
