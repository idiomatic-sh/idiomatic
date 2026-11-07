//! The `parameter` relation: a parameter of a function, method or closure.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::service::term_symbol;
use crate::domain::parameter_kind::types::ParameterKind;
use crate::domain::terms::types::Symbol;

/// `parameter`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Parameter;

impl super::super::interface::sealed::Sealed for Parameter {}

/// The declaration of `parameter`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! parameter {
    ($callback:path) => {
        $callback! {
            item Parameter,
            row {
                /// One row of `parameter`.
                ParameterRow
            },
            column {
                /// The columns of `parameter`.
                ParameterColumn
            },
            expected {
                /// One row of `parameter` as a fixture states it.
                ParameterExpectedRow
            },
            class Code,
            columns {
                /// The parameter: its file and where it starts.
                id Id: NodeId => Node;
                /// How it takes its argument.
                kind Kind: ParameterKind => Symbol(term_symbol);
                /// What it is in the language's own words.
                refinement Refinement: Symbol => Symbol;
                /// The name, as written.
                name Name: Symbol => Symbol;
                /// Its place in the declaration, from zero.
                ordinal Ordinal: Integer => Number;
                /// The callable: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the parameter is.
                span Span: Span => Span;
            }
        }
    };
}

crate::parameter!(crate::relation_row);
