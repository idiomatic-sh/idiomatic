//! Parsed outcome and neutral selector counts for one decoded brace-format string.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `brace-format-parse`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct BraceFormatParse;
impl super::super::interface::sealed::Sealed for BraceFormatParse {}

/// The declaration of `brace-format-parse`.
#[macro_export]
#[doc(hidden)]
macro_rules! brace_format_parse {
    ($callback:path) => {
        $callback! {
            item BraceFormatParse,
            row { /// One outcome for a decoded brace-format occurrence.
                BraceFormatParseRow },
            column { /// The columns of `brace-format-parse`.
                BraceFormatParseColumn },
            expected { /// One expected row of `brace-format-parse`.
                BraceFormatParseExpectedRow },
            class Code,
            columns {
                /// Original decoded string occurrence.
                id Id: NodeId => Node;
                /// Currently `decoded-string`.
                representation Representation: Symbol => Symbol;
                /// `parsed`, `malformed`, `unknown-input` or `unavailable`.
                outcome Outcome: Symbol => Symbol;
                /// Stable generic parser category, or `none` for non-malformed outcomes.
                error_category ErrorCategory: Symbol => Symbol;
                /// Number of automatic positional fields, including nested format specs.
                automatic_count AutomaticCount: Integer => Number;
                /// Number of explicit positional fields, including nested format specs.
                indexed_count IndexedCount: Integer => Number;
                /// Number of keyword fields, including nested format specs.
                keyword_count KeywordCount: Integer => Number;
                /// Number of fields nested inside format specifications.
                nested_field_count NestedFieldCount: Integer => Number;
                /// Source file containing the string occurrence.
                file File: FileId => File;
                /// Original occurrence interval.
                span Span: Span => Span;
            }
        }
    };
}
crate::brace_format_parse!(crate::relation_row);
