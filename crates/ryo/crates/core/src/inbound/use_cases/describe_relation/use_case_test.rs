//! What the `describe_relation` use case promises, against the fake of the language port.

use std::sync::Arc;

use super::errors::DescribeRelationError;
use super::interface::DescribeRelationUseCase as _;
use super::types::DescribeRelationRequest;
use super::use_case::DescribeRelationUseCaseImpl;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_values::domain::descriptions::types::RelationReadability;

use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::outbound::language::fixtures::fake::{FakeCatalog, LanguageFake};
use crate::outbound::language::types::Absence;
use ryo_values::domain::descriptions::types::RelationName;

/// The name asked for, what the catalog answers, and whether the one linked language says the relation is not in it.
type Input = (&'static str, FakeCatalog, bool);
/// How many columns the relation has, how many linked languages answer, and how many of them do not read it.
type Value = (usize, usize, usize);
type Failure = DescribeRelationError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("describes_one_relation"),
        word: Word::Succeeds,
        input: || ("function", FakeCatalog::Lists, false),
        expect: Expect::Value((4, 1, 0)),
    },
    Case {
        name: CaseName("says_which_linked_language_does_not_read_it"),
        word: Word::Succeeds,
        input: || ("function", FakeCatalog::Lists, true),
        expect: Expect::Value((4, 1, 1)),
    },
    Case {
        name: CaseName("a_relation_the_engine_gives_is_read_by_no_language"),
        word: Word::Succeeds,
        input: || ("filesystem.file", FakeCatalog::Lists, false),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || ("nothing", FakeCatalog::Lists, false),
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                DescribeRelationError::UnknownRelation { .. }
            )))
        }),
    },
    Case {
        name: CaseName("fails_when_language_port_fails"),
        word: Word::Fails,
        input: || ("function", FakeCatalog::Fails, false),
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                DescribeRelationError::Language { .. }
            )))
        }),
    },
];

fn subject((name, catalog, absent): Input) -> Result<Value, Failure> {
    let absent = if absent {
        Absence::Because(AbsenceCause::NoSuchConstruct)
    } else {
        Absence::Reads
    };
    DescribeRelationUseCaseImpl::new(Arc::new(LanguageFake {
        catalog,
        absent,
        ..LanguageFake::default()
    }))
    .describe_relation(DescribeRelationRequest {
        name: RelationName(name.into()),
    })
    .map(|report| {
        (
            report.relation.columns.len(),
            report.relation.languages.len(),
            report
                .relation
                .languages
                .iter()
                .filter(|each| each.reads == RelationReadability::DoesNotRead)
                .count(),
        )
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

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
