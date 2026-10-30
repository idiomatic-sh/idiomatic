//! The `inherits` relation: a base a type is declared to extend or implement, by the name it gives.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `inherits`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Inherits;

impl super::super::interface::sealed::Sealed for Inherits {}

/// The declaration of `inherits`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! inherits {
    ($callback:path) => {
        $callback! {
            item Inherits,
            row {
                /// One row of `inherits`.
                InheritsRow
            },
            column {
                /// The columns of `inherits`.
                InheritsColumn
            },
            expected {
                /// One row of `inherits` as a fixture states it.
                InheritsExpectedRow
            },
            class Code,
            columns {
                /// The base or contract: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// The declaration that inherits: its file and where it starts.
                declaration Declaration: NodeId => Node;
                /// The base or contract, as written.
                target Target: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the base or contract is.
                span Span: Span => Span;
            }
        }
    };
}

crate::inherits!(crate::relation_row);
