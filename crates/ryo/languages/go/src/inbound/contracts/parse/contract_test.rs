//! The `parse` contract's own cases, beside the engine's conformance: the engine's refusals run through it over the fakes of the syntax and notes ports, and a file it does not recognize is refused.

use std::sync::Arc;

use ryo::languages::{self, contract};

use super::contract::ParseContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::notes::fixtures::fake::NotesFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;

/// What a row does: reads a file at a path, or runs the engine's refusals.
#[derive(Clone, Copy)]
enum Given {
    Read(&'static str),
    Contract,
}

type Input = Given;
/// How many elements the file's table has.
type Value = usize;
type Failure = languages::ParseError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("passes_the_engines_refusals"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_the_language_does_not_recognize_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("source"),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

/// Reads one file, or runs the engine's refusals, through the contract over the fake of the syntax port.
fn subject(given: Input) -> Result<Value, Failure> {
    let language = ParseContract::new(Arc::new(SyntaxFake), Arc::new(NotesFake));
    let Given::Read(path) = given else {
        contract::parse(&language);
        return Ok(0);
    };
    let (mut table, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    contract::read(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath(path),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(b"source"),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut table,
        &mut symbols,
        &[],
        &mut contract::Collect::default(),
    )?;
    Ok(contract::element_count(&table).0)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
