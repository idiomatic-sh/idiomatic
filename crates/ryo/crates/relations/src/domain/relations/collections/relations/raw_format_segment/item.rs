//! One outcome per raw string token, including empty and malformed bodies.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// The optional `raw-format-segment` relation.
#[derive(Clone, Copy, Debug)]
pub struct RawFormatSegment;
impl super::super::interface::sealed::Sealed for RawFormatSegment {}

/// Typed declaration of the raw format relation.
#[macro_export]
#[doc(hidden)]
macro_rules! raw_format_segment {
    ($callback:path) => {
        $callback! {
            item RawFormatSegment,
            row { /// One source-local raw format fact.
                RawFormatSegmentRow },
            column { /// Typed columns of this relation.
                RawFormatSegmentColumn },
            expected { /// One independently expected raw format fact.
                RawFormatSegmentExpectedRow },
            class Code,
            columns {
                /// Enclosing original visible string expression.
                id Id: NodeId => Node;
                /// Existing string token identity.
                segment Segment: NodeId => Node;
                /// Zero-based token source order within the expression.
                segment_ordinal SegmentOrdinal: Integer => Number;
                /// The percent-string format dialect.
                dialect Dialect: Symbol => Symbol;
                /// Parsed, malformed or unavailable input; empty valid input is parsed.
                outcome Outcome: Symbol => Symbol;
                /// Stable parser error category, or none.
                error_category ErrorCategory: Symbol => Symbol;
                /// Offending character, empty when not applicable.
                error_character ErrorCharacter: Symbol => Symbol;
                /// Unicode scalar offset within the unescaped raw token body.
                error_index ErrorIndex: Integer => Number;
                /// Conversion and dynamic star positional consumption.
                positional_consumption PositionalConsumption: Integer => Number;
                /// Whether any parsed conversion names a mapping key.
                mapping_present MappingPresent: Flag => Flag;
                /// Whether a parsed width consumes an argument.
                dynamic_width DynamicWidth: Flag => Flag;
                /// Whether a parsed precision consumes an argument.
                dynamic_precision DynamicPrecision: Flag => Flag;
                /// Source file.
                file File: FileId => File;
                /// Whole source token including prefixes and quotes.
                span Span: Span => Span;
            }
        }
    };
}
crate::raw_format_segment!(crate::relation_row);
