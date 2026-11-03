//! One parsed brace-format field, preserving order and its format-spec parent.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `brace-format-field`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct BraceFormatField;
impl super::super::interface::sealed::Sealed for BraceFormatField {}

/// The declaration of `brace-format-field`.
#[macro_export]
#[doc(hidden)]
macro_rules! brace_format_field {
    ($callback:path) => {
        $callback! {
            item BraceFormatField,
            row { /// One field occurrence in parser traversal order.
                BraceFormatFieldRow },
            column { /// The columns of `brace-format-field`.
                BraceFormatFieldColumn },
            expected { /// One expected row of `brace-format-field`.
                BraceFormatFieldExpectedRow },
            class Code,
            columns {
                /// Original string occurrence, joining to `brace-format-parse.id`.
                id Id: NodeId => Node;
                /// Zero-based field order, with each outer field preceding nested fields.
                ordinal Ordinal: Integer => Number;
                /// `literal` or `format-spec`.
                context Context: Symbol => Symbol;
                /// Parent outer-field ordinal, or -1 for a literal field.
                parent_ordinal ParentOrdinal: Integer => Number;
                /// `automatic`, `index` or `keyword`.
                selector_kind SelectorKind: Symbol => Symbol;
                /// Automatic/explicit index when it fits the engine integer; otherwise zero guarded by `index_overflow`.
                selector_index SelectorIndex: Integer => Number;
                /// True when a valid usize selector index exceeds the engine integer range.
                index_overflow IndexOverflow: Flag => Flag;
                /// Exact decimal selector text for index fields, keyword text for keyword fields, empty for automatic.
                selector_name SelectorName: Symbol => Symbol;
                /// Source file containing the string occurrence.
                file File: FileId => File;
                /// Original string occurrence interval.
                span Span: Span => Span;
            }
        }
    };
}
crate::brace_format_field!(crate::relation_row);
