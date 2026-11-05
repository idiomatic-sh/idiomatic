//! What the `parse` contract promises, against the fake of the syntax port.

use crate::domain::nodes::types::RustNode;
use crate::domain::tokens::types::Token;
use ryo::languages;
use std::sync::Arc;

use ryo::languages::contract::{flow, parse};

use super::contract::ParseContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::script::{Piece, node, token};

/// What a row does: reads a file (its path, its bytes and the relations wanted), or runs the language contract.
#[derive(Clone, Copy)]
enum Given {
    Read(&'static str, &'static [u8], &'static [languages::Relation]),
    Contract,
    /// A function followed by two bytes that are not text, read by the real adapter: how many syntax errors, and how many functions.
    Unreadable,
}
/// How many elements the file's table has, and how many function rows were written.
type Value = (usize, usize);
type Input = Given;
type Failure = languages::ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("src/lib.rs", b"sample", &[languages::Relation::Function]),
        expect: Expect::Value((4, 1)),
    },
    Case {
        name: CaseName("passes_the_language_contract"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("builds_only_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("src/lib.rs", b"sample", &[]),
        expect: Expect::Value((4, 0)),
    },
    Case {
        name: CaseName("a_file_in_another_language_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("script.py", b"sample", &[languages::Relation::Function]),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
    Case {
        name: CaseName("bytes_that_are_not_text_are_read_after_the_text_that_is"),
        word: Word::Succeeds,
        input: || Given::Unreadable,
        expect: Expect::Value((2, 1)),
    },
];

/// The tree the fake makes of the bytes `sample`: a file holding one function named for them.
fn sample() -> Piece {
    node(
        Kind::Node(RustNode::SourceFile),
        vec![node(
            Kind::Node(RustNode::Fn),
            vec![node(
                Kind::Node(RustNode::Name),
                vec![token(Kind::Token(Token::Ident), "sample")],
            )],
        )],
    )
}

fn subject(given: Input) -> Result<Value, Failure> {
    let (path, bytes, wanted) = match given {
        Given::Read(path, bytes, wanted) => (path, bytes, wanted),
        Given::Unreadable => {
            let language = ParseContract::new(
                Arc::new(crate::outbound::syntax::factory::ra_ap_syntax(
                    crate::outbound::syntax::types::Expanding(
                        &crate::domain::syntax::constants::EXPANDED_MACROS,
                    ),
                )),
                Arc::new(crate::outbound::notes::factory::postcard()),
            );
            let mut out = languages::contract::Collect::default();
            let (mut table, mut symbols) = (
                languages::Elements::default(),
                languages::Symbols::default(),
            );
            languages::contract::read(
                &language,
                languages::ParseRequest {
                    path: languages::SourcePath("src/lib.rs"),
                    file: languages::FileId(0),
                    bytes: languages::ByteSlice(b"fn a() {}\xff\xfe"),
                    notes: languages::NoteRequest::Unwanted,
                    fields: languages::FieldRequest::Unwanted,
                },
                &mut table,
                &mut symbols,
                &[languages::Relation::Function],
                &mut out,
            )?;
            return Ok((
                languages::diagnostic_count(&table).0,
                out.count(languages::Relation::Function),
            ));
        }
        Given::Contract => {
            let language = ParseContract::new(
                Arc::new(crate::outbound::syntax::factory::ra_ap_syntax(
                    crate::outbound::syntax::types::Expanding(
                        &crate::domain::syntax::constants::EXPANDED_MACROS,
                    ),
                )),
                Arc::new(crate::outbound::notes::factory::postcard()),
            );
            parse(&language);
            flow(&language, SAMPLES);
            return Ok((0, 0));
        }
    };
    let language = ParseContract::new(
        Arc::new(SyntaxFake(sample())),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
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
        out.count(languages::Relation::Function),
    ))
}

/// The files the flow of the contract reads: a library file with an impl, a nested function, and a file the parser can only half read.
const SAMPLES: &[languages::SourceFile] = &[
    languages::SourceFile { path: languages::SourcePath("src/lib.rs"), bytes: languages::ByteSlice(b"pub struct P;\nimpl P {\n    pub fn new() -> Self {\n        fn inner() {}\n        Self\n    }\n}\n") },
    languages::SourceFile { path: languages::SourcePath("src/main.rs"), bytes: languages::ByteSlice(b"fn main() {\n    let f = |x: u32| x + 1;\n    println!(\"{}\", f(1));\n}\n") },
    languages::SourceFile { path: languages::SourcePath("src/broken.rs"), bytes: languages::ByteSlice(b"fn good() {}\nfn broken(") },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
