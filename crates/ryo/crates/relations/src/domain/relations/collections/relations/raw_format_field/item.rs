//! Ordered fields from complete raw-token parses; malformed segments contribute no fields.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// The optional `raw-format-field` relation.
#[derive(Clone, Copy, Debug)]
pub struct RawFormatField;
impl super::super::interface::sealed::Sealed for RawFormatField {}

/// Typed declaration of the raw format relation.
#[macro_export]
#[doc(hidden)]
macro_rules! raw_format_field {
    ($callback:path) => {
        $callback! {
            item RawFormatField,
            row { /// One source-local raw format fact.
                RawFormatFieldRow },
            column { /// Typed columns of this relation.
                RawFormatFieldColumn },
            expected { /// One independently expected raw format fact.
                RawFormatFieldExpectedRow },
            class Code,
            columns {
                /// Enclosing original visible string expression.
                id Id: NodeId => Node;
                /// Existing source string token identity.
                segment Segment: NodeId => Node;
                /// Token source order within the expression.
                segment_ordinal SegmentOrdinal: Integer => Number;
                /// Global field order among successful segments of the expression.
                ordinal Ordinal: Integer => Number;
                /// Conversion order within this segment, starting from zero.
                segment_field_ordinal SegmentFieldOrdinal: Integer => Number;
                /// Positional or mapping argument selection.
                argument_kind ArgumentKind: Symbol => Symbol;
                /// Raw mapping key spelling, never Python-escape decoded.
                mapping_key MappingKey: Symbol => Symbol;
                /// Conversion character.
                conversion Conversion: Symbol => Symbol;
                /// None, fixed or star width mode.
                width_mode WidthMode: Symbol => Symbol;
                /// None, dot, fixed or star precision mode.
                precision_mode PrecisionMode: Symbol => Symbol;
                /// Source file.
                file File: FileId => File;
                /// Owning token interval including prefixes and quotes.
                span Span: Span => Span;
                /// Unique standard percent-format flags in ASCII order (space, #, +, -, 0).
                /// Repeated flags collapse; conflicting flags remain unsimplified. Empty means none.
                flags Flags: Symbol => Symbol;
            }
        }
    };
}
crate::raw_format_field!(crate::relation_row);
