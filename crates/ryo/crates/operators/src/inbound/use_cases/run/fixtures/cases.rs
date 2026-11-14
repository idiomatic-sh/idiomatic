//! The rows and steps the run use case's cases are made of: the relation they read, its rows, and the selects and comparisons
//! over it.
use super::TextResources;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Offset, Position, Relation, Span, Value};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, LiteralText, Operand, PlanComparison,
    QueryLiteral, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the use cases read: its refinement (column 2) and name (column 3) are texts.
pub(crate) const DEFINES: Relation = Relation::Defines;

/// The rows of `DEFINES`, one per (refinement, name) pair, each a node of file 0, with `texts` the file's symbols.
pub(crate) fn defines(pairs: &[(&str, &str)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(DEFINES).len());
    for (at, (refinement, name)) in pairs.iter().enumerate() {
        let node = Value::Node(node_id(
            FileId(0),
            Position(u32::try_from(at).unwrap_or(u32::MAX)),
        ));
        cells.extend([
            node,
            texts.symbol("kind"),
            texts.symbol(refinement),
            texts.symbol(name),
            node,
            Value::File(FileId(0)),
            Value::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
        ]);
    }
    Table {
        kinds: kinds(DEFINES).to_vec(),
        cells,
        count: RowCount(pairs.len()),
    }
}

/// A comparison of two operands.
pub(crate) fn compared(test: Test, left: Operand, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left,
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// Whether a column's text matches the pattern its own text is: a regex made from the row, which fails on a text that is no
/// pattern.
pub(crate) fn self_matching(column: u32) -> PlanComparison {
    compared(
        Test::Matches,
        Operand::Column(ColumnIndex(column)),
        Operand::Column(ColumnIndex(column)),
    )
}

/// Whether the name is `text`.
pub(crate) fn named(text: &str) -> PlanComparison {
    compared(
        Test::Equal,
        Operand::Column(ColumnIndex(3)),
        Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    )
}

/// A select of every column of what `from` reads, kept where `comparisons` hold.
pub(crate) fn select(from: PlanInput, comparisons: Vec<PlanComparison>) -> Step {
    Step {
        operation: Operation::Select {
            from: input_of(from),
            comparisons,
            kept: Vec::new(),
        },
        inputs: vec![from],
    }
}

/// The plan's input as the operation names it.
pub(crate) fn input_of(from: PlanInput) -> StepInput {
    match from {
        PlanInput::Node(PlanNodeId(at)) => {
            StepInput::Step(ryo_values::domain::plan_parts::types::StepId(at))
        }
        PlanInput::Relation(relation) => StepInput::Relation(relation),
        PlanInput::Bound(_) | PlanInput::Observation(_) => StepInput::BoundObservation(
            ryo_values::domain::plan_parts::types::ObservationBindingId(0),
        ),
    }
}
