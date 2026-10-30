//! What the `parse` contract promises, against the fakes of the syntax and notes ports and against the language's own adapters.

use ryo::languages::{self, Parse as _};
use std::sync::Arc;

use ryo::languages::contract::{flow, parse};

use super::contract::ParseContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::notes::fixtures::fake::NotesFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;

/// What a row does: reads a file (its path, its bytes and the relations wanted), or runs the language contract.
#[derive(Clone, Copy)]
enum Given {
    Read(&'static str, &'static [u8], &'static [languages::Relation]),
    Contract,
}
/// How many elements the file's table has, and how many reference rows were written; for the language contract, how many samples it
/// flowed and how many kinds the language has.
type Value = (usize, usize);
type Input = Given;
type Failure = languages::ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("app.py", b"sample", &[languages::Relation::Reference]),
        expect: Expect::Value((4, 1)),
    },
    Case {
        name: CaseName("passes_the_language_contract"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value((3, 192)),
    },
    Case {
        name: CaseName("builds_only_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("app.py", b"sample", &[]),
        expect: Expect::Value((4, 0)),
    },
    Case {
        name: CaseName("a_file_in_another_language_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("src/lib.rs", b"sample", &[languages::Relation::Reference]),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let (path, bytes, wanted) = match given {
        Given::Read(path, bytes, wanted) => (path, bytes, wanted),
        Given::Contract => {
            let language = ParseContract::new(
                Arc::new(crate::outbound::syntax::factory::ruff_python_parser()),
                Arc::new(crate::outbound::notes::factory::postcard()),
            );
            parse(&language);
            flow(&language, SAMPLES);
            return Ok((SAMPLES.len(), language.kind_count().0));
        }
    };
    let language = ParseContract::new(Arc::new(SyntaxFake), Arc::new(NotesFake));
    let mut out = languages::contract::Collect::default();
    let (mut table, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::read(
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
        wanted,
        &mut out,
    )?;
    Ok((
        languages::contract::element_count(&table).0,
        out.count(languages::Relation::Reference),
    ))
}

/// The files the flow of the contract reads: a module with definitions and imports, a class with decorators, and a file the parser
/// can only half read.
const SAMPLES: &[languages::SourceFile] = &[
    languages::SourceFile { path: languages::SourcePath("app.py"), bytes: languages::ByteSlice(b"import os\nfrom . import x as y\n\ndef run(a, *b, c=1, **d) -> int:  # run it\n    return os.path.join(a, 'x')\n") },
    languages::SourceFile { path: languages::SourcePath("model.py"), bytes: languages::ByteSlice(b"@dataclass(frozen=True)\nclass Point(Base, metaclass=Meta):\n    x: int = 0\n") },
    languages::SourceFile { path: languages::SourcePath("broken.py"), bytes: languages::ByteSlice(b"good = 1\ndef broken(\n") },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
