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
    /// Reads a stylesheet with the language's own adapter and its roles asked for.
    Roles(&'static [u8]),
}
/// How many elements the file's table has, and how many comment rows were written; for the language contract, how many samples it
/// flowed and how many kinds the language has; for the roles, how many elements are in the role `important` and how many in `name`.
type Value = (usize, usize);
type Input = Given;
type Failure = languages::ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("site.css", b"sample", &[languages::Relation::Comment]),
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("passes_the_language_contract"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value((3, 34)),
    },
    Case {
        name: CaseName("a_declaration_has_its_name_and_its_importance_in_roles_of_their_own"),
        word: Word::Succeeds,
        input: || {
            Given::Roles(b"a { color: red !IMPORTANT; margin: 0 /* x */ ! important } b { c: d }")
        },
        expect: Expect::Value((2, 3)),
    },
    Case {
        name: CaseName("a_bang_with_another_word_is_part_of_the_value"),
        word: Word::Succeeds,
        input: || Given::Roles(b"a { color: red ! notimportant }"),
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("builds_only_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("site.css", b"sample", &[]),
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("a_file_in_another_language_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("site.scss", b"sample", &[languages::Relation::Comment]),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let (path, bytes, wanted) = match given {
        Given::Read(path, bytes, wanted) => (path, bytes, wanted),
        Given::Roles(bytes) => return roles(bytes),
        Given::Contract => {
            let language =
                ParseContract::new(Arc::new(crate::outbound::syntax::factory::cssparser()));
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
        out.count(languages::Relation::Comment),
    ))
}

/// How many elements of `bytes`, read with the language's own adapter and its roles asked for, are in the role `important`, and how
/// many in the role `name`.
fn roles(bytes: &'static [u8]) -> Result<Value, Failure> {
    use crate::domain::syntax::service::field_bits;
    use crate::domain::syntax::types::Field;
    let language = ParseContract::new(Arc::new(crate::outbound::syntax::factory::cssparser()));
    let mut out = languages::contract::Collect::default();
    let (mut table, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::read(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("site.css"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut table,
        &mut symbols,
        &[languages::Relation::SyntaxChild],
        &mut out,
    )?;
    let count = |field: Field| {
        languages::positions(&table)
            .filter(|at| languages::field_at(&table, *at) == field_bits(field))
            .count()
    };
    Ok((count(Field::Important), count(Field::Name)))
}

/// The files the flow of the contract reads: a stylesheet with imports, rules and a conditional rule, one with nesting and urls, and
/// one the parser can only half read.
const SAMPLES: &[languages::SourceFile] = &[
    languages::SourceFile { path: languages::SourcePath("site.css"), bytes: languages::ByteSlice(b"@import \"base.css\";\n/* the card */\n.card, #hero { color: red !important; }\n@media (min-width: 40em) { .card { padding: 1rem; } }\n") },
    languages::SourceFile { path: languages::SourcePath("theme.css"), bytes: languages::ByteSlice(b".a { background: url(img/a.png); &:hover { color: rgb(0 0 0 / 50%); } }\n@font-face { font-family: X; src: url(\"x.woff2\") format(\"woff2\"); }\n") },
    languages::SourceFile { path: languages::SourcePath("broken.css"), bytes: languages::ByteSlice(b"good { a: b }\nbroken\n.card { color: \"open\n}\n") },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
