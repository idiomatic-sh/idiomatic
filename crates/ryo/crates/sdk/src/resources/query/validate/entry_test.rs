//! What `query validate` promises its caller, one row of `CASES` for each case it has.

use super::entry::validate;
use super::types::ValidateRequest;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::host::types::{
    HostColumn, HostColumnName, HostDeclaration, HostKind, HostRelationName,
};
use crate::domain::language::types::Languages;
use crate::domain::observation::types::{
    BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
    ObservationKind, ObservationScope,
};
use crate::domain::query::errors::ValidateError;
use crate::domain::query::types::{QueryModuleCatalog, QueryModuleSource, RyoQlQuery};

/// Whether the caller declares the observation `published` the query reads.
#[derive(Clone, Copy, Debug)]
enum Declared {
    /// The request declares it.
    Yes,
    /// The request declares nothing.
    No,
}

/// What a row asks: a query text, and for the observing rows the declaration the caller makes with it.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A query over the relations ryo owns and the one the host declared.
    Text(&'static str),
    /// A query that reads `published` and may call the module `test/forward`.
    Observing(&'static str, Declared),
}

/// The query a row asks about.
type Input = Asked;
/// How many relations the query reads.
type Value = usize;
type Failure = ValidateError;

const VALID: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
const HOSTED: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";
const READS_PUBLISHED: &str =
    "let result = select path from published; emit result subject path key(path);";
const UNIONS_PUBLISHED: &str =
    "let result = union(left=published, right=published); emit result subject path key(path);";
const FORWARDS_PUBLISHED: &str = "import forward from \"test/forward\"; let result = forward(candidates=published); emit result subject path key(path);";
const REDEFINES_PUBLISHED: &str =
    "let published = select path from \"filesystem.file\"; emit published subject path key(path);";
/// The module the observing rows import: it forwards the input it is given.
const FORWARD_MODULE: &str = "input candidates: relation<path: file>; output relation<path: file>; let published = select path from candidates; return published;";
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("returns_a_report_for_valid_text"),
        word: Word::Succeeds,
        input: || Asked::Text(VALID),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_the_host_declared_is_read_as_ryos_own_are"),
        word: Word::Succeeds,
        input: || Asked::Text(HOSTED),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_declared_observation_is_read_by_the_root_query"),
        word: Word::Succeeds,
        input: || Asked::Observing(READS_PUBLISHED, Declared::Yes),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_declared_observation_is_read_by_a_native_operator"),
        word: Word::Succeeds,
        input: || Asked::Observing(UNIONS_PUBLISHED, Declared::Yes),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_declared_observation_reaches_a_fragment_source"),
        word: Word::Succeeds,
        input: || Asked::Observing(FORWARDS_PUBLISHED, Declared::Yes),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("returns_the_error_with_its_span"),
        word: Word::Refuses,
        input: || Asked::Text(MISSPELLED),
        expect: Expect::Refusal(|error| selects(matches!(error, ValidateError::Syntax { .. }))),
    },
    Case {
        name: CaseName("an_undeclared_observation_is_refused_in_the_root_query"),
        word: Word::Refuses,
        input: || Asked::Observing(READS_PUBLISHED, Declared::No),
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, ValidateError::Located { error, .. } if matches!(**error, ValidateError::Invalid { .. })),
            )
        }),
    },
    Case {
        name: CaseName("an_undeclared_observation_is_refused_in_a_fragment_call"),
        word: Word::Refuses,
        input: || Asked::Observing(FORWARDS_PUBLISHED, Declared::No),
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, ValidateError::Located { error, .. } if matches!(**error, ValidateError::Invalid { .. })),
            )
        }),
    },
    Case {
        name: CaseName("a_query_cannot_redefine_a_declared_observation"),
        word: Word::Refuses,
        input: || Asked::Observing(REDEFINES_PUBLISHED, Declared::Yes),
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, ValidateError::Located { error, .. } if matches!(**error, ValidateError::Invalid { .. })),
            )
        }),
    },
];

/// The declaration the host makes of the relation of the placed files.
fn placed() -> HostDeclaration {
    HostDeclaration {
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
    }
}

/// The observation `published` the caller declares: the files of a run, as one column.
fn published() -> BoundObservationInput {
    BoundObservationInput {
        name: ObservationBindingName("published".into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("path".into()),
            kind: ObservationKind::File,
        }],
        scope: ObservationScope::PerFile,
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let (query, hosted, observations) = match asked {
        Asked::Text(text) => (
            RyoQlQuery::try_from(text).expect("a query"),
            vec![placed()],
            Vec::new(),
        ),
        Asked::Observing(text, declared) => {
            let module =
                QueryModuleSource::try_new("test/forward", FORWARD_MODULE).expect("a module");
            let modules = QueryModuleCatalog::try_new([module]).expect("a catalog");
            (
                RyoQlQuery::with_modules("test/root", text, &modules).expect("a query"),
                Vec::new(),
                match declared {
                    Declared::Yes => vec![published()],
                    Declared::No => Vec::new(),
                },
            )
        }
    };
    validate(
        &Languages::default(),
        ValidateRequest {
            observations,
            query,
            hosted,
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
