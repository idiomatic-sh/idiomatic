//! A query bundle names its sources without ambiguity: a blank identity, a blank query and a repeated identity are refused.
use super::errors::ProgramError;
use super::service::{document_source, query_bundle, source_id, standalone};
use super::types::QuerySourceId;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    NamedSource,
    StandaloneQuery,
    BlankSource,
    BlankQuery,
    RepeatedModule,
}
type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ProgramError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_named_source_keeps_its_identity"),
        word: Word::Succeeds,
        input: || Scenario::NamedSource,
        expect: Expect::Value(Cow::Borrowed("module")),
    },
    Case {
        name: CaseName("a_standalone_query_is_the_root_source"),
        word: Word::Succeeds,
        input: || Scenario::StandaloneQuery,
        expect: Expect::Value(Cow::Borrowed("<query>")),
    },
    Case {
        name: CaseName("a_blank_source_identity_is_refused"),
        word: Word::Refuses,
        input: || Scenario::BlankSource,
        expect: Expect::Error(ProgramError::BlankSourceId),
    },
    Case {
        name: CaseName("a_blank_query_is_refused"),
        word: Word::Refuses,
        input: || Scenario::BlankQuery,
        expect: Expect::Error(ProgramError::BlankRootText),
    },
    Case {
        name: CaseName("a_module_repeating_the_root_identity_is_refused"),
        word: Word::Refuses,
        input: || Scenario::RepeatedModule,
        expect: Expect::Refusal(|error| {
            crate::selects!(matches!(error, ProgramError::DuplicateSourceId { .. }))
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::NamedSource => source_id("module").map(|id| Cow::Owned(id.0.into_string())),
        Scenario::StandaloneQuery => standalone("rows").map(|bundle| Cow::Owned(bundle.root.id.0.into_string())),
        Scenario::BlankSource => source_id("  ").map(|id| Cow::Owned(id.0.into_string())),
        Scenario::BlankQuery => standalone("  ").map(|bundle| Cow::Owned(bundle.root.id.0.into_string())),
        Scenario::RepeatedModule => {
            let root = document_source(QuerySourceId("root".into()), "rows");
            let module = document_source(QuerySourceId("root".into()), "more");
            query_bundle(root, [module]).map(|bundle| Cow::Owned(bundle.root.id.0.into_string()))
        }
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
