//! The twelve named columns preserve their declared order.
use super::super::interface::RelationsItem as _;
use super::item::{FormatSpecification, FormatSpecificationRow};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
#[test]
fn row_codec_preserves_field_ancestry_and_origin() {
    let words = FormatSpecification::words(FormatSpecificationRow {
        id: NodeId(1),
        ordinal: Integer(2),
        parent_ordinal: Integer(-1),
        origin: NodeId(3),
        source_kind: Symbol(4),
        value: Symbol(5),
        outcome: Symbol(6),
        invalid_type: Symbol(7),
        selector: Symbol(8),
        conversion: Symbol(9),
        file: FileId(10),
        span: Span {
            start: Offset(9),
            end: Offset(10),
        },
    });
    assert_eq!(
        words,
        [
            Bits(1),
            Bits(2),
            Bits(u64::MAX),
            Bits(3),
            Bits(4),
            Bits(5),
            Bits(6),
            Bits(7),
            Bits(8),
            Bits(9),
            Bits(10),
            Bits(38_654_705_674)
        ]
    );
}
