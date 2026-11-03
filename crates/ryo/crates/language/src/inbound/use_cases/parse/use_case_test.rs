//! What the `parse` use case promises: the contract, run against the language that reads nothing, and what reading a file comes to.

use super::errors::ParseError;
use super::fixtures::contract;
use super::interface::ParseUseCase as _;
use super::service::build;
use super::types::{FieldRequest, NoteRequest, ParseRequest, Recognition};
use super::use_case::ParseUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use ryo_relations::domain::relations::types::FileId;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;

/// What a row of the cases asks of the language that reads nothing.
#[derive(Clone, Copy)]
enum Scenario {
    /// Whether it reads a file.
    Recognizes,
    /// Reading a file through it.
    Builds,
}

/// What a row asks.
type Input = Scenario;
/// Whether the language reads the file.
type Value = Recognition;
type Failure = ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_language_that_reads_nothing_declines_a_file"),
        word: Word::Succeeds,
        input: || Scenario::Recognizes,
        expect: Expect::Value(Recognition::Declines),
    },
    Case {
        name: CaseName("reading_a_file_through_it_is_unsupported"),
        word: Word::Refuses,
        input: || Scenario::Builds,
        expect: Expect::Error(ParseError::Unsupported),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let request = ParseRequest {
        path: SourcePath("src/lib.rs"),
        file: FileId(0),
        bytes: ByteSlice(b""),
        notes: NoteRequest::Unwanted,
        fields: FieldRequest::Unwanted,
    };
    match scenario {
        Scenario::Recognizes => Ok(ParseUseCaseImpl.recognizes(&request)),
        Scenario::Builds => build(
            &ParseUseCaseImpl,
            request,
            &mut Elements::default(),
            &mut Symbols::default(),
        )
        .map(|()| Recognition::Reads),
    }
}

#[test]
fn succeeds() {
    contract::check(&ParseUseCaseImpl);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
