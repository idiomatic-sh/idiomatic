//! The `keys` service's cases: a key of the same hash and another text takes the next probe, so the model of a document is the same
//! when every key has the same hash.

use super::service::{put, slot};
use super::types::{Child, KeyHash, Probe};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::documents::service::build;
use crate::domain::model::types::Model;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::fixtures::contracts::parse;
use ryo::languages::{self, Position};
use std::collections::HashMap;

/// What a row reads.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// A document whose first two keys are compared, and how keys are hashed.
    Keys(&'static str, KeyHash),
    /// A document whose model is worked out with every key of the same hash.
    Model(&'static str),
}

/// What a row answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// The probe the second key takes, and whether it is a key the document said before.
    Probe(u32, bool),
    /// How many entries the model has, and whether it is the same under both hashers.
    Same(usize, bool),
}

type Input = Given;
type Value = Answer;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_of_the_same_hash_and_another_text_takes_the_next_probe"),
        word: Word::Succeeds,
        input: || Given::Keys("a = 1\nb = 2\n", KeyHash::Same),
        expect: Expect::Value(Answer::Probe(1, false)),
    },
    Case {
        name: CaseName("the_same_text_is_the_same_key"),
        word: Word::Succeeds,
        input: || Given::Keys("a = 1\na = 2\n", KeyHash::Same),
        expect: Expect::Value(Answer::Probe(0, true)),
    },
    Case {
        name: CaseName("keys_of_other_hashes_take_the_first_probe"),
        word: Word::Succeeds,
        input: || Given::Keys("a = 1\nb = 2\n", KeyHash::Standard),
        expect: Expect::Value(Answer::Probe(0, false)),
    },
    Case {
        name: CaseName("keys_of_the_same_hash_and_other_texts_are_other_keys_in_the_model"),
        word: Word::Succeeds,
        input: || {
            Given::Model(
                "a.b = 1\na.c = 2\n\"q k\" = 3\n[x.y]\nz = 4\n[x]\nw = 5\n[[t]]\n[[t]]\nv = [1, {u = 2}]\n",
            )
        },
        expect: Expect::Value(Answer::Same(11, true)),
    },
];

/// The tree of `text`, read with the language's own parse.
fn parsed(text: &'static str) -> Result<(languages::Elements, languages::Symbols), Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let request = languages::ParseRequest {
        path: languages::SourcePath("keys.toml"),
        file: languages::FileId(0),
        bytes: languages::ByteSlice(text.as_bytes()),
        notes: languages::NoteRequest::Unwanted,
        fields: languages::FieldRequest::Unwanted,
    };
    languages::contract::build(&parse(), request, &mut elements, &mut symbols).map_err(drop)?;
    Ok((elements, symbols))
}

fn subject(given: Input) -> Result<Value, Failure> {
    let (text, how) = match given {
        Given::Keys(text, how) => (text, how),
        Given::Model(text) => {
            let (elements, _) = parsed(text)?;
            let tree = languages::Tree {
                elements: &elements,
                source: languages::ByteSlice(text.as_bytes()),
                file: languages::FileId(0),
            };
            let mut standard = Model::default();
            build(&tree, &mut standard);
            let mut colliding = Model {
                hash: KeyHash::Same,
                ..Model::default()
            };
            build(&tree, &mut colliding);
            return Ok(Answer::Same(
                standard.entries.len(),
                standard.entries == colliding.entries
                    && standard.mappings == colliding.mappings
                    && standard.elements == colliding.elements,
            ));
        }
    };
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let request = languages::ParseRequest {
        path: languages::SourcePath("keys.toml"),
        file: languages::FileId(0),
        bytes: languages::ByteSlice(text.as_bytes()),
        notes: languages::NoteRequest::Unwanted,
        fields: languages::FieldRequest::Unwanted,
    };
    languages::contract::build(&parse(), request, &mut elements, &mut symbols).map_err(drop)?;
    let tree = languages::Tree {
        elements: &elements,
        source: languages::ByteSlice(text.as_bytes()),
        file: languages::FileId(0),
    };
    let mut keys = languages::positions(&elements)
        .filter(|at| kind(&tree, *at) == Kind::Token(Token::UnquotedKey));
    let (first, second) = (keys.next().ok_or(())?, keys.next().ok_or(())?);
    let mut children = HashMap::new();
    let opened = slot(&tree, &children, how, Position(0), first);
    put(&mut children, opened, first, Child::Value);
    let found = slot(&tree, &children, how, Position(0), second);
    let Probe(probe) = found.key.probe;
    Ok(Answer::Probe(probe, found.child.is_some()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
