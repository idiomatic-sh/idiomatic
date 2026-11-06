//! The `validation` service's cases, on query text read by the real syntax adapter.

use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::validation::errors::ValidationError;
use crate::domain::validation::service::validate;
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::catalog::interface::CatalogPort as _;
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::program::types::QueryView;

/// The query text a row gives.
type Input = &'static str;
/// Whether the text is valid.
type Value = bool;
type Failure = ValidationError;

const VALID: &str = "param max-lines: integer;\nlet long = select * from \"function\" where id > $max-lines;\nemit long subject file key(file);";
const UNKNOWN_RELATION: &str = "let a = select * from \"nothing\"; emit a subject file key(file);";
const UNKNOWN_STEP: &str = "let a = select * from earlier; emit a subject file key(file);";
const UNKNOWN_PARAMETER: &str =
    "let a = select * from \"function\" where id > $limit; emit a subject file key(file);";
const UNKNOWN_OPERATION: &str = "let a = sort(from=\"function\"); emit a subject file key(file);";
const MISSING_ARGUMENT: &str = "let a = project(from=\"function\"); emit a subject file key(file);";
const DUPLICATE_STEP: &str = "let a = select * from \"function\"; let a = select * from \"function\"; emit a subject file key(file);";
const JOIN_WITHOUT_COLUMNS: &str = "let a = select * from \"function\" as l join \"function\" as r on l.id = r.id; emit a subject file key(file);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_well_formed_query_is_valid"),
        word: Word::Succeeds,
        input: || VALID,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_RELATION,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::UnknownRelation { .. }))
        }),
    },
    Case {
        name: CaseName("unknown_step_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_STEP,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::UnknownStep { .. }))
        }),
    },
    Case {
        name: CaseName("unknown_parameter_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_PARAMETER,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::UnknownParameter { .. }))
        }),
    },
    Case {
        name: CaseName("unknown_operation_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_OPERATION,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::UnknownOperation { .. }))
        }),
    },
    Case {
        name: CaseName("missing_argument_is_refused"),
        word: Word::Refuses,
        input: || MISSING_ARGUMENT,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::MissingArgument { .. }))
        }),
    },
    Case {
        name: CaseName("a_step_defined_twice_is_refused"),
        word: Word::Refuses,
        input: || DUPLICATE_STEP,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::DuplicateStep { .. }))
        }),
    },
    Case {
        name: CaseName("a_join_must_name_its_columns"),
        word: Word::Refuses,
        input: || JOIN_WITHOUT_COLUMNS,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::JoinNeedsColumns { .. }))
        }),
    },
];

/// Reads the text and validates it against the fake catalog.
fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let catalog = CatalogFake::default().relations().expect("the fake lists");
    validate(&program, &catalog.0, &[]).map(|()| true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
