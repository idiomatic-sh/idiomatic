//! The plans of the shapes the lowering's fifth step names, each the smallest of its shape: a step only projections read, and
//! the operations whose side is a relation where the scenes before them gave a step.

use super::parts::{at, joined, kept, nearest_step, of, plan, projected, step, words};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Direction, Input, LiteralText, Operand, PlanComparison,
    QueryLiteral, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_plans::types::Plan;

/// The plans of the fifth step's shapes; none for another name.
pub(super) fn shape_scene(scene: &str) -> Option<Plan> {
    Some(match scene {
        // The elements, read by two projections alone: one keeps the element and its kind, the other the element and its
        // span. No reader asks the text or the parent, so the step's definition holds three of its five columns.
        "the elements two projections read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            projected(0, &[0, 1]),
            projected(0, &[0, 4]),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[0], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 1)],
            ),
        ]),
        // The same elements, read by a projection and by a step that keeps every row whole: the definition holds every
        // column, since one reader reads the whole row.
        "the elements a projection and a whole read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            projected(0, &[0, 1]),
            kept(Input::Step(StepId(0))),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[0], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 4)],
            ),
        ]),
        // The nearest element above each element, read by one projection that keeps what is reached alone. The walk's own
        // pattern names the start by its column, so the definition holds every column though its reader keeps one.
        "the nearest elements above, projected" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            nearest_step(Input::Relation(Relation::SyntaxChild)),
            projected(1, &[5]),
        ]),
        // The words no link starts at: an antijoin whose right side is a relation's own rows, asked by the word.
        "the words no link starts at" => plan(vec![
            words(),
            step(Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::TokenLink),
                left_on: vec![ColumnIndex(1)],
                right_on: vec![ColumnIndex(0)],
            }),
        ]),
        // Each word beside the nearest element above it whose kind is `sentence`: a nearest axis whose domain is a
        // relation's own rows with a filter.
        "the nearest sentence above each word" => plan(vec![
            words(),
            step(Operation::Axis {
                from: Input::Step(StepId(0)),
                edges: Input::Relation(Relation::SyntaxChild),
                domain: Input::Relation(Relation::SyntaxTree),
                direction: Direction::NearestAncestor,
                id: at(1),
                parent: at(0),
                child: at(1),
                domain_id: at(0),
                domain_filter: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(at(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("sentence".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                domain_view: Vec::new(),
                output: Vec::new(),
            }),
        ]),
        // The count of the words of each text beside the words of that text, in the files named `*.pyi`: a test of the
        // file in a table whose cheapest scan, the group table, holds no file column.
        "the words counted by text, in stub files" => plan(vec![
            words(),
            step(Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![at(3)],
                distinct: None,
            }),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(0)),
                (&[0], &[3]),
                vec![
                    of(Side::Left, 0),
                    of(Side::Left, 1),
                    of(Side::Right, 1),
                    of(Side::Right, 4),
                ],
            ),
            step(Operation::Select {
                from: Input::Step(StepId(2)),
                comparisons: vec![PlanComparison {
                    test: Test::GlobAny,
                    left: Operand::Column(at(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("*.pyi".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
        ]),
        // Each element whose text, every `a` of it replaced by `b`, is the value of an import's string literal: a join
        // keyed by a cell a call gives. Scanned from the literals no order has a key; scanned from the elements every
        // access after the scan is keyed.
        "the elements whose replaced text an import names" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            step(Operation::Replace {
                from: Input::Step(StepId(0)),
                text: at(2),
                pattern: LiteralText("a".into()),
                replacement: LiteralText("b".into()),
            }),
            step(Operation::Select {
                from: Input::Relation(Relation::StringLiteral),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(at(5)),
                    right: Operand::Literal(QueryLiteral::Number(Integer(1))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[5], &[2]),
                vec![of(Side::Left, 0), of(Side::Right, 0), of(Side::Left, 3)],
            ),
        ]),
        // Each word beside the element just after it under its sentence.
        "the word after each word" => plan(vec![
            words(),
            axis_of(
                Direction::NextSibling,
                Relation::SyntaxChild,
                Input::Relation(Relation::SyntaxTree),
            ),
        ]),
        // Each scope edge beside every scope above its child, among the children of the edges: an ancestor axis over
        // edges that are no tree of a node relation.
        "the scopes above each scope" => plan(vec![
            kept(Input::Relation(Relation::Scope)),
            axis_of(Direction::Ancestor, Relation::Scope, Input::Step(StepId(0))),
        ]),
        // The links of each kind in a file, counted by their distinct sources: a group count over a relation's own rows.
        "the links counted by kind" => plan(vec![step(Operation::GroupCount {
            from: Input::Relation(Relation::TokenLink),
            keys: vec![at(2), at(3)],
            distinct: Some(at(0)),
        })]),
        // Each element beside the element `(` that ends where it starts: a join keyed, on both sides, by a cell the
        // span's bounds give.
        "the elements that start where a bracket ends" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            step(Operation::SpanBounds {
                from: Input::Step(StepId(0)),
                span: at(4),
            }),
            step(Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(at(2)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("(".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
            step(Operation::SpanBounds {
                from: Input::Step(StepId(2)),
                span: at(4),
            }),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(3)),
                (&[5, 3], &[6, 3]),
                vec![of(Side::Left, 0), of(Side::Right, 0), of(Side::Left, 3)],
            ),
        ]),
        _ => return None,
    })
}

/// An axis from the second column of step 0 over `edges` (parent, then child), its domain's entity the second column too.
fn axis_of(
    direction: Direction,
    edges: Relation,
    domain: Input,
) -> ryo_values::domain::plans::types::PlanStep {
    step(Operation::Axis {
        from: Input::Step(StepId(0)),
        edges: Input::Relation(edges),
        domain,
        direction,
        id: at(1),
        parent: at(0),
        child: at(1),
        domain_id: at(1),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    })
}
