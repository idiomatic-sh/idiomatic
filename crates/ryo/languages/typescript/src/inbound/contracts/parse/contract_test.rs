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
    /// A file read through the real adapters, answered with whether the parser accepted it: its path, and its text.
    Accepted(&'static str, &'static [u8]),
}

type Input = Given;
/// What a row reads of the file's table.
type Value = Answer;

/// What a row reads of the file's table: a count, or the parser's acceptance.
#[derive(Debug, PartialEq)]
enum Answer {
    /// How many elements the table has, or for an embedded file, whether it holds a diagnostic.
    Count(usize),
    /// Whether the parser accepted the file.
    Acceptance(languages::SyntaxAcceptance),
}
type Failure = languages::ParseError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("passes_the_engines_refusals"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value(Answer::Count(0)),
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
        expect: Expect::Value(Answer::Count(0)),
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
        expect: Expect::Value(Answer::Count(0)),
    },
    Case {
        name: CaseName("a_file_with_only_script_blocks_is_read_as_javascript"),
        word: Word::Succeeds,
        input: || Given::Embedded("source.vue", b"<script>const a = 1;</script>"),
        expect: Expect::Value(Answer::Count(0)),
    },
    Case {
        name: CaseName("a_file_with_no_script_is_the_script_languages"),
        word: Word::Refuses,
        input: || Given::Embedded("source.svelte", b"<p>hi</p>"),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
    Case {
        name: CaseName("a_file_the_parser_reads_without_error_is_accepted"),
        word: Word::Succeeds,
        input: || Given::Accepted("source.ts", b"const a: number = 1;\n"),
        expect: Expect::Value(Answer::Acceptance(languages::SyntaxAcceptance::Accepted)),
    },
    Case {
        name: CaseName("a_file_with_a_parse_error_is_rejected"),
        word: Word::Succeeds,
        input: || Given::Accepted("w.ts", b"function f(o){ with (o) { return x; } }\n"),
        expect: Expect::Value(Answer::Acceptance(languages::SyntaxAcceptance::Rejected)),
    },
    Case {
        name: CaseName("a_file_with_an_error_the_semantic_check_finds_is_rejected"),
        word: Word::Succeeds,
        input: || Given::Accepted("k.ts", b"let a = 1;\nlet a = 2;\n"),
        expect: Expect::Value(Answer::Acceptance(languages::SyntaxAcceptance::Rejected)),
    },
    Case {
        name: CaseName("a_file_that_is_not_utf8_is_rejected"),
        word: Word::Succeeds,
        input: || Given::Accepted("source.ts", b"const a = 1;\xff\n"),
        expect: Expect::Value(Answer::Acceptance(languages::SyntaxAcceptance::Rejected)),
    },
];

/// Reads one file, or runs the engine's refusals, through the contract over the fake of the syntax port.
fn subject(given: Input) -> Result<Value, Failure> {
    let (language, path, bytes, read) = match given {
        Given::Contract => {
            contract::parse(&ParseContract::new(
                Arc::new(SyntaxFake),
                Vec::new(),
                Arc::new(NotesFake),
            ));
            return Ok(Answer::Count(0));
        }
        Given::Read(path) => (
            ParseContract::new(Arc::new(SyntaxFake), Vec::new(), Arc::new(NotesFake)),
            path,
            b"source".as_slice(),
            Read::Elements,
        ),
        Given::Embedded(path, bytes) => (real(), path, bytes, Read::Diagnosed),
        Given::Accepted(path, bytes) => (real(), path, bytes, Read::Acceptance),
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
    Ok(match read {
        Read::Elements => Answer::Count(contract::element_count(&table).0),
        Read::Diagnosed => {
            Answer::Count(usize::from(languages::diagnostics(&table).next().is_some()))
        }
        Read::Acceptance => Answer::Acceptance(languages::syntax_acceptance(&table)),
    })
}

/// What a row reads of the table it is given.
#[derive(Clone, Copy)]
enum Read {
    Elements,
    Diagnosed,
    Acceptance,
}

/// The contract over the real syntax, embedded and notes adapters.
fn real() -> ParseContract {
    ParseContract::new(
        Arc::new(crate::outbound::syntax::factory::oxc_parser()),
        crate::fixtures::contracts::embedded(),
        Arc::new(crate::outbound::notes::factory::postcard()),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
