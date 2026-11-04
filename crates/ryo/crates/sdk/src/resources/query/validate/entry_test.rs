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
            observations: Vec::new(),
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

#[test]
fn declared_observation_context_reaches_root_native_and_fragment_sources() {
    use crate::domain::observation::types::ObservationKind;
    use crate::domain::observation::types::{
        BoundObservationSchema, ObservationBindingName, ObservationColumn, ObservationColumnName,
    };
    let declared = BoundObservationSchema {
        name: ObservationBindingName("published".into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("path".into()),
            kind: ObservationKind::File,
        }],
        scope: crate::domain::observation::types::ObservationScope::PerFile,
    };
    let module = crate::domain::query::types::QueryModuleSource::try_new(
        "test/forward",
        "input candidates: relation<path: file>; output relation<path: file>; let published = select path from candidates; return published;",
    ).unwrap();
    for (text, has_context, expected) in [
        (
            "let result = select path from published; emit result subject path key(path);",
            true,
            true,
        ),
        (
            "let result = union(left=published, right=published); emit result subject path key(path);",
            true,
            true,
        ),
        (
            "import forward from \"test/forward\"; let result = forward(candidates=published); emit result subject path key(path);",
            true,
            true,
        ),
        (
            "let result = select path from published; emit result subject path key(path);",
            false,
            false,
        ),
        (
            "import forward from \"test/forward\"; let result = forward(candidates=published); emit result subject path key(path);",
            false,
            false,
        ),
        (
            "let published = select path from \"filesystem.file\"; emit published subject path key(path);",
            true,
            false,
        ),
    ] {
        let query = RyoQlQuery::with_modules("test/root", text, [module.clone()]).unwrap();
        let result = validate(
            &Languages::default(),
            ValidateRequest {
                query,
                hosted: Vec::new(),
                observations: if has_context {
                    vec![declared.clone()]
                } else {
                    Vec::new()
                },
            },
        );
        assert_eq!(result.is_ok(), expected, "{text}: {result:?}");
    }
}
