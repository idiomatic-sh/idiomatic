//! Five fixed words preserve IDs, symbols and the real source span.

use super::super::interface::RelationsItem as _;
use super::item::{NumericLiteral, NumericLiteralRow};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span, Value};
use crate::domain::terms::types::Symbol;

#[test]
fn numeric_payload_is_a_symbol_without_widening_row_values() {
    assert_eq!(
        NumericLiteral::words(NumericLiteralRow {
            id: NodeId(3),
            kind: Symbol(17),
            value: Symbol(23),
            file: FileId(7),
            span: Span {
                start: Offset(11),
                end: Offset(13)
            },
        }),
        [Bits(3), Bits(17), Bits(23), Bits(7), Bits(47_244_640_269)]
    );
    assert_eq!(std::mem::size_of::<Value>(), 16);
}
