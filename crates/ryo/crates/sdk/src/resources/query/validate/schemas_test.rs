//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use ryo_core::inbound::use_cases::validate::errors::ValidateError as CoreError;
use ryo_core::inbound::use_cases::validate::types::ValidateRequest as CoreRequest;
use ryo_language::domain::relations::types::{CellKind, Offset, Span as CoreSpan};
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::plan_parts::types::{QueryText, RefusalMessage, RefusedKind, RefusedQuery};

use super::{error, request};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::host::types::{
    HostColumn, HostColumnName, HostDeclaration, HostKind, HostRelationName,
};
use crate::domain::query::errors::ValidateError;
use crate::domain::query::types::RyoQlQuery;
use crate::resources::query::validate::types::ValidateRequest;

/// The core's error offered to the conversion.
type Input = CoreError;
/// The byte the SDK's error says the text goes wrong at.
type Value = usize;
/// Why the schemas end in error.
type Failure = ValidateError;

/// The query text the rows validate.
const TEXT: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";

/// A refusal of the text of `kind` from byte `start` to byte `end`.
fn refused(kind: RefusedKind, start: u32, end: u32) -> RefusedQuery {
    RefusedQuery {
        kind,
        message: RefusalMessage(String::new()),
        span: CoreSpan {
            start: Offset(start),
            end: Offset(end),
        },
    }
}

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_syntax_error_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Syntax, 3, 5),
        },
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("an_unknown_relation_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::UnknownRelation, 7, 14),
        },
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("a_type_error_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Type, 2, 4),
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_invalid_query_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Invalid, 5, 6),
        },
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("a_word_for_no_kind_makes_the_whole_text_invalid"),
        word: Word::Succeeds,
        input: || CoreError::UnknownKind {
            reason: String::new(),
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_correct_query_that_cannot_be_compiled_makes_the_whole_text_invalid"),
        word: Word::Succeeds,
        input: || CoreError::Uncompilable {
            error: refused(RefusedKind::Invalid, 5, 6),
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("fails_when_the_catalog_fails"),
        word: Word::Fails,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Catalog, 0, 0),
        },
        expect: Expect::Error(ValidateError::Catalog {
            reason: String::new(),
        }),
    },
];

/// Converts one error of the core and answers where the SDK's says the text goes wrong.
fn subject(input: Input) -> Result<Value, Failure> {
    match error(TEXT, input) {
        ValidateError::Syntax { span, .. }
        | ValidateError::UnknownRelation { span, .. }
        | ValidateError::Type { span, .. }
        | ValidateError::Invalid { span, .. } => Ok(span.start.0),
        error @ ValidateError::Catalog { .. } => Err(error),
    }
}

/// What a row of the requests gives: the relations the caller declares, and the ones the core is expected to be told of.
type Declared = (Vec<HostDeclaration>, Vec<HostedRelation>);

/// The rows of the requests: the core's request is the one the command line builds for the same text when the caller declares
/// nothing (the command line declares none), and carries each relation the caller declares with its columns. The two edges hand the
/// one use case the same request, so they answer alike.
const REQUESTS: &[Case<Declared, bool, Failure>] = &[
    Case {
        name: CaseName("with_no_declaration_the_request_is_the_command_lines"),
        word: Word::Succeeds,
        input: || (Vec::new(), Vec::new()),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_declared_relation_is_handed_on_with_its_columns"),
        word: Word::Succeeds,
        input: || {
            (
                vec![HostDeclaration {
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
                vec![HostedRelation {
                    name: HostedName("host.placed".into()),
                    columns: vec![
                        HostedColumn {
                            name: ColumnLabel("path".into()),
                            kind: CellKind::File,
                        },
                        HostedColumn {
                            name: ColumnLabel("spec".into()),
                            kind: CellKind::Symbol,
                        },
                    ],
                }],
            )
        },
        expect: Expect::Value(true),
    },
];

/// Builds the core's request for `TEXT` and the declarations, and answers whether it is the one expected.
fn requested((declared, expected): Declared) -> bool {
    let query = RyoQlQuery::try_from(TEXT).expect("the text is not blank");
    request(ValidateRequest {
        query,
        hosted: declared,
    }) == CoreRequest {
        text: QueryText(TEXT.into()),
        hosted: expected,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(REQUESTS, Word::Succeeds, |row| {
        Ok::<_, Failure>(requested(row))
    });
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
