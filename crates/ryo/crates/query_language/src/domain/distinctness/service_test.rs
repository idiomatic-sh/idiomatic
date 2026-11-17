//! A step is distinct on a relation's key column it keeps, on every column of a projection, and through a join that meets one row at
//! most; a column many rows share, or a join that meets many rows, is shared.

use super::service::distinct_on;
use super::types::DistinctMemo;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::types::Uniqueness;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, RepeatedRows, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service: a step of the plan and the columns of it.
#[derive(Clone, Copy)]
enum Scenario {
    /// The child of the syntax edges, the key of the relation, kept by a filter.
    TheChildOfTheEdges,
    /// The parent of the syntax edges, which every child under it shares.
    TheParentOfTheEdges,
    /// The files with a marker, a projection that drops the files repeated.
    TheMarkedFiles,
    /// The edges joined on their child with the edge of their parent: one parent edge at most for each.
    TheEdgesJoinedToTheirParentEdge,
    /// The edges joined on their parent with the edges under it: every sibling of each.
    TheEdgesJoinedToTheirSiblings,
    /// The functions' ids and names after their names were split into words, a projection that keeps its repeated rows, asked of
    /// both columns.
    TheWordsOfOneFunctionKeptRepeated,
}

/// Whether the step is distinct on the columns.
type Value = Uniqueness;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_keeping_a_key_column_of_a_relation_is_distinct_on_it"),
        word: Word::Succeeds,
        input: || Scenario::TheChildOfTheEdges,
        expect: Expect::Value(Uniqueness::Unique),
    },
    Case {
        name: CaseName("a_column_many_rows_share_is_shared"),
        word: Word::Succeeds,
        input: || Scenario::TheParentOfTheEdges,
        expect: Expect::Value(Uniqueness::Shared),
    },
    Case {
        name: CaseName("a_projection_is_distinct_on_every_column_it_keeps"),
        word: Word::Succeeds,
        input: || Scenario::TheMarkedFiles,
        expect: Expect::Value(Uniqueness::Unique),
    },
    Case {
        name: CaseName("a_join_meeting_one_row_at_most_keeps_the_key_of_its_input"),
        word: Word::Succeeds,
        input: || Scenario::TheEdgesJoinedToTheirParentEdge,
        expect: Expect::Value(Uniqueness::Unique),
    },
    Case {
        name: CaseName("a_join_meeting_many_rows_is_shared_on_the_key_of_its_input"),
        word: Word::Succeeds,
        input: || Scenario::TheEdgesJoinedToTheirSiblings,
        expect: Expect::Value(Uniqueness::Shared),
    },
    Case {
        name: CaseName(
            "a_projection_that_keeps_repeated_rows_after_a_split_is_shared_on_the_key_it_kept",
        ),
        word: Word::Succeeds,
        input: || Scenario::TheWordsOfOneFunctionKeptRepeated,
        expect: Expect::Value(Uniqueness::Shared),
    },
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// The edges joined with the edges on column `left` of the first and `right` of the second, keeping the child of the first.
fn edges_joined(left: u32, right: u32) -> Operation {
    Operation::Join {
        left: Reads::Step(StepId(0)),
        right: Reads::Step(StepId(0)),
        left_on: vec![ColumnIndex(left)],
        right_on: vec![ColumnIndex(right)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(1),
        }],
        filter: Vec::new(),
        view: Vec::new(),
        fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
        reach: None,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxChild),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Project {
            from: Reads::Relation(Relation::Comment),
            columns: vec![ColumnIndex(2)],
            distinct: RepeatedRows::Dropped,
        }),
        // The parent of each edge is the child of one edge at most.
        step(edges_joined(0, 1)),
        // The parent of each edge is the parent of every sibling.
        step(edges_joined(0, 0)),
        // Each function once for each word of its name, its id and name kept with the repeats.
        step(Operation::Words {
            from: Reads::Relation(Relation::Function),
            text: ColumnIndex(1),
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(4)),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
            distinct: RepeatedRows::Kept,
        }),
    ];
    let (at, columns): (u32, &[u32]) = match scenario {
        Scenario::TheChildOfTheEdges => (0, &[1]),
        Scenario::TheParentOfTheEdges => (0, &[0]),
        Scenario::TheMarkedFiles => (1, &[0]),
        Scenario::TheEdgesJoinedToTheirParentEdge => (2, &[0]),
        Scenario::TheEdgesJoinedToTheirSiblings => (3, &[0]),
        // Asked of every column it keeps: a projection that drops repeats would be distinct on them.
        Scenario::TheWordsOfOneFunctionKeptRepeated => (5, &[0, 1]),
    };
    let keys: Vec<ColumnIndex> = columns.iter().copied().map(ColumnIndex).collect();
    Ok(distinct_on(
        &steps,
        &mut DistinctMemo::default(),
        Reads::Step(StepId(at)),
        &keys,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
