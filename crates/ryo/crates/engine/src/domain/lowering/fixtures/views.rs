//! The plans of a step that is one read of a relation with its pins, read by several steps or by two rules; and of the
//! steps that are no such read (a test that is no pin, a fill's rows), which stay definitions.

use super::parts::{at, joined, kept, of, plan, projected, step};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, Input, LiteralText, Operand, PlanComparison, QueryLiteral, RepeatedRows,
    Side, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::Plan;

/// A comparison of a column with a text the query wrote.
fn against(test: Test, column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(at(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The rows of `from` every comparison holds of.
fn where_all(from: Input, comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: Vec::new(),
    })
}

/// The elements whose kind is not `comment`: a test that is no pin, so the step is no view of the tree.
fn uncommented() -> PlanStep {
    where_all(
        Input::Relation(Relation::SyntaxTree),
        vec![against(Test::NotEqual, 1, "comment")],
    )
}

/// The plans of the view scenes; none for another name.
pub(super) fn view_scene(scene: &str) -> Option<Plan> {
    let tree = Input::Relation(Relation::SyntaxTree);
    Some(match scene {
        "the elements that are no comment" => plan(vec![uncommented()]),
        // The scene `the elements two projections read`, its first step held by a test that is no pin.
        "the elements that are no comment, two projections read" => plan(vec![
            uncommented(),
            projected(0, &[0, 1]),
            projected(0, &[0, 4]),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[0], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 1)],
            ),
        ]),
        // The scene `the elements a projection and a whole read`, its first step held by a test that is no pin.
        "the elements that are no comment, a projection and a whole read" => plan(vec![
            uncommented(),
            projected(0, &[0, 1]),
            kept(Input::Step(StepId(0))),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[0], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 4)],
            ),
        ]),
        // The child edges whose field is `label` and that are not their parent's first: a pin, then a test that is none.
        "the edges of the field label, past the first" => plan(vec![where_all(
            Input::Relation(Relation::SyntaxChild),
            vec![
                against(Test::Equal, 3, "label"),
                PlanComparison {
                    test: Test::NotEqual,
                    left: Operand::Column(at(2)),
                    right: Operand::Literal(QueryLiteral::Number(Integer(0))),
                    origin: ComparisonOrigin::Unrecorded,
                },
            ],
        )]),
        // The words (the elements of kind `word`), each beside its edge and beside its span: a pinned read of the tree
        // two steps read, one by the element an edge gives.
        "the words, beside their edges and their spans" => plan(vec![
            where_all(tree, vec![against(Test::Equal, 1, "word")]),
            joined(
                Input::Relation(Relation::SyntaxChild),
                Input::Step(StepId(0)),
                (&[1], &[0]),
                vec![of(Side::Left, 0), of(Side::Right, 0), of(Side::Right, 2)],
            ),
            projected(0, &[0, 4]),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[1], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 2), of(Side::Right, 1)],
            ),
        ]),
        // The elements projected to element, kind and file; counted by kind; each element beside the count of its kind.
        // The count's rows are a fill's: it reads the projection as a definition. The join reads it in place.
        "the elements beside the count of their kind" => plan(vec![
            step(Operation::Project {
                from: tree,
                columns: vec![at(0), at(1), at(3)],
                distinct: RepeatedRows::Dropped,
            }),
            step(Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![at(1), at(2)],
                distinct: None,
            }),
            joined(
                Input::Step(StepId(0)),
                Input::Step(StepId(1)),
                (&[1], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 2)],
            ),
        ]),
        // F401's steps 195 to 197 in small: a projection of the tree several steps read, one of them holding its kind to
        // a glob of literals, another to one kind.
        "the sentences and words of a projection, and its comments" => plan(vec![
            step(Operation::Project {
                from: tree,
                columns: vec![at(0), at(1), at(3)],
                distinct: RepeatedRows::Dropped,
            }),
            where_all(
                Input::Step(StepId(0)),
                vec![against(Test::GlobAny, 1, "{sentence,word}")],
            ),
            where_all(
                Input::Step(StepId(0)),
                vec![against(Test::Equal, 1, "comment")],
            ),
            step(Operation::Union {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(2)),
            }),
        ]),
        _ => return None,
    })
}
