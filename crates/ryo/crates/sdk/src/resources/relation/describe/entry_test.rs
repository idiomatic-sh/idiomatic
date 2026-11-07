//! What `relation describe` promises its caller, one row of `CASES` for each case it has.

use super::entry::describe;
use super::errors::DescribeError;
use super::types::DescribeRequest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::language::types::Languages;
use crate::domain::relation::types::RelationName;

type Input = &'static str;
/// How many columns the relation has.
type Value = usize;
type Failure = DescribeError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("describes_a_relation"),
        word: Word::Succeeds,
        input: || "function",
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || "nothing",
        expect: Expect::Refusal(|error| selects!(error, DescribeError::UnknownRelation { .. })),
    },
];

/// The language that reads nothing, which says every relation of the catalog is not in it: what a relation is does not depend on a language.
fn nothing() -> crate::domain::language::types::Language {
    use std::sync::Arc;

    crate::resources::languages::link(
        crate::domain::language::types::LanguageName("nothing".into()),
        crate::domain::language::types::LanguageBuild("0".into()),
        Arc::new(ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl),
        Arc::new(ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl),
    )
}

fn subject(name: Input) -> Result<Value, Failure> {
    describe(
        Languages(vec![nothing()]),
        DescribeRequest {
            name: RelationName(name.into()),
        },
    )
    .map(|report| report.relation.columns.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
