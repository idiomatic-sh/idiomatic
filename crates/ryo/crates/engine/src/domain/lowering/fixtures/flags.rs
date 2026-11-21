//! The plans of a selection that holds a flag column to the word a flag is shown as. `literal-properties` holds three
//! flags: `zero` (its third column), `empty` and `boolean`.

use super::parts::{plan, step};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::Plan;

/// A text the query wrote.
fn written(text: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(text.into())))
}

/// A comparison of two operands.
fn held(test: Test, left: Operand, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left,
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The literals every comparison holds of.
fn literals(comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from: Input::Relation(Relation::LiteralProperties),
        comparisons,
        kept: Vec::new(),
    })
}

/// The plans of the flag scenes; none for another name.
pub(super) fn flag_scene(scene: &str) -> Option<Plan> {
    let column = |place| Operand::Column(ColumnIndex(place));
    Some(match scene {
        // `where kind != "float" and zero = "true"`: the first comparison is no pin, so two rules that emit the step
        // read its table, and its emptiness is said.
        "the integer literals that are zero" => plan(vec![literals(vec![
            held(Test::NotEqual, column(1), written("float")),
            held(Test::Equal, column(2), written("true")),
        ])]),
        "the literals that are not empty" => plan(vec![literals(vec![
            held(Test::NotEqual, column(1), written("float")),
            held(Test::Equal, column(3), written("false")),
        ])]),
        // In each of the next two the text `float` is the pack's first text, so the word of the text `true` would be
        // its second: no row here can read the same for the text's word and the raised flag's cell.
        "the literals that are not a boolean" => plan(vec![literals(vec![
            held(Test::NotEqual, column(1), written("float")),
            held(Test::NotEqual, column(4), written("true")),
        ])]),
        "the literals that are zero, the word first" => plan(vec![literals(vec![
            held(Test::NotEqual, column(1), written("float")),
            held(Test::Equal, written("true"), column(2)),
        ])]),
        _ => return None,
    })
}
