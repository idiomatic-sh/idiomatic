//! A canonical identifier occurrence retains its original source extent.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// A lexical identifier, normalized according to its source language.
#[derive(Clone, Copy, Debug)]
pub struct Identifier;
impl super::super::interface::sealed::Sealed for Identifier {}
/// Declare lexical identity separately from source tokens and resolved names.
#[macro_export]
#[doc(hidden)]
macro_rules! identifier {
    ($callback:path) => {
        $callback! {
            item Identifier,
            row { /// One lexical identifier occurrence.
                IdentifierRow },
            column { /// Canonical spelling and original source occurrence.
                IdentifierColumn },
            expected { /// Independently stated identifier occurrence.
                IdentifierExpectedRow },
            class Code,
            columns {
                /// The syntax occurrence carrying the identifier, not its raw token.
                id Id: NodeId => Node;
                /// Canonical lexical spelling. Dotted import identifiers retain their full path; no binding resolution is implied.
                name Name: Symbol => Symbol;
                /// The original source file.
                file File: FileId => File;
                /// The original identifier extent, before normalization.
                span Span: Span => Span;
            }
        }
    };
}
crate::identifier!(crate::relation_row);
