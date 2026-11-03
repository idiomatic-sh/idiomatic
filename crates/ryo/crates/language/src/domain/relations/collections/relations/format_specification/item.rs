//! Standard formatting syntax observations, independent of a value's custom __format__.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// Optional format-specification rows.
#[derive(Clone, Copy, Debug)]
pub struct FormatSpecification;
impl super::super::interface::sealed::Sealed for FormatSpecification {}
/// The format-specification declaration.
#[macro_export]
#[doc(hidden)]
macro_rules! format_specification {
    ($callback:path) => {
        $callback! {
            item FormatSpecification,
            row { /// One field's format specification.
                FormatSpecificationRow },
            column { /// The public format-specification columns.
                FormatSpecificationColumn },
            expected { /// One independently expected format-specification observation.
                FormatSpecificationExpectedRow },
            class Code,
            columns {
                    /// Original enclosing string occurrence.
                    id Id: NodeId => Node;
                    /// Zero-based field order within the string; outer fields precede nested fields.
                    ordinal Ordinal: Integer => Number;
                    /// Enclosing field ordinal, or -1 for a top-level field.
                    parent_ordinal ParentOrdinal: Integer => Number;
                    /// Source format-spec node where available; otherwise the enclosing string.
                    origin Origin: NodeId => Node;
                    /// brace-string or fstring; no method-call policy is implied.
                    source_kind SourceKind: Symbol => Symbol;
                    /// Decoded specification; dynamic f-string specifications have an unknown value.
                    value Value: Symbol => Symbol;
                    /// recognized, dynamic, invalid-type or other-error for the standard mini-language.
                    outcome Outcome: Symbol => Symbol;
                    /// Unsupported standard presentation character, empty otherwise.
                    invalid_type InvalidType: Symbol => Symbol;
                    /// Complete brace-field selector path, empty for automatic fields or f-string expressions.
                selector Selector: Symbol => Symbol;
                /// Explicit conversion character, or empty when formatting the original value.
                conversion Conversion: Symbol => Symbol;
                /// Source file.
                    file File: FileId => File;
                    /// Origin extent; brace-string fields use the full literal extent, not fabricated subspans.
                    span Span: Span => Span;
            }
        }
    };
}
crate::format_specification!(crate::relation_row);
