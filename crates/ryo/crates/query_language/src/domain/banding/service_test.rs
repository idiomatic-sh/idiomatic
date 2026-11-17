//! A join's `where` splits into its band and the rest: a comparison in order between a column of each side is an edge, at most
//! two of them the band; any other condition stays the filter after the join; a `nearest join` needs exactly one.

use super::service::band_split;
use super::types::BandSplit;
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use crate::selects;
use ryo_values::domain::band_joins::types::BandTest;
use ryo_values::domain::query_syntax::types as ast;
use ryo_values::domain::query_tokens::types as tokens;

/// The tests a row writes in its `where`, each `left test right` over `side.column`, and the kind of its join.
#[derive(Clone, Copy, Debug)]
struct Given {
    /// The join's kind.
    kind: ast::JoinKind,
    /// Its conditions, each over the column `o` of a side.
    tests: &'static [Written],
}

/// One `side.o test side.o`.
#[derive(Clone, Copy, Debug)]
struct Written {
    /// The alias of its left operand.
    left: &'static str,
    /// The test.
    test: ast::WrittenTest,
    /// The alias of its right operand.
    right: &'static str,
}

/// What the split answered: each band edge's test, and how many conditions stayed.
#[derive(Debug, PartialEq, Eq)]
struct Split {
    /// The test of each edge, as the right column compares with the left one.
    band: std::borrow::Cow<'static, [BandTest]>,
    /// How many conditions stayed the filter.
    rest: usize,
}

type Input = Given;
type Value = Split;
type Failure = LoweringError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_right_column_below_a_left_one_is_a_below_edge"),
        word: Word::Succeeds,
        input: || Given {
            kind: ast::JoinKind::Equal,
            tests: &[Written {
                left: "r",
                test: ast::WrittenTest::LessThan,
                right: "l",
            }],
        },
        expect: Expect::Value(Split {
            band: std::borrow::Cow::Borrowed(&[BandTest::Below]),
            rest: 0,
        }),
    },
    Case {
        name: CaseName("a_left_column_written_first_is_turned_around"),
        word: Word::Succeeds,
        input: || Given {
            kind: ast::JoinKind::Equal,
            tests: &[Written {
                left: "l",
                test: ast::WrittenTest::LessThanOrEqual,
                right: "r",
            }],
        },
        expect: Expect::Value(Split {
            band: std::borrow::Cow::Borrowed(&[BandTest::AtOrAbove]),
            rest: 0,
        }),
    },
    Case {
        name: CaseName("a_not_equal_and_a_one_sided_test_stay_the_filter"),
        word: Word::Succeeds,
        input: || Given {
            kind: ast::JoinKind::Equal,
            tests: &[
                Written {
                    left: "r",
                    test: ast::WrittenTest::NotEqual,
                    right: "l",
                },
                Written {
                    left: "l",
                    test: ast::WrittenTest::LessThan,
                    right: "l",
                },
            ],
        },
        expect: Expect::Value(Split {
            band: std::borrow::Cow::Borrowed(&[]),
            rest: 2,
        }),
    },
    Case {
        name: CaseName("a_third_order_comparison_stays_the_filter"),
        word: Word::Succeeds,
        input: || Given {
            kind: ast::JoinKind::Equal,
            tests: &[
                Written {
                    left: "r",
                    test: ast::WrittenTest::GreaterThanOrEqual,
                    right: "l",
                },
                Written {
                    left: "r",
                    test: ast::WrittenTest::LessThanOrEqual,
                    right: "l",
                },
                Written {
                    left: "r",
                    test: ast::WrittenTest::GreaterThan,
                    right: "l",
                },
            ],
        },
        expect: Expect::Value(Split {
            band: std::borrow::Cow::Borrowed(&[BandTest::AtOrAbove, BandTest::AtOrBelow]),
            rest: 1,
        }),
    },
    Case {
        name: CaseName("a_nearest_join_with_two_order_comparisons_is_refused"),
        word: Word::Refuses,
        input: || Given {
            kind: ast::JoinKind::Nearest,
            tests: &[
                Written {
                    left: "r",
                    test: ast::WrittenTest::LessThan,
                    right: "l",
                },
                Written {
                    left: "r",
                    test: ast::WrittenTest::GreaterThan,
                    right: "l",
                },
            ],
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::NearestNeedsOneOrder { count: 2, .. }
            ))
        }),
    },
];

/// The column `o` of the side `alias`.
fn column(alias: &str) -> ast::ColumnReference {
    ast::ColumnReference {
        alias: ast::Alias::Written(tokens::AliasName(alias.into())),
        column: ryo_values::domain::descriptions::types::ColumnName("o".into()),
    }
}

/// The source `function` as `alias`.
fn source(alias: &str) -> ast::QuerySource {
    ast::QuerySource {
        reference: ast::SourceReference::Relation(tokens::WrittenRelation("function".into())),
        alias: ast::Alias::Written(tokens::AliasName(alias.into())),
        span: lowering_builder().span,
    }
}

fn subject(given: Input) -> Result<Value, Failure> {
    let builder = lowering_builder();
    let join = ast::Join {
        kind: given.kind,
        source: source("r"),
        on: Vec::new(),
    };
    let select = ast::Select {
        columns: ast::Columns::Every,
        from: source("l"),
        join: ast::Joining::Joined(join.clone()),
        filters: given
            .tests
            .iter()
            .map(|each| {
                ast::Condition::Test(ast::Filter {
                    left: ast::WrittenOperand::Column(column(each.left)),
                    test: each.test,
                    right: ast::WrittenOperand::Column(column(each.right)),
                    span: builder.span,
                })
            })
            .collect(),
    };
    let BandSplit { band, rest } = band_split(&builder, &select, &join)?;
    Ok(Split {
        band: band.iter().map(|edge| edge.test).collect(),
        rest: rest.len(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
