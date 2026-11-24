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
    Properties(languages::NodeAccess<'static>),
    Contract,
}
/// How many elements the file's table has, and how many entry rows were written; for the language contract, how many samples it
/// flowed and how many kinds the language has.
type Value = (usize, usize);
type Input = Given;
type Failure = languages::ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("public_inspection_preserves_decoded_values"),
        word: Word::Succeeds,
        input: || Given::Properties(languages::NodeAccess::Inspect(&[])),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_comment_query_keeps_raw_spelling"),
        word: Word::Succeeds,
        input: || Given::Properties(languages::NodeAccess::Query(&[languages::Relation::Comment])),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_string_query_keeps_its_decoded_value"),
        word: Word::Succeeds,
        input: || Given::Properties(languages::NodeAccess::Query(&[languages::Relation::StringLiteral])),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("parses_a_sample_file_into_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("Cargo.toml", b"sample", &[languages::Relation::Entry]),
        expect: Expect::Value((6, 1)),
    },
    Case {
        name: CaseName("passes_the_language_contract"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value((3, 32)),
    },
    Case {
        name: CaseName("builds_only_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("Cargo.toml", b"sample", &[]),
        expect: Expect::Value((6, 0)),
    },
    Case {
        name: CaseName("a_file_in_another_language_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("src/lib.rs", b"sample", &[languages::Relation::Entry]),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

/// The same source spelling is present for every scope. Only a value reader needs the decoded tab.
fn properties(access: languages::NodeAccess<'_>) -> Result<Value, Failure> {
    let bytes = languages::ByteSlice(b"answer = \"a\\tb\"\n");
    let mut table = languages::Elements::default();
    let mut words = languages::Symbols::default();
    crate::linked().parse_engine().build_native_file(
        languages::ParseRequest {
            path: languages::SourcePath("example.toml"),
            bytes,
            file: languages::FileId(0),
            fields: languages::FieldRequest::Unwanted,
            notes: languages::NoteRequest::Unwanted,
        },
        &mut table,
        &mut words,
        access,
    )?;
    let raw = languages::positions(&table).filter(|&at| {
        languages::text_of(&words, bytes, languages::text_at(&table, at))
            .is_some_and(|value| value.0 == "\"a\\tb\"")
    }).count();
    let decoded = languages::positions(&table).filter(|&at| {
        languages::decoded_at(&table, at).is_some_and(|value| value.0 == "a\tb")
    }).count();
    Ok((raw, decoded))
}

fn subject(given: Input) -> Result<Value, Failure> {
    let (path, bytes, wanted) = match given {
        Given::Read(path, bytes, wanted) => (path, bytes, wanted),
        Given::Properties(access) => return properties(access),
        Given::Contract => {
            let language =
                ParseContract::new(Arc::new(crate::outbound::syntax::factory::toml_parser()));
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
        out.count(languages::Relation::Entry),
    ))
}

/// The files the flow of the contract reads: a manifest with tables, one with an array of tables and inline tables, and a file the
/// parser can only half read.
const SAMPLES: &[languages::SourceFile] = &[
    languages::SourceFile { path: languages::SourcePath("Cargo.toml"), bytes: languages::ByteSlice(b"[package]\nname = \"demo\" # the name\n\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\n") },
    languages::SourceFile { path: languages::SourcePath("config.toml"), bytes: languages::ByteSlice(b"a.b = 1\n[[bin]]\nname = 'x'\n[[bin]]\nname = 'y'\n") },
    languages::SourceFile { path: languages::SourcePath("broken.toml"), bytes: languages::ByteSlice(b"good = 1\nbroken = \n[unclosed\n") },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
