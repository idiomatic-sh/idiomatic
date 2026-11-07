//! What the `parse` contract promises, against the fake of the syntax port and against the language's own adapter.

use ryo::languages::{self, Parse as _};
use std::sync::Arc;

use ryo::languages::contract::{flow, parse};

use super::contract::ParseContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::fixtures::fake::SyntaxFake;

/// What a row does: reads a file (its path, its bytes and the relations wanted), or runs the language contract.
#[derive(Clone, Copy)]
enum Given {
    Read(&'static str, &'static [u8], &'static [languages::Relation]),
    Contract,
}
/// How many elements the file's table has, and how many syntax tree rows were written; for the language contract, how many samples it
/// flowed and how many kinds the language has.
type Value = (usize, usize);
type Input = Given;
type Failure = languages::ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("README.md", b"sample", &[languages::Relation::SyntaxTree]),
        expect: Expect::Value((3, 3)),
    },
    Case {
        name: CaseName("passes_the_language_contract"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value((3, 43)),
    },
    Case {
        name: CaseName("builds_only_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("README.md", b"sample", &[]),
        expect: Expect::Value((3, 0)),
    },
    Case {
        name: CaseName("a_file_in_another_language_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("src/lib.rs", b"sample", &[languages::Relation::SyntaxTree]),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let (path, bytes, wanted) = match given {
        Given::Read(path, bytes, wanted) => (path, bytes, wanted),
        Given::Contract => {
            let language =
                ParseContract::new(Arc::new(crate::outbound::syntax::factory::pulldown_cmark()));
            parse(&language);
            flow(&language, SAMPLES);
            return Ok((SAMPLES.len(), language.kind_count().0));
        }
    };
    let language = ParseContract::new(Arc::new(SyntaxFake));
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
        out.count(languages::Relation::SyntaxTree),
    ))
}

/// The files the flow of the contract reads: a README with headings, a list, code and a link, a document with front matter, a
/// table and an HTML comment, and a file whose last bytes are not text.
const SAMPLES: &[languages::SourceFile] = &[
    languages::SourceFile { path: languages::SourcePath("README.md"), bytes: languages::ByteSlice(b"# demo\n\nA *small* tool.\n\n- one\n- two\n\n```sh\ncargo run\n```\n\nSee [docs](docs/index.md).\n") },
    languages::SourceFile { path: languages::SourcePath("docs/page.markdown"), bytes: languages::ByteSlice(b"---\ntitle: x\n---\n<!-- note -->\n\n| a | b |\n|---|---|\n| 1 | 2 |\n") },
    languages::SourceFile { path: languages::SourcePath("broken.md"), bytes: languages::ByteSlice(b"# good\n\xff\xfe\n") },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
