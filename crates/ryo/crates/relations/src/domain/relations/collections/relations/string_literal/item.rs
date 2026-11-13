//! The `string_literal` relation: a string literal, decoded.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `string_literal`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct StringLiteral;

impl super::super::interface::sealed::Sealed for StringLiteral {}

/// The declaration of `string_literal`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! string_literal {
    ($callback:path) => {
        $callback! {
            item StringLiteral,
            row {
                /// One row of `string_literal`.
                StringLiteralRow
            },
            column {
                /// The columns of `string_literal`.
                StringLiteralColumn
            },
            expected {
                /// One row of `string_literal` as a fixture states it.
                StringLiteralExpectedRow
            },
            class Literal,
            scope File,
            notes Unneeded,
            columns {
                /// The literal: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What encloses it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The decoded value. Free text: the contents of a literal are the author's.
                value Value: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the literal is, quotes included.
                span Span: Span => Span;
                /// Whether the literal is an import specifier: the text an import, an export from, a dynamic import or a call that loads code
                /// writes to name the module, package or file it loads, which `static-import` and `source-dependency` read too. A call that loads a
                /// module by the name a literal gives counts. Every other literal, and every literal of a language that writes no import as a
                /// string, is lowered.
                import Import: Flag => Flag;
            }
        }
    };
}

crate::string_literal!(crate::relation_row);
