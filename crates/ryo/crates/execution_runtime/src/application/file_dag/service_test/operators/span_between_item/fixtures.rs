//! The support of the `span_between_item` cases.
use ryo_relations::domain::relations::types::{Offset, Span};

/// The span from byte `start` to byte `end`.
pub(super) const fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}
