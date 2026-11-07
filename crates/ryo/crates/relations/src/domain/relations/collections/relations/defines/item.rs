//! The `defines` relation: a name a file declares or brings in, under its shared kind, with the language's own refinement.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::service::term_symbol;
use crate::domain::declaration_kind::types::DeclarationKind;
use crate::domain::terms::types::Symbol;

/// `defines`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Defines;

impl super::super::interface::sealed::Sealed for Defines {}

/// The declaration of `defines`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! defines {
    ($callback:path) => {
        $callback! {
            item Defines,
            row {
                /// One row of `defines`.
                DefinesRow
            },
            column {
                /// The columns of `defines`.
                DefinesColumn
            },
            expected {
                /// One row of `defines` as a fixture states it.
                DefinesExpectedRow
            },
            class Code,
            columns {
                /// The declaration: its file and where it starts.
                id Id: NodeId => Node;
                /// What it declares, from the shared closed set.
                kind Kind: DeclarationKind => Symbol(term_symbol);
                /// What it is in the language's own words.
                refinement Refinement: Symbol => Symbol;
                /// The name, as written.
                name Name: Symbol => Symbol;
                /// What encloses it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the name is.
                span Span: Span => Span;
            }
        }
    };
}

crate::defines!(crate::relation_row);
