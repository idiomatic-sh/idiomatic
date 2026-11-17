//! A filter of a join is made on the input whose columns it tests, and the infallible prefix of a filter of a union on both its
//! inputs when every reader of the union is a filter whose prefix is not empty; a union another step reads whole stays as it is.

use super::service::push_selections;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::rewriting::types::Planned;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, HostedNumber, Input as Reads, JoinColumn, LiteralText, Operand,
    ParameterId, PlanComparison, QueryLiteral, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The only reader of a union of a join keyed on the file is a filter.
    FilteredUnion,
    /// The only reader of a join keyed on the file is a filter of a column of its right input.
    FilteredJoin,
    /// A union is read by a filter and by a join.
    UnionReadWhole,
    /// A union of two filters is read by two filters, each with an infallible prefix, and the first with a pattern after it.
    UnionOfFilterReadersPushesThePrefix,
    /// A union of a join keyed on the file is read by a filter that tests a pattern a parameter gives first.
    UnionFalliblePrefixStays,
    /// A union whose rows' width the plan does not say (its left is a hosted relation) is read by a filter of a written name.
    UnionOfUnknownWidthStays,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_filter_of_a_union_of_a_join_keyed_on_the_file_is_made_on_both_its_inputs_and_into_the_join",
        ),
        word: Word::Succeeds,
        input: || Scenario::FilteredUnion,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_filter_of_a_join_keyed_on_the_file_on_a_column_of_its_right_input_is_made_on_that_input",
        ),
        word: Word::Succeeds,
        input: || Scenario::FilteredJoin,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_union_another_step_reads_whole_stays_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::UnionReadWhole,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "the_infallible_prefix_of_each_filter_of_a_union_is_made_on_both_its_inputs",
        ),
        word: Word::Succeeds,
        input: || Scenario::UnionOfFilterReadersPushesThePrefix,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_union_whose_filter_may_fail_first_stays_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::UnionFalliblePrefixStays,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_union_whose_width_is_unknown_is_not_pushed"),
        word: Word::Succeeds,
        input: || Scenario::UnionOfUnknownWidthStays,
        expect: Expect::Value(true),
    },
];

/// A step of `operation`, made for each file.
pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// A filter of `from` by `comparisons`, keeping every column.
pub(super) fn select(from: Reads, comparisons: Vec<PlanComparison>) -> PlanStep {
    select_kept(from, comparisons, &[])
}

/// A filter of `from` by `comparisons`, keeping the columns `kept`.
pub(super) fn select_kept(from: Reads, comparisons: Vec<PlanComparison>, kept: &[u32]) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: kept.iter().copied().map(ColumnIndex).collect(),
    })
}

/// Column `column` compared with the written text `text` by `test`.
fn written(test: Test, column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// Column `column` equals the text `text`.
pub(super) fn named(column: u32, text: &str) -> PlanComparison {
    written(Test::Equal, column, text)
}

/// Column `column` matches the written pattern `pattern`.
pub(super) fn matches(column: u32, pattern: &str) -> PlanComparison {
    written(Test::Matches, column, pattern)
}

/// Column `column` equals the text "main".
fn named_main(column: u32) -> Vec<PlanComparison> {
    vec![named(column, "main")]
}

/// Whether `rewrite` gives back, for the plan and the step it emits, the plan and the step the row expects.
pub(super) fn tested(
    (before, emitted, after, emits): (Vec<PlanStep>, StepId, Vec<PlanStep>, StepId),
    rewrite: fn(Vec<PlanStep>, StepId) -> Planned,
) -> bool {
    let planned = rewrite(before, emitted);
    planned.steps == after && planned.emitted == emits
}

/// The union of the functions and the comments, read by a filter of the names `main` that then tests a pattern a parameter
/// gives, and by a filter of the names `run`; a union of the two filters is emitted.
fn union_of_filtered_readers() -> (Vec<PlanStep>, StepId, Vec<PlanStep>, StepId) {
    let pattern = PlanComparison {
        test: Test::Matches,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ComparisonOrigin::Unrecorded,
    };
    let union = |left: u32, right: u32| {
        step(Operation::Union {
            left: Reads::Step(StepId(left)),
            right: Reads::Step(StepId(right)),
        })
    };
    let main = named(1, "main");
    let run_name = named(1, "run");
    (
        vec![
            functions(),
            comments(),
            union(0, 1),
            select(Reads::Step(StepId(2)), vec![main.clone(), pattern.clone()]),
            select(Reads::Step(StepId(2)), vec![run_name.clone()]),
            union(3, 4),
        ],
        StepId(5),
        vec![
            functions(),
            comments(),
            select(Reads::Step(StepId(0)), vec![main.clone()]),
            select(Reads::Step(StepId(1)), vec![main]),
            union(2, 3),
            select(Reads::Step(StepId(4)), vec![pattern]),
            select(Reads::Step(StepId(0)), vec![run_name.clone()]),
            select(Reads::Step(StepId(1)), vec![run_name]),
            union(6, 7),
            union(5, 8),
        ],
        StepId(9),
    )
}

fn join(left: Reads, right: Reads) -> PlanStep {
    step(Operation::Join {
        left,
        right,
        left_on: vec![ColumnIndex(2)],
        right_on: vec![ColumnIndex(2)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
        ],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    })
}

fn functions() -> PlanStep {
    select(Reads::Relation(Relation::Function), Vec::new())
}

fn comments() -> PlanStep {
    select(Reads::Relation(Relation::Comment), Vec::new())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let union = step(Operation::Union {
        left: Reads::Step(StepId(2)),
        right: Reads::Step(StepId(1)),
    });
    let (before, emitted, after, emits) = match scenario {
        Scenario::FilteredUnion => (
            vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                step(Operation::Union {
                    left: Reads::Step(StepId(2)),
                    right: Reads::Step(StepId(1)),
                }),
                select(Reads::Step(StepId(3)), named_main(1)),
            ],
            StepId(4),
            vec![
                functions(),
                comments(),
                select(Reads::Step(StepId(1)), named_main(1)),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(2))),
                select(Reads::Step(StepId(1)), named_main(1)),
                step(Operation::Union {
                    left: Reads::Step(StepId(3)),
                    right: Reads::Step(StepId(4)),
                }),
            ],
            StepId(5),
        ),
        Scenario::FilteredJoin => (
            vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                select(Reads::Step(StepId(2)), named_main(1)),
            ],
            StepId(3),
            vec![
                functions(),
                comments(),
                select(Reads::Step(StepId(1)), named_main(1)),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(2))),
            ],
            StepId(3),
        ),
        Scenario::UnionReadWhole => {
            let steps = vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                union,
                select(Reads::Step(StepId(3)), named_main(1)),
                step(Operation::Join {
                    left: Reads::Step(StepId(4)),
                    right: Reads::Step(StepId(3)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: Vec::new(),
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                }),
            ];
            (steps.clone(), StepId(5), steps, StepId(5))
        }
        Scenario::UnionOfFilterReadersPushesThePrefix => union_of_filtered_readers(),
        Scenario::UnionOfUnknownWidthStays => {
            let steps = vec![
                step(Operation::Hosted {
                    relation: HostedNumber(0),
                }),
                functions(),
                step(Operation::Union {
                    left: Reads::Step(StepId(0)),
                    right: Reads::Step(StepId(1)),
                }),
                select(Reads::Step(StepId(2)), named_main(1)),
            ];
            (steps.clone(), StepId(3), steps, StepId(3))
        }
        Scenario::UnionFalliblePrefixStays => {
            let steps = vec![
                functions(),
                comments(),
                join(Reads::Step(StepId(0)), Reads::Step(StepId(1))),
                step(Operation::Union {
                    left: Reads::Step(StepId(2)),
                    right: Reads::Step(StepId(1)),
                }),
                select(
                    Reads::Step(StepId(3)),
                    vec![
                        PlanComparison {
                            test: Test::Matches,
                            left: Operand::Column(ColumnIndex(1)),
                            right: Operand::Parameter(ParameterId(0)),
                            origin: ComparisonOrigin::Unrecorded,
                        },
                        named(1, "main"),
                    ],
                ),
            ];
            (steps.clone(), StepId(4), steps, StepId(4))
        }
    };
    Ok(tested((before, emitted, after, emits), push_selections))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod placement_test;
