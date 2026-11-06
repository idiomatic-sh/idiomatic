//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use std::sync::Arc;

use ryo_core::inbound::use_cases::describe_relation::errors::DescribeRelationError as CoreError;

use super::super::entry::describe;
use super::super::errors::DescribeError;
use super::super::types::DescribeRequest;
use super::DescribeRelationErrorSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::language::types::Languages;
use crate::domain::relation::types::RelationName;

/// What a row offers the schemas.
enum Offered {
    /// A relation asked for by name, described through the whole function.
    Relation(&'static str),
    /// A relation whose `kind` column holds a closed set: the report carries its words.
    Words(&'static str),
    /// A failure of the core, offered to the conversion into the caller's.
    CoreFails(CoreError),
}

/// What a row gives the schemas.
type Input = Offered;
/// How many relations the report describes under the name asked for.
type Value = usize;
/// Why the schemas end in error.
type Failure = DescribeError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_report_describes_the_relation_asked_for"),
        word: Word::Succeeds,
        input: || Offered::Relation("function"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_closed_column_carries_its_words_across"),
        word: Word::Succeeds,
        input: || Offered::Words("defines"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("fails_when_the_language_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Language {
                reason: String::new(),
            })
        },
        expect: Expect::Refusal(|error| selects!(error, DescribeError::Language { .. })),
    },
];

/// The language that reads nothing: what a relation is does not depend on a language.
fn nothing() -> crate::domain::language::types::Language {
    crate::resources::languages::link(
        crate::domain::language::types::LanguageName("nothing".into()),
        crate::domain::language::types::LanguageBuild("0".into()),
        Arc::new(ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl),
        Arc::new(ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl),
    )
}

/// Describes the relation and counts the report as one when it carries the name asked for, or converts the core's failure.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Offered::Relation(name) => describe(
            Languages(vec![nothing()]),
            DescribeRequest {
                name: RelationName(name.into()),
            },
        )
        .map(|report| usize::from(&*report.relation.name.0 == name)),
        Offered::Words(name) => describe(
            Languages(vec![nothing()]),
            DescribeRequest {
                name: RelationName(name.into()),
            },
        )
        .map(|report| {
            usize::from(report.relation.columns.iter().any(|column| {
                &*column.name.0 == "kind" && column.words.iter().any(|word| &*word.0 == "function")
            }))
        }),
        Offered::CoreFails(error) => Err(DescribeError::from(DescribeRelationErrorSchema(error))),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
