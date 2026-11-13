//! Five fixed words preserve IDs, symbols and the real source span.

use super::super::interface::RelationsItem as _;
use super::item::{NumericLiteral, NumericLiteralRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span, Value as CellValue};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = ([Bits; 5], usize);
type Failure = ();

/// The size in bytes of one stored cell value.
const CELL_VALUE_BYTES: usize = 16;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("numeric_payload_is_a_symbol_without_widening_row_values"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((
        [Bits(3), Bits(17), Bits(23), Bits(7), Bits(47_244_640_269)],
        CELL_VALUE_BYTES,
    )),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok((
        NumericLiteral::words(NumericLiteralRow {
            id: NodeId(3),
            kind: Symbol(17),
            value: Symbol(23),
            file: FileId(7),
            span: Span {
                start: Offset(11),
                end: Offset(13),
            },
        }),
        std::mem::size_of::<CellValue>(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
