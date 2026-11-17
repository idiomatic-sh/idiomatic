//! A filter of a join places each comparison of its infallible prefix that tests one input on that input: a left one is a filter
//! of the left, a right one rides the probe when the probe walks the tree by a node key and is a filter of the right otherwise.
//! A comparison of both inputs, and every one from the first that may fail on, stays after the join in its order; a reader of
//! the join that is no filter keeps reading the join as it was.

use super::super::service::push_selections;
use super::{matches, named, select, select_kept, step, tested};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, Operand, ParameterId, PlanComparison, Side, StepId,
    Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A filter of a join of two facts relations tests a column of each.
    RelationJoinSidesPlaced,
    /// A filter of a join of the edges with the elements on the element's id tests an edge column and an element column.
    NodeKeyedTreeRightRidesTheWalk,
    /// A filter of a join with the elements on their span tests an element column.
    SpanKeyedTreeRightIsASelect,
    /// A filter of a join with the elements on their id tests the kind, then a pattern of the text.
    FallibleTreeRightStaysAfterTheJoin,
    /// A filter tests the left, then a pattern a parameter gives, then the left again.
    FalliblePrefixStays,
    /// A filter tests the left, then a column of the left against one of the right.
    CrossSideStays,
    /// The two alternatives of an `or` each filter the join, and a union reads both.
    OrAlternativesPlaceEach,
    /// A join is read by a filter and by an antijoin.
    NonFilterReaderKeepsTheJoin,
    /// A filter of a join keeps two of its columns, and tests nothing it cannot place.
    KeptFoldsIntoOutput,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_of_a_join_of_two_relations_is_made_on_each_side_it_tests"),
        word: Word::Succeeds,
        input: || Scenario::RelationJoinSidesPlaced,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_right_filter_of_a_join_with_the_tree_on_a_node_rides_the_walk"),
        word: Word::Succeeds,
        input: || Scenario::NodeKeyedTreeRightRidesTheWalk,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_right_filter_of_a_join_with_the_tree_on_a_span_is_a_filter_of_the_tree"),
        word: Word::Succeeds,
        input: || Scenario::SpanKeyedTreeRightIsASelect,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pattern_of_the_tree_stays_after_the_join_and_the_kind_rides_the_walk"),
        word: Word::Succeeds,
        input: || Scenario::FallibleTreeRightStaysAfterTheJoin,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_comparison_that_may_fail_and_every_one_after_it_stay_in_their_order"),
        word: Word::Succeeds,
        input: || Scenario::FalliblePrefixStays,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_comparison_of_both_sides_stays_after_the_join"),
        word: Word::Succeeds,
        input: || Scenario::CrossSideStays,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("each_alternative_of_an_or_places_its_own_sides"),
        word: Word::Succeeds,
        input: || Scenario::OrAlternativesPlaceEach,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_reader_of_a_join_that_is_no_filter_keeps_the_join"),
        word: Word::Succeeds,
        input: || Scenario::NonFilterReaderKeepsTheJoin,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_columns_a_placed_filter_keeps_are_the_joins_answer"),
        word: Word::Succeeds,
        input: || Scenario::KeptFoldsIntoOutput,
        expect: Expect::Value(true),
    },
];

/// The join of `left` on `left_on` with `right` on `right_on`, answering `output`, its right rows kept where `filter` holds.
fn join(
    (left, left_on): (Reads, u32),
    (right, right_on): (Reads, u32),
    output: &[(Side, u32)],
    filter: Vec<PlanComparison>,
) -> PlanStep {
    step(Operation::Join {
        left,
        right,
        left_on: vec![ColumnIndex(left_on)],
        right_on: vec![ColumnIndex(right_on)],
        output: output
            .iter()
            .map(|(side, column)| JoinColumn {
                side: *side,
                column: ColumnIndex(*column),
            })
            .collect(),
        filter,
        view: Vec::new(),
        reach: None,
    })
}

/// Reads the relation `relation`.
fn relation(relation: Relation) -> Reads {
    Reads::Relation(relation)
}

/// Reads the step at `at`.
fn at(at: u32) -> Reads {
    Reads::Step(StepId(at))
}

/// Column `left` equals column `right`.
fn same(left: u32, right: u32) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(left)),
        right: Operand::Column(ColumnIndex(right)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

/// Column `column` matches the pattern the first parameter gives.
fn matches_parameter(column: u32) -> PlanComparison {
    PlanComparison {
        test: Test::Matches,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

/// The edges joined with the elements on the child of the edge, answering the parent, the field, then the element's `answered`.
fn edges_with_elements(answered: &[(Side, u32)]) -> PlanStep {
    let mut output = vec![(Side::Left, 0), (Side::Left, 3)];
    output.extend_from_slice(answered);
    join(
        (relation(Relation::SyntaxChild), 1),
        (relation(Relation::SyntaxTree), 0),
        &output,
        Vec::new(),
    )
}

/// The semantic edges joined with the semantic elements on the child of the edge, answering the parent, the field, the kind
/// and the text.
fn semantic_join() -> PlanStep {
    join(
        (relation(Relation::SemanticChild), 1),
        (relation(Relation::SemanticSyntax), 0),
        &[
            (Side::Left, 0),
            (Side::Left, 3),
            (Side::Right, 1),
            (Side::Right, 2),
        ],
        Vec::new(),
    )
}

/// The plan a row gives, the step it emits, the plan it expects back, and the step that emits there.
type Planned = (Vec<PlanStep>, StepId, Vec<PlanStep>, StepId);

fn placed(scenario: Scenario) -> Planned {
    match scenario {
        Scenario::RelationJoinSidesPlaced => (
            vec![
                semantic_join(),
                select(at(0), vec![named(1, "operator"), named(3, "||")]),
            ],
            StepId(1),
            vec![
                select(
                    relation(Relation::SemanticChild),
                    vec![named(3, "operator")],
                ),
                select(relation(Relation::SemanticSyntax), vec![named(2, "||")]),
                join(
                    (at(0), 1),
                    (at(1), 0),
                    &[
                        (Side::Left, 0),
                        (Side::Left, 3),
                        (Side::Right, 1),
                        (Side::Right, 2),
                    ],
                    Vec::new(),
                ),
            ],
            StepId(2),
        ),
        Scenario::NodeKeyedTreeRightRidesTheWalk => (
            vec![
                edges_with_elements(&[(Side::Right, 1)]),
                select(at(0), vec![named(1, "body"), named(2, "return-statement")]),
            ],
            StepId(1),
            vec![
                select(relation(Relation::SyntaxChild), vec![named(3, "body")]),
                join(
                    (at(0), 1),
                    (relation(Relation::SyntaxTree), 0),
                    &[(Side::Left, 0), (Side::Left, 3), (Side::Right, 1)],
                    vec![named(1, "return-statement")],
                ),
            ],
            StepId(1),
        ),
        Scenario::SpanKeyedTreeRightIsASelect => {
            let spanned = |right: Reads| {
                join(
                    (relation(Relation::Function), 3),
                    (right, 4),
                    &[(Side::Left, 0), (Side::Right, 1)],
                    Vec::new(),
                )
            };
            (
                vec![
                    spanned(relation(Relation::SyntaxTree)),
                    select(at(0), vec![named(1, "comment")]),
                ],
                StepId(1),
                vec![
                    select(relation(Relation::SyntaxTree), vec![named(1, "comment")]),
                    spanned(at(0)),
                ],
                StepId(1),
            )
        }
        Scenario::FallibleTreeRightStaysAfterTheJoin => (
            vec![
                edges_with_elements(&[(Side::Right, 1), (Side::Right, 2)]),
                select(
                    at(0),
                    vec![named(2, "ident"), matches(3, "^(?:r#)?feature$")],
                ),
            ],
            StepId(1),
            vec![
                join(
                    (relation(Relation::SyntaxChild), 1),
                    (relation(Relation::SyntaxTree), 0),
                    &[
                        (Side::Left, 0),
                        (Side::Left, 3),
                        (Side::Right, 1),
                        (Side::Right, 2),
                    ],
                    vec![named(1, "ident")],
                ),
                select(at(0), vec![matches(3, "^(?:r#)?feature$")]),
            ],
            StepId(1),
        ),
        Scenario::FalliblePrefixStays => (
            vec![
                semantic_join(),
                select(
                    at(0),
                    vec![named(1, "x"), matches_parameter(3), named(0, "y")],
                ),
            ],
            StepId(1),
            vec![
                select(relation(Relation::SemanticChild), vec![named(3, "x")]),
                join(
                    (at(0), 1),
                    (relation(Relation::SemanticSyntax), 0),
                    &[
                        (Side::Left, 0),
                        (Side::Left, 3),
                        (Side::Right, 1),
                        (Side::Right, 2),
                    ],
                    Vec::new(),
                ),
                select(at(1), vec![matches_parameter(3), named(0, "y")]),
            ],
            StepId(2),
        ),
        Scenario::CrossSideStays => (
            vec![
                semantic_join(),
                select(at(0), vec![named(1, "x"), same(1, 3)]),
            ],
            StepId(1),
            vec![
                select(relation(Relation::SemanticChild), vec![named(3, "x")]),
                join(
                    (at(0), 1),
                    (relation(Relation::SemanticSyntax), 0),
                    &[
                        (Side::Left, 0),
                        (Side::Left, 3),
                        (Side::Right, 1),
                        (Side::Right, 2),
                    ],
                    Vec::new(),
                ),
                select(at(1), vec![same(1, 3)]),
            ],
            StepId(2),
        ),
        Scenario::OrAlternativesPlaceEach => {
            let alternative =
                |kind: &str| select_kept(at(0), vec![named(1, "test"), named(2, kind)], &[0]);
            let placed = |edges: u32, elements: u32| {
                join(
                    (at(edges), 1),
                    (at(elements), 0),
                    &[(Side::Left, 0)],
                    Vec::new(),
                )
            };
            (
                vec![
                    semantic_join(),
                    alternative("if-statement"),
                    alternative("while-statement"),
                    step(Operation::Union {
                        left: at(1),
                        right: at(2),
                    }),
                ],
                StepId(3),
                vec![
                    select(relation(Relation::SemanticChild), vec![named(3, "test")]),
                    select(
                        relation(Relation::SemanticSyntax),
                        vec![named(1, "if-statement")],
                    ),
                    placed(0, 1),
                    select(relation(Relation::SemanticChild), vec![named(3, "test")]),
                    select(
                        relation(Relation::SemanticSyntax),
                        vec![named(1, "while-statement")],
                    ),
                    placed(3, 4),
                    step(Operation::Union {
                        left: at(2),
                        right: at(5),
                    }),
                ],
                StepId(6),
            )
        }
        Scenario::NonFilterReaderKeepsTheJoin => {
            let antijoin = |left: u32, right: u32| {
                step(Operation::Antijoin {
                    left: at(left),
                    right: at(right),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                })
            };
            (
                vec![
                    semantic_join(),
                    select(at(0), vec![named(1, "x")]),
                    antijoin(1, 0),
                ],
                StepId(2),
                vec![
                    semantic_join(),
                    select(relation(Relation::SemanticChild), vec![named(3, "x")]),
                    join(
                        (at(1), 1),
                        (relation(Relation::SemanticSyntax), 0),
                        &[
                            (Side::Left, 0),
                            (Side::Left, 3),
                            (Side::Right, 1),
                            (Side::Right, 2),
                        ],
                        Vec::new(),
                    ),
                    antijoin(2, 0),
                ],
                StepId(3),
            )
        }
        Scenario::KeptFoldsIntoOutput => (
            vec![
                semantic_join(),
                select_kept(at(0), vec![named(2, "call")], &[3, 0]),
            ],
            StepId(1),
            vec![
                select(relation(Relation::SemanticSyntax), vec![named(1, "call")]),
                join(
                    (relation(Relation::SemanticChild), 1),
                    (at(0), 0),
                    &[(Side::Right, 2), (Side::Left, 0)],
                    Vec::new(),
                ),
            ],
            StepId(1),
        ),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(tested(placed(scenario), push_selections))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
