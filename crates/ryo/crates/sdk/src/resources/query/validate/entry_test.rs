//! What `query validate` promises its caller, one row of `CASES` for each case it has.

use super::entry::validate;
use super::types::ValidateRequest;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::host::types::{
    HostColumn, HostColumnName, HostDeclaration, HostKind, HostRelationName,
};
use crate::domain::language::types::Languages;
use crate::domain::query::errors::ValidateError;
use crate::domain::query::types::RyoQlQuery;

/// The query text.
type Input = &'static str;
/// How many relations the query reads.
type Value = usize;
type Failure = ValidateError;

const VALID: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
const HOSTED: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("returns_a_report_for_valid_text"),
        word: Word::Succeeds,
        input: || VALID,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_the_host_declared_is_read_as_ryos_own_are"),
        word: Word::Succeeds,
        input: || HOSTED,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("returns_the_error_with_its_span"),
        word: Word::Refuses,
        input: || MISSPELLED,
        expect: Expect::Refusal(|error| selects(matches!(error, ValidateError::Syntax { .. }))),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    let query = RyoQlQuery::try_from(text).expect("a query");
    validate(
        &Languages::default(),
        ValidateRequest {
            query,
            hosted: vec![HostDeclaration {
                name: HostRelationName("host.placed".into()),
                columns: vec![
                    HostColumn {
                        name: HostColumnName("path".into()),
                        kind: HostKind::File,
                    },
                    HostColumn {
                        name: HostColumnName("spec".into()),
                        kind: HostKind::Text,
                    },
                ],
            }],
        },
    )
    .map(|report| report.relations.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
