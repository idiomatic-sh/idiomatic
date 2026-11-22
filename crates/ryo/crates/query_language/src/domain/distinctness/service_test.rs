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
use std::borrow::Cow;

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

/// What a row asks: whether one scenario's step is distinct on its columns, or a ladder at a depth and at twice it.
#[derive(Clone, Copy)]
enum Ask {
    /// Whether the scenario's step is distinct on its columns.
    Distinct(Scenario),
    /// Whether the last step of a ladder of so many joins, each of the step before with itself on its key, is distinct on it.
    Ladder(usize),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// Whether the step is distinct on the columns.
    Distinct(Uniqueness),
    /// How many sets of columns of steps a ladder was followed for at a depth and at twice it, beside how many steps each has.
    Counted(Cow<'static, str>),
}

type Value = Answer;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Ask;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_keeping_a_key_column_of_a_relation_is_distinct_on_it"),
        word: Word::Succeeds,
        input: || Ask::Distinct(Scenario::TheChildOfTheEdges),
        expect: Expect::Value(Answer::Distinct(Uniqueness::Unique)),
    },
    Case {
        name: CaseName("a_column_many_rows_share_is_shared"),
        word: Word::Succeeds,
        input: || Ask::Distinct(Scenario::TheParentOfTheEdges),
        expect: Expect::Value(Answer::Distinct(Uniqueness::Shared)),
    },
    Case {
        name: CaseName("a_projection_is_distinct_on_every_column_it_keeps"),
        word: Word::Succeeds,
        input: || Ask::Distinct(Scenario::TheMarkedFiles),
        expect: Expect::Value(Answer::Distinct(Uniqueness::Unique)),
    },
    Case {
        name: CaseName("a_join_meeting_one_row_at_most_keeps_the_key_of_its_input"),
        word: Word::Succeeds,
        input: || Ask::Distinct(Scenario::TheEdgesJoinedToTheirParentEdge),
        expect: Expect::Value(Answer::Distinct(Uniqueness::Unique)),
    },
    Case {
        name: CaseName("a_join_meeting_many_rows_is_shared_on_the_key_of_its_input"),
        word: Word::Succeeds,
        input: || Ask::Distinct(Scenario::TheEdgesJoinedToTheirSiblings),
        expect: Expect::Value(Answer::Distinct(Uniqueness::Shared)),
    },
    Case {
        name: CaseName(
            "a_projection_that_keeps_repeated_rows_after_a_split_is_shared_on_the_key_it_kept",
        ),
        word: Word::Succeeds,
        input: || Ask::Distinct(Scenario::TheWordsOfOneFunctionKeptRepeated),
        expect: Expect::Value(Answer::Distinct(Uniqueness::Shared)),
    },
    Case {
        name: CaseName("a_ladder_of_joins_of_a_step_with_itself_follows_each_step_once"),
        word: Word::Succeeds,
        input: || Ask::Ladder(16),
        expect: Expect::Value(Answer::Counted(Cow::Borrowed(
            "16 levels, 17 steps: 17 sets followed, Unique; 32 levels, 33 steps: 33 sets followed, Unique",
        ))),
    },
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
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
        reach: None,
    }
}

/// The child of each syntax edge, then `levels` joins, each of the step before with itself on its one column, keeping it from both
/// sides; how many sets of columns of steps whether the last is distinct on its first column followed, and what it found.
fn ladder_followed(levels: usize) -> String {
    let mut steps = vec![step(Operation::Select {
        from: Reads::Relation(Relation::SyntaxChild),
        comparisons: Vec::new(),
        kept: vec![ColumnIndex(1)],
    })];
    for _ in 0..levels {
        let before = Reads::Step(StepId(
            u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
        ));
        steps.push(step(Operation::Join {
            left: before,
            right: before,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        }));
    }
    let top = Reads::Step(StepId(
        u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
    ));
    let mut memo = DistinctMemo::default();
    let found = distinct_on(&steps, &mut memo, top, &[ColumnIndex(0)]);
    format!(
        "{levels} levels, {} steps: {} sets followed, {found:?}",
        steps.len(),
        memo.followed.0
    )
}

fn subject(ask: Input) -> Result<Value, Failure> {
    let scenario = match ask {
        Ask::Distinct(scenario) => scenario,
        Ask::Ladder(levels) => {
            return Ok(Answer::Counted(Cow::Owned(format!(
                "{}; {}",
                ladder_followed(levels),
                ladder_followed(levels.saturating_mul(2))
            ))));
        }
    };
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
    Ok(Answer::Distinct(distinct_on(
        &steps,
        &mut DistinctMemo::default(),
        Reads::Step(StepId(at)),
        &keys,
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
