//! The parse outcome of one regular-expression pattern carried by a source element.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `pattern-parse`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct PatternParse;
impl super::super::interface::sealed::Sealed for PatternParse {}

/// The declaration of `pattern-parse`.
#[macro_export]
#[doc(hidden)]
macro_rules! pattern_parse {
    ($callback:path) => {
        $callback! {
            item PatternParse,
            row { /// One outcome for a pattern carrier.
                PatternParseRow },
            column { /// The columns of `pattern-parse`.
                PatternParseColumn },
            expected { /// One expected row of `pattern-parse`.
                PatternParseExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The source element that carries the pattern, such as a pattern literal or the string a pattern constructor takes; or,
                /// when a call gives flags but no pattern text, the flags it gives.
                carrier Carrier: NodeId => Node;
                /// The flags the pattern was parsed with, as written, or `unknown` when the source does not state them.
                flags Flags: Symbol => Symbol;
                /// `parsed`, `malformed` (pattern refused) or `malformed-flags` (flags refused, pattern unread).
                outcome Outcome: Symbol => Symbol;
                /// The parser's message for what it refused; nothing for a parsed pattern.
                error Error: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the parser refused it, when it did; else the carrier's extent.
                span Span: Span => Span;
            }
        }
    };
}
crate::pattern_parse!(crate::relation_row);
