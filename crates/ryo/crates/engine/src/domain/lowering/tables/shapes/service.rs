//! The services of the `shapes` part: what a row of a step holds, kind by kind, and whether a comparison or a join can be
//! written over it. Pure functions of shapes, read by the admission of a rule.

use crate::domain::lowering::flattening::service::function_call;
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::tables::sets::service::literals;
use crate::domain::lowering::types::SelectedRule;
use crate::domain::steps::calls::types::{SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::types::{TEXTS, TestOperation};
use ryo_relations::domain::relations::types::{CellKind, Count};
use ryo_values::domain::plan_parts::types::CapturePattern;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, Operand, PlanComparison, QueryLiteral, Side, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};

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
        Test::NumericGreaterThanOrEqual => TestOperation::NumericAtLeast(TEXTS),
        Test::IntegerSuccessor => TestOperation::Successor(TEXTS),
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
        // A text held to a set of literals is written as pins where it is a node's kind or field, whatever states the
        // set: a list a parameter is bound to has no operand, and is read as its literals.
        let listed = operand_kind(rule, shape, &comparison.left) == Some(CellKind::Symbol)
            && literals(rule, comparison).is_some();
        listed
            || (test
                && operand_kind(rule, shape, &comparison.left).is_some()
                && operand_kind(rule, shape, &comparison.right).is_some())
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

/// The search a `source-matches` operation is: whether it numbers lines (it holds a base line), and the group it asks of
/// each occurrence. None for any other operation, and for a group's line asked of lines that are not numbered, which the
/// contract states no call for.
#[must_use]
pub(crate) fn source_search(operation: &Operation) -> Option<SourceSearch> {
    let Operation::SourceMatches {
        base_line,
        capture,
        capture_text,
        capture_line,
        ..
    } = operation
    else {
        return None;
    };
    let lines = match base_line {
        Some(_) => SearchLines::Numbered,
        None => SearchLines::Unnumbered,
    };
    let group = match capture {
        Some(_) => SearchGroup::Named {
            text: *capture_text,
            line: *capture_line,
        },
        None => SearchGroup::Whole,
    };
    let unstated = lines == SearchLines::Unnumbered
        && matches!(
            group,
            SearchGroup::Named {
                line: LineOutput::Appended,
                ..
            }
        );
    (!unstated).then_some(SourceSearch { lines, group })
}

/// The cells a search gives for each occurrence, in the contract's order: the occurrence's span; its line, when lines are
/// numbered; then for a named group its span, its text and its line as asked.
#[must_use]
pub(crate) fn search_kinds(search: SourceSearch) -> Vec<CellKind> {
    let mut kinds = vec![CellKind::Span];
    if search.lines == SearchLines::Numbered {
        kinds.push(CellKind::Number);
    }
    if let SearchGroup::Named { text, line } = search.group {
        kinds.push(CellKind::Span);
        if text == TextOutput::Appended {
            kinds.push(CellKind::Symbol);
        }
        if line == LineOutput::Appended {
            kinds.push(CellKind::Number);
        }
    }
    kinds
}

/// How many texts a capture gives: one for each slot of its pattern, and a slot opens with `{`. A COPY of the operators
/// crate's `capture::service::slot_count`, which the engine crate cannot call (it does not depend on that crate): to be
/// routed to the one definition when it is where the engine can reach it (build/l1/QUESTIONS.md Q10).
#[must_use]
pub(crate) fn capture_slots(pattern: &CapturePattern) -> Count {
    Count(pattern.0.bytes().filter(|byte| *byte == b'{').count())
}
