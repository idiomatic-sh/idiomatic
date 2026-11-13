//! One edge of a parsed regular-expression pattern.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;

/// `pattern-child`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct PatternChild;
impl super::super::interface::sealed::Sealed for PatternChild {}

/// The declaration of `pattern-child`.
#[macro_export]
#[doc(hidden)]
macro_rules! pattern_child {
    ($callback:path) => {
        $callback! {
            item PatternChild,
            row { /// One edge of a parsed pattern.
                PatternChildRow },
            column { /// The columns of `pattern-child`.
                PatternChildColumn },
            expected { /// One expected row of `pattern-child`.
                PatternChildExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The parent pattern element.
                parent Parent: NodeId => Node;
                /// The child pattern element.
                child Child: NodeId => Node;
                /// The child's place under the parent, from zero.
                ordinal Ordinal: Integer => Number;
                /// The child's role under the parent, in the pattern grammar's own words.
                field Field: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}
crate::pattern_child!(crate::relation_row);
