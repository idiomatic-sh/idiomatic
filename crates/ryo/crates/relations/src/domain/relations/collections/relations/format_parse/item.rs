//! A decoded format string's grammar outcome and consumption summary.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `format-parse`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FormatParse;
impl super::super::interface::sealed::Sealed for FormatParse {}

/// The declaration of `format-parse`.
#[macro_export]
#[doc(hidden)]
macro_rules! format_parse {
    ($callback:path) => {
        $callback! {
            item FormatParse,
            row { /// One outcome for a parsed format-string occurrence.
                FormatParseRow },
            column { /// The columns of `format-parse`.
                FormatParseColumn },
            expected { /// One expected row of `format-parse`.
                FormatParseExpectedRow },
            class Code,
            columns {
                /// The original literal occurrence.
                id Id: NodeId => Node;
                /// The format dialect, initially `percent-string`.
                dialect Dialect: Symbol => Symbol;
                /// The parser input form, initially `decoded-string`.
                representation Representation: Symbol => Symbol;
                /// `parsed`, `malformed`, `unknown-input` or `unavailable`; parsed may have zero fields.
                outcome Outcome: Symbol => Symbol;
                /// Stable parser category, or `none` when the outcome has no error.
                error_category ErrorCategory: Symbol => Symbol;
                /// The offending format character, empty when not applicable.
                error_character ErrorCharacter: Symbol => Symbol;
                /// Unicode-scalar offset into the decoded input, meaningful only for malformed input.
                error_index ErrorIndex: Integer => Number;
                /// Positional values consumed by conversions and dynamic width/precision stars.
                positional_consumption PositionalConsumption: Integer => Number;
                /// Whether a parsed conversion uses a mapping key.
                mapping_present MappingPresent: Flag => Flag;
                /// Whether a parsed conversion consumes a dynamic width value.
                dynamic_width DynamicWidth: Flag => Flag;
                /// Whether a parsed conversion consumes a dynamic precision value.
                dynamic_precision DynamicPrecision: Flag => Flag;
                /// The source file containing the literal occurrence.
                file File: FileId => File;
                /// The literal occurrence's original source interval.
                span Span: Span => Span;
            }
        }
    };
}
crate::format_parse!(crate::relation_row);
