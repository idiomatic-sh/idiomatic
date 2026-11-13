//! One conversion and its argument consumption in a parsed format string.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `format-field`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FormatField;
impl super::super::interface::sealed::Sealed for FormatField {}

/// The declaration of `format-field`.
#[macro_export]
#[doc(hidden)]
macro_rules! format_field {
    ($callback:path) => {
        $callback! {
            item FormatField,
            row { /// One conversion field in a parsed format string.
                FormatFieldRow },
            column { /// The columns of `format-field`.
                FormatFieldColumn },
            expected { /// One expected row of `format-field`.
                FormatFieldExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The original format-string occurrence, joining this row to `format-parse`.
                id Id: NodeId => Node;
                /// Conversion order in the decoded format string, starting from zero.
                ordinal Ordinal: Integer => Number;
                /// `positional` or `mapping`.
                argument_kind ArgumentKind: Symbol => Symbol;
                /// Decoded mapping key, or empty when argument-kind is positional.
                mapping_key MappingKey: Symbol => Symbol;
                /// The conversion character.
                conversion Conversion: Symbol => Symbol;
                /// `none`, `fixed` or `star`.
                width_mode WidthMode: Symbol => Symbol;
                /// `none`, `dot`, `fixed` or `star`.
                precision_mode PrecisionMode: Symbol => Symbol;
                /// The source file containing the literal occurrence.
                file File: FileId => File;
                /// The whole literal occurrence's original source interval.
                span Span: Span => Span;
            }
        }
    };
}
crate::format_field!(crate::relation_row);
