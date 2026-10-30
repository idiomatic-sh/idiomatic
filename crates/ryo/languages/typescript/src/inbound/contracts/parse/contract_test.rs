//! The `parse` contract's own cases, beside the engine's conformance: the engine's refusals run through it over the syntax port's fake, and a file it does not recognize is refused.

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
    /// A file of another format, read through the real syntax and embedded adapters: its path, and its text.
    Embedded(&'static str, &'static [u8]),
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
    Case {
        name: CaseName("a_file_with_a_typescript_block_is_read"),
        word: Word::Succeeds,
        input: || {
            Given::Embedded(
                "source.vue",
                b"<script lang=\"ts\">const a: number = 1;</script>",
            )
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_with_a_tsx_block_is_read_with_jsx"),
        word: Word::Succeeds,
        input: || {
            Given::Embedded(
                "source.svelte",
                b"<script lang=\"tsx\">const a = <b />;</script>",
            )
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_with_only_script_blocks_is_the_script_languages"),
        word: Word::Refuses,
        input: || Given::Embedded("source.vue", b"<script>const a = 1;</script>"),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
    Case {
        name: CaseName("a_file_with_no_script_is_the_script_languages"),
        word: Word::Refuses,
        input: || Given::Embedded("source.svelte", b"<p>hi</p>"),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

/// Reads one file, or runs the engine's refusals, through the contract over the fake of the syntax port.
fn subject(given: Input) -> Result<Value, Failure> {
    let (language, path, bytes, mode) = match given {
        Given::Contract => {
            contract::parse(&ParseContract::new(
                Arc::new(SyntaxFake),
                Vec::new(),
                Arc::new(NotesFake),
            ));
            return Ok(0);
        }
        Given::Read(path) => (
            ParseContract::new(Arc::new(SyntaxFake), Vec::new(), Arc::new(NotesFake)),
            path,
            b"source".as_slice(),
            false,
        ),
        Given::Embedded(path, bytes) => (
            ParseContract::new(
                Arc::new(crate::outbound::syntax::factory::oxc_parser()),
                crate::fixtures::contracts::embedded(),
                Arc::new(crate::outbound::notes::factory::postcard()),
            ),
            path,
            bytes,
            true,
        ),
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
            bytes: languages::ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut table,
        &mut symbols,
        &[],
        &mut contract::Collect::default(),
    )?;
    Ok(if mode {
        usize::from(languages::diagnostics(&table).next().is_some())
    } else {
        contract::element_count(&table).0
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
