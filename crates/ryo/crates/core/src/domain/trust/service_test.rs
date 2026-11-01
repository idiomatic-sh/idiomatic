//! The `trust` service's cases, on values alone.

use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::LiteralText;

use super::service::{blocked, matters};
use super::types::{Bearing, Blocking, RelationRead};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::{Lacking, LackingKind, Partial};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison, Operand, QueryLiteral, Test};
use ryo_values::domain::stored_facts::types::PartialCause;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Asks whether a read of `defines` that keeps the kinds of the comparison matters when rows of this kind could be missing.
    Matters(Test, &'static str, &'static str),
    /// Asks whether a read that keeps every kind matters when any row could be missing.
    MattersWhenAny,
    /// Asks whether a read of one relation is blocked by a relation read in part that it does not read.
    Blocked,
}

/// What a row of the cases does.
type Input = Scenario;
/// A yes or no as a number.
type Value = usize;
type Failure = ();

fn partial(kind: Option<&str>) -> Partial {
    Partial {
        relation: Relation::Defines,
        cause: PartialCause("a cause".into()),
        lacking: kind.map_or(Lacking::Any, |kind| Lacking::Kind(LackingKind(kind.into()))),
        at: None,
    }
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::Matters(test, literal, missing) => {
            let read = RelationRead {
                relation: Relation::Defines,
                kinds: vec![Comparison {
                    test,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
                    origin: None,
                }],
            };
            usize::from(matters(&read, &partial(Some(missing))) == Bearing::Matters)
        }
        Scenario::MattersWhenAny => {
            let read = RelationRead {
                relation: Relation::Defines,
                kinds: Vec::new(),
            };
            usize::from(matters(&read, &partial(None)) == Bearing::Matters)
        }
        Scenario::Blocked => {
            let read = RelationRead {
                relation: Relation::Reference,
                kinds: Vec::new(),
            };
            usize::from(blocked(&[read], &[partial(None)]) == Blocking::Blocked)
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_of_a_kind_the_read_does_not_let_through_cannot_matter"),
        word: Word::Succeeds,
        input: || Scenario::Matters(Test::Equal, "enum", "variable"),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("rows_of_a_kind_a_glob_of_kinds_lets_through_matter"),
        word: Word::Succeeds,
        input: || Scenario::Matters(Test::GlobAny, "{struct,variable}", "variable"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_glob_of_kinds_that_leaves_the_kind_out_cannot_matter"),
        word: Word::Succeeds,
        input: || Scenario::Matters(Test::GlobAny, "{struct,enum}", "variable"),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_read_that_keeps_every_kind_matters_when_any_row_could_be_missing"),
        word: Word::Succeeds,
        input: || Scenario::MattersWhenAny,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_read_in_part_blocks_only_the_reads_of_that_relation"),
        word: Word::Succeeds,
        input: || Scenario::Blocked,
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
