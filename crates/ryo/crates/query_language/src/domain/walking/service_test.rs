//! The `walking` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::fuse_descents;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as Reads;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A descent to every node below, the join with the children of what it reached and the join with the elements they are,
    /// are one walk below the children.
    ADescentAndTheJoinsAfterItAreOneWalk,
    /// A descent that admits more than the nodes is not the walk.
    ADescentAmongSomeNodesIsLeft,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn a_descent_and_the_joins_after_it_are_one_walk(scenario: Scenario) -> usize {
    let nodes = if scenario == Scenario::ADescentAndTheJoinsAfterItAreOneWalk {
        PlanComparison {
            test: Test::Equal,
            left: Operand::Column(column(2)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(Box::from("")))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }
    } else {
        PlanComparison {
            test: Test::Equal,
            left: Operand::Column(column(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("fn".into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }
    };
    let tokens = PlanComparison {
        test: Test::NotEqual,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(Box::from("")))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let left = |at: u32| JoinColumn {
        side: Side::Left,
        column: column(at),
    };
    let right = |at: u32| JoinColumn {
        side: Side::Right,
        column: column(at),
    };
    let per_file = |operation| PlanStep {
        operation,
        scope: StepScope::PerFile,
    };
    let steps = vec![
        per_file(Operation::Select {
            from: Reads::Relation(Relation::SyntaxTree),
            comparisons: Vec::new(),
            kept: vec![column(0), column(1), column(3), column(4)],
        }),
        per_file(Operation::Axis {
            from: Reads::Step(StepId(0)),
            edges: Reads::Relation(Relation::SyntaxChild),
            domain: Reads::Relation(Relation::SyntaxTree),
            direction: Direction::Descendant,
            id: column(0),
            parent: column(0),
            child: column(1),
            domain_id: column(0),
            domain_filter: vec![nodes],
            domain_view: vec![column(0), column(1), column(3), column(4)],
            output: Vec::new(),
        }),
        per_file(Operation::Join {
            left: Reads::Step(StepId(1)),
            right: Reads::Relation(Relation::SyntaxChild),
            left_on: vec![column(4)],
            right_on: vec![column(0)],
            output: vec![right(1), left(0), left(2), left(3)],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
        per_file(Operation::Join {
            left: Reads::Step(StepId(2)),
            right: Reads::Relation(Relation::SyntaxTree),
            left_on: vec![column(0)],
            right_on: vec![column(0)],
            output: vec![left(1), left(2), left(3), right(0)],
            filter: vec![tokens.clone()],
            view: Vec::new(),

            reach: None,
        }),
    ];
    let fused = fuse_descents(steps);
    usize::from(
        matches!(&fused[3].operation, Operation::Axis { direction: Direction::Deep, domain_filter, output, from: Reads::Step(StepId(0)), .. }
            if domain_filter == &[tokens] && output == &[left(0), left(2), left(3), right(0)])
            == (scenario == Scenario::ADescentAndTheJoinsAfterItAreOneWalk),
    )
}

fn subject(scenario: Input) -> Value {
    a_descent_and_the_joins_after_it_are_one_walk(scenario)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_descent_and_the_joins_after_it_are_one_walk_below_the_children"),
        word: Word::Succeeds,
        input: || Scenario::ADescentAndTheJoinsAfterItAreOneWalk,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_descent_among_some_nodes_only_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::ADescentAmongSomeNodesIsLeft,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
