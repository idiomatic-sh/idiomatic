//! The `self_join_recogniser` service's cases, on values alone.

use std::borrow::Cow;

use super::service::{compile, number_origins};
use super::types::{Decline, Extremum, Program, StoredNumberOrigin};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Source, JoinColumn, Operand, PlanComparison,
    RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// A change to the fixture plan: a selection, its self-join on the first column answering the number of each side, a strict
/// order of the two numbers, a projection of the first and an anti-join of the selection by it.
#[derive(Clone, Copy, Debug)]
enum Change {
    /// None.
    Unchanged,
    /// The join is also a frontier of the plan.
    ExtraConsumer,
    /// The join filters by the order comparison.
    FilteredJoin,
    /// The join keeps a view of its first column.
    ViewedJoin,
    /// The order is a natural one.
    NaturalOrder,
}

/// What a row asks of the recogniser.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The shape found in the fixture plan over this relation, so changed.
    Recognise(Relation, Change),
    /// The origins of the number the second step answers, that step being `through` over a selection of `source_location`.
    Trace(Through),
}

/// The one shape found: its source, replaced join and select, retained project and anti-join, its extremum, and how many
/// origins its number has.
#[derive(Debug, PartialEq)]
struct Found {
    shapes: usize,
    steps: [StepId; 5],
    extremum: Extremum,
    origins: usize,
}

/// What the recogniser answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Shape(Found),
    Origins(Cow<'static, [StoredNumberOrigin]>),
}

type Input = Ask;
type Value = Answer;
type Failure = Decline;

fn fixture(relation: Relation) -> (Plan, Vec<CellKind>) {
    let schema = kinds(relation).to_vec();
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .map(|at| u32::try_from(at).expect("a relation's column number fits in u32"))
            .expect("typed numeric source"),
    );
    let source = Operation::Select {
        from: Source::Relation(relation),
        comparisons: vec![],
        kept: vec![],
    };
    let join = Operation::Join {
        left: Source::Step(StepId(0)),
        right: Source::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: number,
            },
            JoinColumn {
                side: Side::Right,
                column: number,
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let select = Operation::Select {
        from: Source::Step(StepId(1)),
        comparisons: vec![PlanComparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: vec![],
    };
    let project = Operation::Project {
        from: Source::Step(StepId(2)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let anti = Operation::Antijoin {
        left: Source::Step(StepId(0)),
        right: Source::Step(StepId(3)),
        left_on: vec![number],
        right_on: vec![ColumnIndex(0)],
    };
    (
        Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: [source, join, select, project, anti]
                .into_iter()
                .map(|operation| PlanStep {
                    operation,
                    scope: StepScope::PerFile,
                })
                .collect(),
            comparison_origins: vec![],
            emit: Emit {
                step: StepId(4),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
        },
        schema,
    )
}
fn prepared(plan: &Plan, schema: &[CellKind]) -> Program {
    compile(plan, &|id| (id == StepId(0)).then_some(schema))
}

/// One operation over the selection, built from the number's column there, with the number's column in its answer.
type Through = fn(ColumnIndex) -> (Operation, ColumnIndex);
/// The origins of the number the second step answers, when that step is `through` over the first step's selection of a
/// stored relation.
fn traced(through: Through) -> Result<Vec<StoredNumberOrigin>, Failure> {
    let (mut plan, _) = fixture(Relation::SourceLocation);
    let Operation::Join { output, .. } = &plan.steps[1].operation else {
        panic!("the fixture's second step is the join");
    };
    let (operation, column) = through(output[0].column);
    plan.steps.truncate(1);
    plan.steps.push(PlanStep {
        operation,
        scope: StepScope::PerFile,
    });
    number_origins(
        &plan,
        Source::Step(StepId(1)),
        column,
        crate::domain::operator::types::StepPlace(2),
    )
    .ok_or(Decline::UnprovenNumberOrigin)
}
fn project(number: ColumnIndex) -> (Operation, ColumnIndex) {
    (
        Operation::Project {
            from: Source::Step(StepId(0)),
            columns: vec![number],
            distinct: RepeatedRows::Kept,
        },
        ColumnIndex(0),
    )
}
fn join(number: ColumnIndex) -> (Operation, ColumnIndex) {
    (
        Operation::Join {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(0)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: number,
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        ColumnIndex(0),
    )
}
fn antijoin(number: ColumnIndex) -> (Operation, ColumnIndex) {
    (
        Operation::Antijoin {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(0)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        number,
    )
}
fn difference(number: ColumnIndex) -> (Operation, ColumnIndex) {
    (
        Operation::Difference {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(0)),
        },
        number,
    )
}
fn intersection(number: ColumnIndex) -> (Operation, ColumnIndex) {
    (
        Operation::Intersection {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(0)),
        },
        number,
    )
}
fn union(number: ColumnIndex) -> (Operation, ColumnIndex) {
    (
        Operation::Union {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(0)),
        },
        number,
    )
}

fn recognised(relation: Relation, change: Change) -> Result<Value, Failure> {
    let (mut plan, schema) = fixture(relation);
    match change {
        Change::Unchanged => {}
        Change::ExtraConsumer => plan.frontier.push(Source::Step(StepId(1))),
        Change::FilteredJoin => {
            let Operation::Select { comparisons, .. } = &plan.steps[2].operation else {
                panic!("the fixture's third step is the order select");
            };
            let comparison = comparisons[0].clone();
            if let Operation::Join { filter, .. } = &mut plan.steps[1].operation {
                filter.push(comparison);
            }
        }
        Change::ViewedJoin => {
            if let Operation::Join { view, .. } = &mut plan.steps[1].operation {
                view.push(ColumnIndex(0));
            }
        }
        Change::NaturalOrder => {
            if let Operation::Select { comparisons, .. } = &mut plan.steps[2].operation {
                comparisons[0].test = Test::NaturalLessThan;
            }
        }
    }
    let found = prepared(&plan, &schema);
    if let Some(declined) = found.declines.first() {
        return Err(declined.reason);
    }
    let shape = &found.shapes[0];
    Ok(Answer::Shape(Found {
        shapes: found.shapes.len(),
        steps: [
            shape.source,
            shape.replaced_join,
            shape.replaced_select,
            shape.retained_project,
            shape.retained_antijoin,
        ],
        extremum: shape.extremum,
        origins: shape.number.origins.len(),
    }))
}

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Recognise(relation, change) => recognised(relation, change),
        Ask::Trace(through) => traced(through).map(|origins| Answer::Origins(origins.into())),
    }
}

/// The origin of the number of `source_location`.
const LOCATION_NUMBER: Answer = Answer::Origins(Cow::Borrowed(&[StoredNumberOrigin {
    relation: Relation::SourceLocation,
    column: ColumnIndex(3),
}]));

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "an_eligible_shape_names_the_original_source_the_replacements_and_the_retained_operators",
        ),
        word: Word::Succeeds,
        input: || Ask::Recognise(Relation::SourceLocation, Change::Unchanged),
        expect: Expect::Value(Answer::Shape(Found {
            shapes: 1,
            steps: [StepId(0), StepId(1), StepId(2), StepId(3), StepId(4)],
            extremum: Extremum::Maximum,
            origins: 1,
        })),
    },
    Case {
        name: CaseName("an_extra_consumer_of_the_join_is_declined"),
        word: Word::Refuses,
        input: || Ask::Recognise(Relation::SourceLocation, Change::ExtraConsumer),
        expect: Expect::Error(Decline::ExtraConsumer),
    },
    Case {
        name: CaseName("a_join_with_a_filter_is_not_a_plain_self_join"),
        word: Word::Refuses,
        input: || Ask::Recognise(Relation::SourceLocation, Change::FilteredJoin),
        expect: Expect::Error(Decline::NotPlainSelfJoin),
    },
    Case {
        name: CaseName("a_join_with_a_view_is_not_a_plain_self_join"),
        word: Word::Refuses,
        input: || Ask::Recognise(Relation::SourceLocation, Change::ViewedJoin),
        expect: Expect::Error(Decline::NotPlainSelfJoin),
    },
    Case {
        name: CaseName("a_non_strict_order_is_declined"),
        word: Word::Refuses,
        input: || Ask::Recognise(Relation::SourceLocation, Change::NaturalOrder),
        expect: Expect::Error(Decline::NonStrictOrder),
    },
    Case {
        name: CaseName("a_number_of_a_native_relation_is_not_a_stored_origin"),
        word: Word::Refuses,
        input: || Ask::Recognise(Relation::SyntaxChild, Change::Unchanged),
        expect: Expect::Error(Decline::UnprovenNumberOrigin),
    },
    Case {
        name: CaseName("a_number_traced_through_a_projection_keeps_its_relation_origin"),
        word: Word::Succeeds,
        input: || Ask::Trace(project),
        expect: Expect::Value(LOCATION_NUMBER),
    },
    Case {
        name: CaseName("a_number_traced_through_a_plain_join_keeps_its_relation_origin"),
        word: Word::Succeeds,
        input: || Ask::Trace(join),
        expect: Expect::Value(LOCATION_NUMBER),
    },
    Case {
        name: CaseName("a_number_traced_through_an_antijoin_keeps_its_relation_origin"),
        word: Word::Succeeds,
        input: || Ask::Trace(antijoin),
        expect: Expect::Value(LOCATION_NUMBER),
    },
    Case {
        name: CaseName("a_number_traced_through_a_difference_keeps_its_relation_origin"),
        word: Word::Succeeds,
        input: || Ask::Trace(difference),
        expect: Expect::Value(LOCATION_NUMBER),
    },
    Case {
        name: CaseName("a_number_traced_through_an_intersection_keeps_its_relation_origin"),
        word: Word::Succeeds,
        input: || Ask::Trace(intersection),
        expect: Expect::Value(LOCATION_NUMBER),
    },
    Case {
        name: CaseName("a_number_traced_through_a_union_keeps_its_relation_origin"),
        word: Word::Succeeds,
        input: || Ask::Trace(union),
        expect: Expect::Value(LOCATION_NUMBER),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
