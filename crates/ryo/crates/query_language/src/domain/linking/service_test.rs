//! Linking names where a refusal is and which module an import reads, calling no parser.

use super::errors::LinkingError;
use super::service::{located, source_id};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use ryo_values::domain::program::types::ImportDeclaration;
use ryo_values::domain::query_tokens::types::{
    ByteOffset, QuerySourceId, QuerySpan, WrittenLiteral, WrittenName,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// An import of a module with an id.
    NamedModule,
    /// An import of a module whose id is blank.
    BlankModule,
    /// A refusal located in a source.
    LocatedRefusal,
}

/// The id of the module the import reads.
type Value = QuerySourceId;
type Failure = LinkingError;
/// What a row asks of the service.
type Input = Scenario;

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(3),
    end: ByteOffset(11),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_import_reads_the_module_it_names"),
        word: Word::Succeeds,
        input: || Scenario::NamedModule,
        expect: Expect::Value(QuerySourceId(Box::from("module"))),
    },
    Case {
        name: CaseName("an_import_of_a_blank_module_id_is_refused_at_the_import"),
        word: Word::Refuses,
        input: || Scenario::BlankModule,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LinkingError::Located { origin, .. } if origin.span == SPAN))
        }),
    },
    Case {
        name: CaseName("a_located_refusal_keeps_its_source_span_and_message"),
        word: Word::Refuses,
        input: || Scenario::LocatedRefusal,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, LinkingError::Located { origin, message } if origin.span == SPAN && origin.source_id.0.as_ref() == "root" && message == "refused")
            )
        }),
    },
];

fn import(module: &str) -> ImportDeclaration {
    ImportDeclaration {
        alias: WrittenName("alias".into()),
        module: WrittenLiteral(module.into()),
        span: SPAN,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = QuerySourceId(Box::from("root"));
    match scenario {
        Scenario::NamedModule => source_id(&root, &import("module")),
        Scenario::BlankModule => source_id(&root, &import(" ")),
        Scenario::LocatedRefusal => Err(located(&root, SPAN, Vec::new(), "refused")),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
