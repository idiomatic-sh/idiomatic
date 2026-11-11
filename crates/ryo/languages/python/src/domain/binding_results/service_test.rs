//! The `binding_results` service's cases: the text each binding vocabulary writes, and when a builtin a read resolves to became
//! available.

use super::service::{availability, certainty, context, event, relationship, state};
use super::types::{EventKind, ReadContext, ReadState, ShadowRelationship, UseCertainty};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::Integer;

/// Which vocabulary a row writes out.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Every kind of binding event.
    Events,
    /// Every read state, with a builtin since 3.10.
    States,
    /// Every read context, use certainty and shadow relationship.
    Relations,
    /// The availability of each read state: a builtin's version, else none.
    Availabilities,
}
/// What a row is given.
type Input = Given;
/// The texts the vocabulary writes, in order.
type Value = &'static str;
/// Naming is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_event_kind_has_its_text"),
        word: Word::Succeeds,
        input: || Given::Events,
        expect: Expect::Value("wildcard,read,bind,annotation,delete,cleanup,global,nonlocal"),
    },
    Case {
        name: CaseName("every_read_state_has_its_text"),
        word: Word::Succeeds,
        input: || Given::States,
        expect: Expect::Value(
            "unique-declaration,alternative-declarations,possibly-unbound,unbound,uncertain,builtin,declaration-or-builtin",
        ),
    },
    Case {
        name: CaseName("contexts_certainties_and_shadows_have_their_text"),
        word: Word::Succeeds,
        input: || Given::Relations,
        expect: Expect::Value("value,type,deletion,unique,alternative,replacement,lexical"),
    },
    Case {
        name: CaseName("only_a_builtin_read_has_an_availability"),
        word: Word::Succeeds,
        input: || Given::Availabilities,
        expect: Expect::Value("0.0,0.0,0.0,0.0,0.0,3.10,3.10"),
    },
];

/// Every read state, a builtin and a declaration-or-builtin since 3.10.
const STATES: [ReadState; 7] = [
    ReadState::UniqueDeclaration,
    ReadState::AlternativeDeclarations,
    ReadState::PossiblyUnbound,
    ReadState::Unbound,
    ReadState::Uncertain,
    ReadState::Builtin {
        since_major: Integer(3),
        since_minor: Integer(10),
    },
    ReadState::DeclarationOrBuiltin {
        since_major: Integer(3),
        since_minor: Integer(10),
    },
];

/// Writes out the vocabulary the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let texts: Vec<String> = match given {
        Given::Events => [
            EventKind::Wildcard,
            EventKind::Read,
            EventKind::Bind,
            EventKind::Annotation,
            EventKind::Delete,
            EventKind::Cleanup,
            EventKind::Global,
            EventKind::Nonlocal,
        ]
        .into_iter()
        .map(|each| event(each).0.to_owned())
        .collect(),
        Given::States => STATES
            .into_iter()
            .map(|each| state(each).0.to_owned())
            .collect(),
        Given::Relations => [ReadContext::Value, ReadContext::Type, ReadContext::Deletion]
            .into_iter()
            .map(|each| context(each).0.to_owned())
            .chain(
                [UseCertainty::Unique, UseCertainty::Alternative]
                    .into_iter()
                    .map(|each| certainty(each).0.to_owned()),
            )
            .chain(
                [ShadowRelationship::Replacement, ShadowRelationship::Lexical]
                    .into_iter()
                    .map(|each| relationship(each).0.to_owned()),
            )
            .collect(),
        Given::Availabilities => STATES
            .into_iter()
            .map(|each| {
                let since = availability(each);
                format!("{}.{}", since.major.0, since.minor.0)
            })
            .collect(),
    };
    Ok(Box::leak(texts.join(",").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
