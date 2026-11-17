//! An antijoin that removes every row a strictly later row of its file follows keeps one row per file: the last point of each
//! file, whether a strict-order self join or an `ordered-match` names the rows it removes, once the points are distinct; over
//! points that may repeat, or an order that is not strict, it is shared.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::distinctness::service::distinct_on;
use crate::domain::distinctness::types::DistinctMemo;
use crate::domain::lowering::types::Uniqueness;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge, BandJoin, BandTest};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, KeyFan, RepeatedRows, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// The points the extremum is taken of.
#[derive(Clone, Copy, Debug)]
enum Points {
    /// `select ordinal as point, file from "binding-event"`: a point may repeat.
    Selected,
    /// The same, projected with its repeats dropped: each point once.
    Projected,
}

/// What names the points a later point of their file follows.
#[derive(Clone, Copy, Debug)]
enum Followed {
    /// `select l.point, l.file from S as l join S as r on l.file = r.file where l.point < r.point`.
    StrictSelfJoin,
    /// The same with `l.point <= r.point`: every point follows itself.
    NonStrictSelfJoin,
    /// `ordered-match(left=S, right=S, left-on=[file], right-on=[file], left-order=point, right-order=point)`.
    OrderedMatch,
}

/// What a row asks of the service: the points and what names the followed ones.
#[derive(Clone, Copy, Debug)]
struct Given {
    /// The points.
    points: Points,
    /// What names the points a later one follows.
    by: Followed,
}
type Input = Given;
/// Whether the antijoin is distinct on the file.
type Value = Uniqueness;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_last_point_by_a_strict_self_join_of_distinct_points_is_one_per_file"),
        word: Word::Succeeds,
        input: || Given {
            points: Points::Projected,
            by: Followed::StrictSelfJoin,
        },
        expect: Expect::Value(Uniqueness::Unique),
    },
    Case {
        name: CaseName("the_last_point_by_an_ordered_match_of_distinct_points_is_one_per_file"),
        word: Word::Succeeds,
        input: || Given {
            points: Points::Projected,
            by: Followed::OrderedMatch,
        },
        expect: Expect::Value(Uniqueness::Unique),
    },
    Case {
        name: CaseName("the_last_point_by_a_strict_self_join_of_selected_events_is_shared"),
        word: Word::Succeeds,
        input: || Given {
            points: Points::Selected,
            by: Followed::StrictSelfJoin,
        },
        expect: Expect::Value(Uniqueness::Shared),
    },
    Case {
        name: CaseName("the_last_point_by_an_ordered_match_of_selected_events_is_shared"),
        word: Word::Succeeds,
        input: || Given {
            points: Points::Selected,
            by: Followed::OrderedMatch,
        },
        expect: Expect::Value(Uniqueness::Shared),
    },
    Case {
        name: CaseName("the_points_a_non_strict_self_join_names_leave_a_shared_answer"),
        word: Word::Succeeds,
        input: || Given {
            points: Points::Projected,
            by: Followed::NonStrictSelfJoin,
        },
        expect: Expect::Value(Uniqueness::Shared),
    },
];

/// The point (column 0) and the file (column 1) of the points.
const POINT: ColumnIndex = ColumnIndex(0);
const FILE: ColumnIndex = ColumnIndex(1);

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// The step that pairs each point of `points` with a later one of its file, answering the earlier one's point and file.
fn followed(points: Reads, by: Followed) -> Operation {
    let output = vec![
        JoinColumn {
            side: Side::Left,
            column: POINT,
        },
        JoinColumn {
            side: Side::Left,
            column: FILE,
        },
    ];
    let band = |test| {
        Operation::BandJoin(BandJoin {
            left: points,
            right: points,
            left_on: vec![FILE],
            right_on: vec![FILE],
            band: vec![BandEdge {
                left: POINT,
                test,
                right: POINT,
            }],
            answer: BandAnswer::Every,
            output: output.clone(),
            filter: Vec::new(),
            view: Vec::new(),
            fan: KeyFan::Shared,
        })
    };
    match by {
        Followed::StrictSelfJoin => band(BandTest::Above),
        Followed::NonStrictSelfJoin => band(BandTest::AtOrAbove),
        Followed::OrderedMatch => Operation::OrderedMatch {
            left: points,
            right: points,
            left_on: vec![FILE],
            right_on: vec![FILE],
            left_order: POINT,
            right_order: POINT,
            output: output.clone(),
        },
    }
}

fn subject(Given { points, by }: Input) -> Result<Value, Failure> {
    let source = Reads::Step(StepId(match points {
        Points::Selected => 0,
        Points::Projected => 1,
    }));
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::BindingEvent),
            comparisons: Vec::new(),
            // `ordinal` and `file` of `binding-event`.
            kept: vec![ColumnIndex(5), ColumnIndex(6)],
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(0)),
            columns: vec![POINT, FILE],
            distinct: RepeatedRows::Dropped,
        }),
        step(followed(source, by)),
        step(Operation::Antijoin {
            left: source,
            right: Reads::Step(StepId(2)),
            left_on: vec![POINT, FILE],
            right_on: vec![POINT, FILE],
        }),
    ];
    Ok(distinct_on(
        &steps,
        &mut DistinctMemo::default(),
        Reads::Step(StepId(3)),
        &[FILE],
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
