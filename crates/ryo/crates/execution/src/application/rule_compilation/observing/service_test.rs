//! Observer normalization preserves column mappings, bag semantics and diagnostic ownership.
use super::service::prefix;
use crate::application::rule_compilation::constants::{
    BAG_PRESERVING_PROJECTION, CALL_TEXT, COMPOSED_PREFIX, DISTINCT_IS_BARRIER, IDENTITY_SELECT,
    RELATION_ROOTED_FILTER, ZERO_WIDTH_PROJECTION,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, ComparisonOrigin, ComparisonOriginId, Function, Input as PlanInput,
    LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// A typed symbol predicate with independent source-origin ownership.
fn comparison(column: ColumnIndex, origin: ComparisonOriginId) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(CALL_TEXT.into()))),
        origin: ComparisonOrigin::Recorded(origin),
    }
}

/// One side of an observed comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Side {
    Column(u32),
    Parameter(ParameterId),
    Text(Cow<'static, str>),
    /// A function applied to columns.
    Applied(Function, Cow<'static, [u32]>),
    /// Any other operand, as it prints.
    Other(String),
}

/// One comparison of an observed prefix: its origin and its two sides.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Compared {
    origin: ComparisonOrigin,
    left: Side,
    right: Side,
}

/// The prefix the observer answers for a step: the relation it is rooted at, its comparisons and
/// the columns it keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Observed {
    relation: Relation,
    comparisons: Cow<'static, [Compared]>,
    kept: Cow<'static, [u32]>,
}

/// A chain of steps, each named by why it has (or has no) prefix; step `n` reads the prefixes of
/// the steps before it.
type Input = Vec<(&'static str, Operation)>;
/// Each step's name and the prefix observed for it.
type Value = Cow<'static, [(&'static str, Option<Observed>)]>;
type Failure = String;

/// A comparison by `Test::Equal` of a column against the call text, with recorded origin `origin`.
const fn call_text(origin: u32, column: u32) -> Compared {
    Compared {
        origin: ComparisonOrigin::Recorded(ComparisonOriginId(origin)),
        left: Side::Column(column),
        right: Side::Text(Cow::Borrowed(CALL_TEXT)),
    }
}

/// The call text compared with column 1, recorded under origin 7.
const ORIGIN_7: &[Compared] = &[call_text(7, 1)];

/// The composed comparisons: origin 7's, and origin 99's concatenation of column 1 with itself,
/// as the projection maps column 3 to 1, against the parameter.
const COMPOSED: &[Compared] = &[
    call_text(7, 1),
    Compared {
        origin: ComparisonOrigin::Recorded(ComparisonOriginId(99)),
        left: Side::Applied(Function::Concat, Cow::Borrowed(&[1, 1])),
        right: Side::Parameter(ParameterId(0)),
    },
];

/// The prefix rooted at semantic syntax with `comparisons`, keeping `kept`.
const fn rooted(comparisons: &'static [Compared], kept: &'static [u32]) -> Option<Observed> {
    Some(Observed {
        relation: Relation::SemanticSyntax,
        comparisons: Cow::Borrowed(comparisons),
        kept: Cow::Borrowed(kept),
    })
}

/// The identity select of semantic syntax.
fn identity_select() -> Operation {
    Operation::Select {
        from: PlanInput::Relation(Relation::SemanticSyntax),
        comparisons: vec![],
        kept: vec![],
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "observer_composes_reordered_and_repeated_projections_without_erasing_origins",
        ),
        word: Word::Succeeds,
        input: || {
            let mut tested = comparison(ColumnIndex(1), ComparisonOriginId(99));
            tested.left = Operand::Function(Applied {
                function: Function::Concat,
                arguments: vec![
                    Operand::Column(ColumnIndex(3)),
                    Operand::Column(ColumnIndex(1)),
                ],
            });
            tested.right = Operand::Parameter(ParameterId(0));
            vec![
                (
                    RELATION_ROOTED_FILTER,
                    Operation::Select {
                        from: PlanInput::Relation(Relation::SemanticSyntax),
                        comparisons: vec![comparison(ColumnIndex(1), ComparisonOriginId(7))],
                        kept: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
                    },
                ),
                (
                    BAG_PRESERVING_PROJECTION,
                    Operation::Project {
                        from: PlanInput::Step(StepId(0)),
                        columns: vec![
                            ColumnIndex(1),
                            ColumnIndex(2),
                            ColumnIndex(0),
                            ColumnIndex(2),
                        ],
                        distinct: RepeatedRows::Kept,
                    },
                ),
                (
                    COMPOSED_PREFIX,
                    Operation::Select {
                        from: PlanInput::Step(StepId(1)),
                        comparisons: vec![tested],
                        kept: vec![ColumnIndex(2), ColumnIndex(0)],
                    },
                ),
            ]
        },
        expect: Expect::Value(Cow::Borrowed(&[
            (RELATION_ROOTED_FILTER, rooted(ORIGIN_7, &[2, 0, 1])),
            (BAG_PRESERVING_PROJECTION, rooted(ORIGIN_7, &[0, 1, 2, 1])),
            (COMPOSED_PREFIX, rooted(COMPOSED, &[2, 0])),
        ])),
    },
    Case {
        name: CaseName(
            "observer_stops_at_distinct_and_invalid_inputs_and_distinguishes_empty_projection",
        ),
        word: Word::Succeeds,
        input: || {
            vec![
                (IDENTITY_SELECT, identity_select()),
                (
                    ZERO_WIDTH_PROJECTION,
                    Operation::Project {
                        from: PlanInput::Step(StepId(0)),
                        columns: vec![],
                        distinct: RepeatedRows::Kept,
                    },
                ),
                (
                    DISTINCT_IS_BARRIER,
                    Operation::Project {
                        from: PlanInput::Step(StepId(0)),
                        columns: vec![ColumnIndex(0)],
                        distinct: RepeatedRows::Dropped,
                    },
                ),
            ]
        },
        expect: Expect::Value(Cow::Borrowed(&[
            (IDENTITY_SELECT, rooted(&[], &[0, 1, 2, 3, 4])),
            (ZERO_WIDTH_PROJECTION, rooted(&[], &[])),
            (DISTINCT_IS_BARRIER, None),
        ])),
    },
    Case {
        name: CaseName("a_select_of_a_missing_step_has_no_prefix"),
        word: Word::Succeeds,
        input: || {
            vec![(
                MISSING_STEP,
                Operation::Select {
                    from: PlanInput::Step(StepId(99)),
                    comparisons: vec![],
                    kept: vec![],
                },
            )]
        },
        expect: Expect::Value(Cow::Borrowed(&[(MISSING_STEP, None)])),
    },
    Case {
        name: CaseName("a_filter_on_a_missing_column_has_no_prefix"),
        word: Word::Succeeds,
        input: || {
            vec![(
                MISSING_COLUMN,
                Operation::Select {
                    from: PlanInput::Relation(Relation::SemanticSyntax),
                    comparisons: vec![comparison(ColumnIndex(99), ComparisonOriginId(7))],
                    kept: vec![],
                },
            )]
        },
        expect: Expect::Value(Cow::Borrowed(&[(MISSING_COLUMN, None)])),
    },
    Case {
        name: CaseName("a_projection_of_a_missing_column_has_no_prefix"),
        word: Word::Succeeds,
        input: || {
            vec![(
                MISSING_COLUMN,
                Operation::Project {
                    from: PlanInput::Relation(Relation::SemanticSyntax),
                    columns: vec![ColumnIndex(99)],
                    distinct: RepeatedRows::Kept,
                },
            )]
        },
        expect: Expect::Value(Cow::Borrowed(&[(MISSING_COLUMN, None)])),
    },
];

/// Why a select of a step that does not exist has no prefix.
const MISSING_STEP: &str = "step 99 does not exist";
/// Why a step naming a column its input lacks has no prefix.
const MISSING_COLUMN: &str = "column 99 does not exist";

/// A side as a row states it.
fn side(operand: &Operand) -> Side {
    match operand {
        Operand::Column(column) => Side::Column(column.0),
        Operand::Parameter(parameter) => Side::Parameter(*parameter),
        Operand::Literal(QueryLiteral::Text(text)) => Side::Text(Cow::Owned(text.0.to_string())),
        Operand::Function(applied) => {
            let columns = applied
                .arguments
                .iter()
                .map(|argument| match argument {
                    Operand::Column(column) => Some(column.0),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>();
            columns.map_or_else(
                || Side::Other(format!("{operand:?}")),
                |columns| Side::Applied(applied.function, Cow::Owned(columns)),
            )
        }
        Operand::Literal(_) => Side::Other(format!("{operand:?}")),
    }
}

/// Observes each step's prefix over the prefixes of the steps before it.
fn subject(steps: Input) -> Result<Value, Failure> {
    let mut below = Vec::new();
    let mut observed = Vec::new();
    for (why, operation) in steps {
        let found = prefix(&below, &operation);
        observed.push((
            why,
            found
                .as_ref()
                .map(|(relation, comparisons, kept)| Observed {
                    relation: *relation,
                    comparisons: Cow::Owned(
                        comparisons
                            .iter()
                            .map(|compared| Compared {
                                origin: compared.origin,
                                left: side(&compared.left),
                                right: side(&compared.right),
                            })
                            .collect(),
                    ),
                    kept: Cow::Owned(kept.iter().map(|column| column.0).collect()),
                }),
        ));
        below.push(found);
    }
    Ok(Cow::Owned(observed))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
