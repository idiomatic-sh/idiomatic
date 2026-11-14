//! One element of a parsed regular-expression pattern.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `pattern-syntax`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct PatternSyntax;
impl super::super::interface::sealed::Sealed for PatternSyntax {}

/// The declaration of `pattern-syntax`.
#[macro_export]
#[doc(hidden)]
macro_rules! pattern_syntax {
    ($callback:path) => {
        $callback! {
            item PatternSyntax,
            row { /// One element of a parsed pattern.
                PatternSyntaxRow },
            column { /// The columns of `pattern-syntax`.
                PatternSyntaxColumn },
            expected { /// One expected row of `pattern-syntax`.
                PatternSyntaxExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The pattern element.
                id Id: NodeId => Node;
                /// The source element that carries the pattern.
                carrier Carrier: NodeId => Node;
                /// What it is, in the pattern grammar's own words.
                kind Kind: Symbol => Symbol;
                /// The element's pattern text, as the pattern spells it.
                text Text: Symbol => Symbol;
                /// The code point of a character, the index of an indexed reference, or 0 when neither applies.
                value Value: Integer => Number;
                /// The file it is in.
                file File: FileId => File;
                /// Where it is in the file, or the carrier's extent when the pattern text is not the file's own bytes.
                span Span: Span => Span;
            }
        }
    };
}
crate::pattern_syntax!(crate::relation_row);
