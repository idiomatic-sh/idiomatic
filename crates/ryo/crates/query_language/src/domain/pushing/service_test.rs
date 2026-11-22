//! The `pushing` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::{index_among, push_lookups};
use super::types::ColumnPrefixLength;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::ComparisonOrigin;
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
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A lookup by the parent of the edges of a join of edges and elements is two joins, the edges first.
    ALookupByTheParentIsPushed,
    /// A lookup by another column of the join is left as it is.
    ALookupByAnotherColumnIsLeft,
    /// A column found among the kept ones is placed after the columns before them; one not kept is refused, never read as
    /// the first.
    AColumnNotKeptIsRefused,
    /// A lookup into a filter of the join, by a kind, is pushed as a lookup into the join that keeps that kind itself.
    ALookupOverAnUnfoldedRightFilterPushesAsOverTheFoldedOne,
    /// A lookup into a filter of the join, by a kind and a pattern of the text, keeps the pattern above the element join.
    ALookupKeepsAFallibleRightComparisonAboveTheElementJoin,
    /// The same lookup, whose answer does not hold the text the pattern reads, is left as it is.
    ALookupWhoseResidualReadsAnUnansweredColumnDeclines,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::CrossFile,
    }
}

/// The functions, the join of the edges of the tree with its elements, and a lookup of the first into the
/// second by the column `asked` of the join.
fn lookup(asked: u32) -> Vec<PlanStep> {
    vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Relation(Relation::SyntaxChild),
            right: Reads::Relation(Relation::SyntaxTree),
            left_on: vec![column(1)],
            right_on: vec![column(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: column(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: column(0),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(0)),
            right: Reads::Step(StepId(1)),
            left_on: vec![column(0)],
            right_on: vec![column(asked)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: column(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: column(1),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
    ]
}

/// Column `at` compared with the written text `text` by `test`.
fn written(test: Test, at: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(column(at)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The columns `columns` of the side `side`.
fn sided(side: Side, columns: &[u32]) -> Vec<JoinColumn> {
    columns
        .iter()
        .map(|at| JoinColumn {
            side,
            column: column(*at),
        })
        .collect()
}

/// The functions; the edges joined with the elements on the child, answering the parent, the element's id, kind and text, the
/// elements kept where `folded` holds; when `between` is some, a filter of that join by it; and a lookup of the functions into
/// the last of these by the parent, answering the function and the columns `answered` of the right.
fn filtered_lookup(
    folded: Vec<PlanComparison>,
    between: Option<Vec<PlanComparison>>,
    answered: &[u32],
) -> Vec<PlanStep> {
    let mut steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Relation(Relation::SyntaxChild),
            right: Reads::Relation(Relation::SyntaxTree),
            left_on: vec![column(1)],
            right_on: vec![column(0)],
            output: [sided(Side::Left, &[0]), sided(Side::Right, &[0, 1, 2])].concat(),
            filter: folded,
            view: Vec::new(),
            reach: None,
        }),
    ];
    if let Some(comparisons) = between {
        steps.push(step(Operation::Select {
            from: Reads::Step(StepId(1)),
            comparisons,
            kept: Vec::new(),
        }));
    }
    let read = StepId(u32::try_from(steps.len() - 1).unwrap_or(u32::MAX));
    steps.push(step(Operation::Join {
        left: Reads::Step(StepId(0)),
        right: Reads::Step(read),
        left_on: vec![column(0)],
        right_on: vec![column(0)],
        output: [sided(Side::Left, &[0]), sided(Side::Right, answered)].concat(),
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }));
    steps
}

/// Where the answer numbers the steps from the edge join on: the edge join is `FROM_THE_EDGE_JOIN`, the next one after it, and
/// so on; a step before the edge join keeps its own number, which is below it.
const FROM_THE_EDGE_JOIN: u32 = 1000;

/// The steps from the first of the two joins a lookup was pushed into: the join with the edges, the join with the elements,
/// and what follows, each step they read from the edge join on numbered from `FROM_THE_EDGE_JOIN`, so two plans that differ
/// only in how many steps come before the edge join compare equal, and a step that reads the wrong one of them does not.
fn from_the_edge_join(steps: &[PlanStep]) -> Vec<Operation> {
    let edge = steps
        .iter()
        .position(|step| {
            matches!(
                step.operation,
                Operation::Join {
                    right: Reads::Relation(Relation::SyntaxChild),
                    ..
                }
            )
        })
        .unwrap_or(steps.len());
    let edge = u32::try_from(edge).unwrap_or(u32::MAX);
    steps
        .iter()
        .skip(usize::try_from(edge).unwrap_or(usize::MAX))
        .map(|step| {
            let mut operation = step.operation.clone();
            each_input(&mut operation, &mut |input| {
                if let Reads::Step(id) = input
                    && id.0 >= edge
                {
                    *id = StepId(id.0 - edge + FROM_THE_EDGE_JOIN);
                }
            });
            operation
        })
        .collect()
}

/// Whether the lookup over a filter of the kind pushes the same two joins as the lookup over the join that keeps the kind itself,
/// and the element join keeps that kind as it walks.
fn unfolded_as_folded() -> usize {
    let kind = |at: u32| vec![written(Test::Equal, at, "return-statement")];
    let unfolded = push_lookups(filtered_lookup(Vec::new(), Some(kind(2)), &[1]), StepId(3));
    let folded = push_lookups(filtered_lookup(kind(1), None, &[1]), StepId(2));
    let pushed = from_the_edge_join(&unfolded.steps);
    usize::from(
        pushed == from_the_edge_join(&folded.steps)
            && matches!(pushed.as_slice(), [_, Operation::Join { filter, .. }] if *filter == kind(1)),
    )
}

/// The lookup over a filter of the kind and a pattern of the text, answering the columns `answered` of the right: the operations
/// from the edge join on, after the push.
fn fallible_lookup(answered: &[u32]) -> Vec<Operation> {
    let steps = filtered_lookup(
        Vec::new(),
        Some(vec![
            written(Test::Equal, 2, "ident"),
            written(Test::Matches, 3, "^(?:r#)?feature$"),
        ]),
        answered,
    );
    from_the_edge_join(&push_lookups(steps, StepId(3)).steps)
}

/// Whether the pattern stays a filter of the element join's answer, reading the text the answer holds, and the element join,
/// reading the edge join, keeps only the kind.
fn fallible_kept_above() -> usize {
    usize::from(matches!(
        fallible_lookup(&[3]).as_slice(),
        [
            _,
            Operation::Join { left: Reads::Step(edges), filter, output, .. },
            Operation::Select { from: Reads::Step(elements), comparisons, kept },
        ] if edges.0 == FROM_THE_EDGE_JOIN
            && elements.0 == FROM_THE_EDGE_JOIN + 1
            && *filter == [written(Test::Equal, 1, "ident")]
            && output.len() == 2
            && *comparisons == [written(Test::Matches, 1, "^(?:r#)?feature$")]
            && kept.is_empty()
    ))
}

/// Whether the lookup whose answer does not hold the text is left as it is.
fn unanswered_declines() -> usize {
    let steps = filtered_lookup(
        Vec::new(),
        Some(vec![
            written(Test::Equal, 2, "ident"),
            written(Test::Matches, 3, "^(?:r#)?feature$"),
        ]),
        &[1],
    );
    usize::from(push_lookups(steps.clone(), StepId(3)).steps == steps)
}

fn pushed_lookup() -> usize {
    let planned = push_lookups(lookup(0), StepId(2));
    usize::from(
        planned.steps.len() == 4
            && planned.emitted == StepId(3)
            && matches!(
                &planned.steps[2].operation,
                Operation::Join {
                    left: Reads::Step(StepId(0)),
                    right: Reads::Relation(Relation::SyntaxChild),
                    ..
                }
            )
            && matches!(
                &planned.steps[3].operation,
                Operation::Join {
                    left: Reads::Step(StepId(2)),
                    right: Reads::Relation(Relation::SyntaxTree),
                    ..
                }
            ),
    )
}

fn left_lookup() -> usize {
    let steps = lookup(1);
    let planned = push_lookups(steps.clone(), StepId(2));
    usize::from(planned.steps == steps && planned.emitted == StepId(2))
}

fn placed_or_refused() -> usize {
    let kept = [column(4), column(2)];
    usize::from(
        index_among(&kept, column(2), ColumnPrefixLength(3)) == Some(column(4))
            && index_among(&kept, column(4), ColumnPrefixLength(0)) == Some(column(0))
            && index_among(&kept, column(7), ColumnPrefixLength(0)).is_none()
            && index_among(&kept, column(2), ColumnPrefixLength(usize::MAX)).is_none(),
    )
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::ALookupByTheParentIsPushed => pushed_lookup(),
        Scenario::ALookupByAnotherColumnIsLeft => left_lookup(),
        Scenario::AColumnNotKeptIsRefused => placed_or_refused(),
        Scenario::ALookupOverAnUnfoldedRightFilterPushesAsOverTheFoldedOne => unfolded_as_folded(),
        Scenario::ALookupKeepsAFallibleRightComparisonAboveTheElementJoin => fallible_kept_above(),
        Scenario::ALookupWhoseResidualReadsAnUnansweredColumnDeclines => unanswered_declines(),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_lookup_by_the_parent_of_the_edges_is_two_joins_the_edges_first"),
        word: Word::Succeeds,
        input: || Scenario::ALookupByTheParentIsPushed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lookup_by_another_column_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::ALookupByAnotherColumnIsLeft,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_column_not_kept_is_refused_never_read_as_the_first"),
        word: Word::Succeeds,
        input: || Scenario::AColumnNotKeptIsRefused,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lookup_over_an_unfolded_right_filter_pushes_as_over_the_folded_one"),
        word: Word::Succeeds,
        input: || Scenario::ALookupOverAnUnfoldedRightFilterPushesAsOverTheFoldedOne,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lookup_keeps_a_fallible_right_comparison_above_the_element_join"),
        word: Word::Succeeds,
        input: || Scenario::ALookupKeepsAFallibleRightComparisonAboveTheElementJoin,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lookup_whose_residual_reads_an_unanswered_column_declines"),
        word: Word::Succeeds,
        input: || Scenario::ALookupWhoseResidualReadsAnUnansweredColumnDeclines,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
