//! The plans of a selection that holds a column to one of a set of literals, each way a plan writes the set: a glob of
//! literal alternatives, a regular expression of them, an `in` list, a union of equalities; and the ones that are no set.

use super::parts::{plan, step};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, LiteralText, Operand, ParameterId, PlanComparison,
    QueryLiteral, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::Plan;

/// The rows of `relation` whose column at `column` passes `test` against `right`.
fn held(relation: Relation, column: u32, test: Test, right: Operand) -> PlanStep {
    step(Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![PlanComparison {
            test,
            left: Operand::Column(ColumnIndex(column)),
            right,
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    })
}

/// A text the query wrote.
fn written(text: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(text.into())))
}

/// The plans of the set scenes; none for another name. The kind is column 1 of the syntax tree and its text column 2; the
/// field is column 3 of the tree's edges.
pub(super) fn set_scene(scene: &str) -> Option<Plan> {
    let tree = Relation::SyntaxTree;
    Some(match scene {
        "the elements of two kinds, by a glob" => plan(vec![held(
            tree,
            1,
            Test::GlobAny,
            written("{sentence,word}"),
        )]),
        "the elements of two kinds, by an expression" => plan(vec![held(
            tree,
            1,
            Test::Matches,
            written("^(?:sentence|word)$"),
        )]),
        // The list is the rule's first parameter (`set_bindings`).
        "the elements of two kinds, by a list" => plan(vec![held(
            tree,
            1,
            Test::In,
            Operand::Parameter(ParameterId(0)),
        )]),
        "the elements of two kinds, by a union" => plan(vec![
            held(tree, 1, Test::Equal, written("sentence")),
            held(tree, 1, Test::Equal, written("word")),
            step(Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
            }),
        ]),
        "the edges in two fields, by a glob" => plan(vec![held(
            Relation::SyntaxChild,
            3,
            Test::GlobAny,
            written("{subject,object}"),
        )]),
        // One alternative holds a star: the glob matches more than literals.
        "the elements of a kind by a glob with a star" => plan(vec![held(
            tree,
            1,
            Test::GlobAny,
            written("{sentence,wor*}"),
        )]),
        // Each word beside the nearest element above it whose kind is `sentence` or `clause`: the set is in the filter
        // of a nearest axis's domain, which the walk reads in place as one read.
        "the nearest sentence or clause above each word" => plan(vec![
            super::parts::words(),
            step(Operation::Axis {
                from: Input::Step(StepId(0)),
                edges: Input::Relation(Relation::SyntaxChild),
                domain: Input::Relation(tree),
                direction: ryo_values::domain::plan_parts::types::Direction::NearestAncestor,
                id: ColumnIndex(1),
                parent: ColumnIndex(0),
                child: ColumnIndex(1),
                domain_id: ColumnIndex(0),
                domain_filter: vec![PlanComparison {
                    test: Test::GlobAny,
                    left: Operand::Column(ColumnIndex(1)),
                    right: written("{sentence,clause}"),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                domain_view: Vec::new(),
                output: Vec::new(),
            }),
        ]),
        // The text of an element is no kind and no field: no constant names a bucket of it by a small code.
        "the elements of two texts, by a glob" => {
            plan(vec![held(tree, 2, Test::GlobAny, written("{a,b}"))])
        }
        // The same list as the kinds', held against the text: a list has no operand of a test.
        "the elements of two texts, by a list" => plan(vec![held(
            tree,
            2,
            Test::In,
            Operand::Parameter(ParameterId(0)),
        )]),
        _ => return None,
    })
}

/// What the parameters of a set scene's rule are bound to: the list of the two kinds for the scenes that name a list,
/// nothing for every other scene.
#[cfg(test)]
pub(super) fn set_bindings(
    scene: &str,
) -> Vec<ryo_values::domain::query_bindings::types::ParameterBinding> {
    match scene {
        "the elements of two kinds, by a list" | "the elements of two texts, by a list" => {
            let kinds = ["sentence", "word"].map(|kind| LiteralText(kind.into()));
            vec![ryo_values::domain::query_bindings::types::ParameterBinding::Texts(kinds.into())]
        }
        _ => Vec::new(),
    }
}
