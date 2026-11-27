//! The `trust` service's cases, on values alone.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::LiteralText;

use super::service::matters;
use super::types::relation_read::value::RelationRead;
use super::types::Bearing;
use crate::domain::coverage::types::{Lacking, LackingKind, Partial};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::stored_facts::types::PartialCause;

/// What a row asks of trust.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// Whether a read of `defines` keeping the kinds that compare so with this text (every kind when none) bears on `defines`
    /// read in part, lacking rows of this kind (any row when none).
    Matters(Option<(Test, &'static str)>, Option<&'static str>),
}

/// What trust answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Bearing(Bearing),
}

type Input = Ask;
type Value = Answer;
type Failure = std::convert::Infallible;

fn partial(kind: Option<&str>) -> Partial {
    Partial {
        relation: Relation::Defines,
        cause: PartialCause("a cause".into()),
        lacking: kind.map_or(Lacking::Any, |kind| Lacking::Kind(LackingKind(kind.into()))),
        at: crate::domain::coverage::types::PartialPlace::Repository,
    }
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Matters(kinds, missing) => {
            let read = RelationRead {
                relation: Relation::Defines,
                kinds: kinds
                    .map(|(test, literal)| PlanComparison {
                        test,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    })
                    .into_iter()
                    .collect(),
            };
            Answer::Bearing(matters(&read, &partial(missing)))
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_of_a_kind_the_read_does_not_let_through_cannot_matter"),
        word: Word::Succeeds,
        input: || Ask::Matters(Some((Test::Equal, "enum")), Some("variable")),
        expect: Expect::Value(Answer::Bearing(Bearing::DoesNotMatter)),
    },
    Case {
        name: CaseName("rows_of_a_kind_a_glob_of_kinds_lets_through_matter"),
        word: Word::Succeeds,
        input: || Ask::Matters(Some((Test::GlobAny, "{struct,variable}")), Some("variable")),
        expect: Expect::Value(Answer::Bearing(Bearing::Matters)),
    },
    Case {
        name: CaseName("a_glob_of_kinds_that_leaves_the_kind_out_cannot_matter"),
        word: Word::Succeeds,
        input: || Ask::Matters(Some((Test::GlobAny, "{struct,enum}")), Some("variable")),
        expect: Expect::Value(Answer::Bearing(Bearing::DoesNotMatter)),
    },
    Case {
        name: CaseName("a_read_that_keeps_every_kind_matters_when_any_row_could_be_missing"),
        word: Word::Succeeds,
        input: || Ask::Matters(None, None),
        expect: Expect::Value(Answer::Bearing(Bearing::Matters)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
