//! Source-coordinate boundaries and index reuse.

use super::service::{column, index, line};
use super::types::{
    SourceByteColumn, SourceLineBytes, SourceLineIndex, SourceLineNumber, SourceLineOffset,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = fn() -> bool;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode_and_all_line_endings_use_byte_endpoints"),
        word: Word::Succeeds,
        input: || {
            || {
                let mut elements = SourceLineIndex::default();
                index(&mut elements, SourceLineBytes("é\r\nx\ry\nz".as_bytes()));
                [(0, 1), (2, 1), (3, 1), (4, 2), (6, 3), (8, 4), (9, 4)]
                    .into_iter()
                    .all(|(offset, expected)| {
                        line(&elements, SourceLineOffset(offset)) == SourceLineNumber(expected)
                    })
                    && [(0, 0), (2, 2), (3, 3), (4, 0), (6, 0), (8, 0), (9, 1)]
                        .into_iter()
                        .all(|(offset, expected)| {
                            column(&elements, SourceLineOffset(offset))
                                == SourceByteColumn(expected)
                        })
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_source_trailing_newline_and_reused_file_index"),
        word: Word::Succeeds,
        input: || {
            || {
                let mut elements = SourceLineIndex::default();
                index(&mut elements, SourceLineBytes(b""));
                let empty = line(&elements, SourceLineOffset(0)) == SourceLineNumber(1)
                    && column(&elements, SourceLineOffset(0)) == SourceByteColumn(0);
                elements.0.clear();
                index(&mut elements, SourceLineBytes(b"x\n"));
                let endpoints = line(&elements, SourceLineOffset(1)) == SourceLineNumber(1)
                    && line(&elements, SourceLineOffset(2)) == SourceLineNumber(2)
                    && column(&elements, SourceLineOffset(2)) == SourceByteColumn(0);
                elements.0.clear();
                index(&mut elements, SourceLineBytes(b"next file"));
                empty
                    && endpoints
                    && line(&elements, SourceLineOffset(8)) == SourceLineNumber(1)
                    && column(&elements, SourceLineOffset(8)) == SourceByteColumn(8)
            }
        },
        expect: Expect::Value(true),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
