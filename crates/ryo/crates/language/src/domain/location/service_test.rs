//! Source-coordinate boundaries and index reuse.

use super::service::{index, line};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::clear_elements;
use crate::domain::elements::types::Elements;
use crate::domain::relations::types::{Integer, Offset};
use crate::domain::symbols::types::ByteSlice;

type Input = fn() -> bool;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode_and_all_line_endings_use_byte_endpoints"),
        word: Word::Succeeds,
        input: || {
            || {
                let mut elements = Elements::default();
                index(&mut elements, ByteSlice("é\r\nx\ry\nz".as_bytes()));
                [(0, 1), (2, 1), (3, 1), (4, 2), (6, 3), (8, 4), (9, 4)]
                    .into_iter()
                    .all(|(offset, expected)| line(&elements, Offset(offset)) == Integer(expected))
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_source_trailing_newline_and_reused_file_index"),
        word: Word::Succeeds,
        input: || {
            || {
                let mut elements = Elements::default();
                index(&mut elements, ByteSlice(b""));
                let empty = line(&elements, Offset(0)) == Integer(1);
                clear_elements(&mut elements);
                index(&mut elements, ByteSlice(b"x\n"));
                let endpoints = line(&elements, Offset(1)) == Integer(1)
                    && line(&elements, Offset(2)) == Integer(2);
                clear_elements(&mut elements);
                index(&mut elements, ByteSlice(b"next file"));
                empty && endpoints && line(&elements, Offset(8)) == Integer(1)
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
