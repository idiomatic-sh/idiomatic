//! The `walking` service's cases, on plans written by hand.

use std::borrow::Cow;

use ryo_relations::domain::relations::types::Relation;

use super::service::{fuse_descents, fuse_neighbours};
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
    /// A lookup of the parent through the edges, keeping the edge's columns `kept`.
    AParentLookup(Kept),
    /// A lookup of the children through the edges, keeping the edge's columns `kept`.
    AChildLookup(Kept),
}

/// Which of the edge's columns, besides the element it reaches, a lookup keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kept {
    /// Its field, the role of the child under the parent.
    Field,
    /// Its ordinal, the place of the child under the parent.
    Ordinal,
    /// Its file.
    File,
    /// Its parent and its child.
    Ends,
}

/// What the service made of the last step: still a join, or one axis.
#[derive(Debug, PartialEq, Eq)]
enum Fused {
    /// The step is left a join.
    Join,
    /// The step is one axis in this direction, from this step, keeping the join's filter, giving these columns.
    Axis {
        direction: Direction,
        from: Reads,
        output: Cow<'static, [JoinColumn]>,
    },
    /// The step is something else, or an axis that lost the join's filter.
    Other,
}

/// What the service made of the step the row looks at.
type Value = Fused;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

const fn left(at: u32) -> JoinColumn {
    JoinColumn {
        side: Side::Left,
        column: ColumnIndex(at),
    }
}

const fn right(at: u32) -> JoinColumn {
    JoinColumn {
        side: Side::Right,
        column: ColumnIndex(at),
    }
}

fn per_file(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// The elements that are tokens: they have text.
fn tokens() -> PlanComparison {
    PlanComparison {
        test: Test::NotEqual,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(Box::from("")))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

/// What the service made of `step`, whose join filtered with `filter`.
fn fused(step: &PlanStep, filter: &PlanComparison) -> Fused {
    match &step.operation {
        Operation::Join { .. } => Fused::Join,
        Operation::Axis {
            direction,
            from,
            domain_filter,
            output,
            ..
        } if domain_filter.as_slice() == std::slice::from_ref(filter) => Fused::Axis {
            direction: *direction,
            from: *from,
            output: Cow::Owned(output.clone()),
        },
        _ => Fused::Other,
    }
}

fn a_descent_and_the_joins_after_it_are_one_walk(scenario: Scenario) -> Fused {
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
    let tokens = tokens();
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
    let fused_steps = fuse_descents(steps);
    fused(&fused_steps[3], &tokens)
}

/// The two joins RyoQL writes for a lookup through the edges: the elements joined with the edges that name them by `by`, keeping
/// the element's id and file, the edge's other end and the edge's columns `kept`; then that joined with the elements the other end
/// is, keeping every column of the first join and the kind of the element reached.
fn a_lookup_through_the_edges(by: u32, other_end: u32, kept: Kept) -> Fused {
    let kept_edge: &[u32] = match kept {
        // The edge's field, as `syntax-child` declares its columns: parent, child, ordinal, field, file.
        Kept::Field => &[3],
        Kept::Ordinal => &[2],
        Kept::File => &[4],
        Kept::Ends => &[0, 1],
    };
    let mut edges_output = vec![left(0), left(3), right(other_end)];
    edges_output.extend(kept_edge.iter().map(|at| right(*at)));
    let mut output: Vec<JoinColumn> = (0..u32::try_from(edges_output.len()).unwrap_or(u32::MAX))
        .map(left)
        .collect();
    output.push(right(1));
    let tokens = tokens();
    let steps = vec![
        per_file(Operation::Join {
            left: Reads::Relation(Relation::SyntaxTree),
            right: Reads::Relation(Relation::SyntaxChild),
            left_on: vec![column(0)],
            right_on: vec![column(by)],
            output: edges_output,
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
        per_file(Operation::Join {
            left: Reads::Step(StepId(0)),
            right: Reads::Relation(Relation::SyntaxTree),
            left_on: vec![column(2)],
            right_on: vec![column(0)],
            output,
            filter: vec![tokens.clone()],
            view: Vec::new(),

            reach: None,
        }),
    ];
    let fused_steps = fuse_neighbours(steps);
    fused(&fused_steps[1], &tokens)
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::ADescentAndTheJoinsAfterItAreOneWalk | Scenario::ADescentAmongSomeNodesIsLeft => {
            a_descent_and_the_joins_after_it_are_one_walk(scenario)
        }
        // The element found by the edge's child is its parent.
        Scenario::AParentLookup(kept) => a_lookup_through_the_edges(1, 0, kept),
        // The elements found by the edge's parent are its children.
        Scenario::AChildLookup(kept) => a_lookup_through_the_edges(0, 1, kept),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_descent_and_the_joins_after_it_are_one_walk_below_the_children"),
        word: Word::Succeeds,
        input: || Scenario::ADescentAndTheJoinsAfterItAreOneWalk,
        expect: Expect::Value(Fused::Axis {
            direction: Direction::Deep,
            from: Reads::Step(StepId(0)),
            output: Cow::Borrowed(&[left(0), left(2), left(3), right(0)]),
        }),
    },
    Case {
        name: CaseName("a_descent_among_some_nodes_only_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::ADescentAmongSomeNodesIsLeft,
        expect: Expect::Value(Fused::Join),
    },
    Case {
        name: CaseName("a_parent_lookup_keeping_the_edge_field_is_left_two_joins"),
        word: Word::Succeeds,
        input: || Scenario::AParentLookup(Kept::Field),
        expect: Expect::Value(Fused::Join),
    },
    Case {
        name: CaseName("a_child_lookup_keeping_the_edge_field_is_left_two_joins"),
        word: Word::Succeeds,
        input: || Scenario::AChildLookup(Kept::Field),
        expect: Expect::Value(Fused::Join),
    },
    Case {
        name: CaseName("a_lookup_keeping_the_edge_ordinal_is_left_two_joins"),
        word: Word::Succeeds,
        input: || Scenario::AParentLookup(Kept::Ordinal),
        expect: Expect::Value(Fused::Join),
    },
    Case {
        name: CaseName("a_parent_lookup_keeping_the_edge_file_reads_the_element_file"),
        word: Word::Succeeds,
        input: || Scenario::AParentLookup(Kept::File),
        // The element's file, as `syntax-tree` declares its columns: id, kind, text, file, span.
        expect: Expect::Value(Fused::Axis {
            direction: Direction::Parent,
            from: Reads::Relation(Relation::SyntaxTree),
            output: Cow::Borrowed(&[left(0), left(3), right(0), right(3), right(1)]),
        }),
    },
    Case {
        name: CaseName("a_parent_lookup_is_one_axis"),
        word: Word::Succeeds,
        input: || Scenario::AParentLookup(Kept::Ends),
        expect: Expect::Value(Fused::Axis {
            direction: Direction::Parent,
            from: Reads::Relation(Relation::SyntaxTree),
            output: Cow::Borrowed(&[left(0), left(3), right(0), right(0), left(0), right(1)]),
        }),
    },
    Case {
        name: CaseName("a_child_lookup_is_one_axis"),
        word: Word::Succeeds,
        input: || Scenario::AChildLookup(Kept::Ends),
        expect: Expect::Value(Fused::Axis {
            direction: Direction::Child,
            from: Reads::Relation(Relation::SyntaxTree),
            output: Cow::Borrowed(&[left(0), left(3), right(0), left(0), right(0), right(1)]),
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
