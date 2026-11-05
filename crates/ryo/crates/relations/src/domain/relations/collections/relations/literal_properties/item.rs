//! Primitive literal categories and limited parser-derived properties.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `literal-properties`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct LiteralProperties;

impl super::super::interface::sealed::Sealed for LiteralProperties {}

/// The declaration of `literal-properties`.
#[macro_export]
#[doc(hidden)]
macro_rules! literal_properties {
    ($callback:path) => {
        $callback! {
            item LiteralProperties,
            row {
                /// One row of `literal-properties`.
                LiteralPropertiesRow
            },
            column {
                /// The columns of `literal-properties`.
                LiteralPropertiesColumn
            },
            expected {
                /// One row of `literal-properties` as a fixture states it.
                LiteralPropertiesExpectedRow
            },
            class Code,
            columns {
                /// The original primitive literal occurrence.
                id Id: NodeId => Node;
                /// One of integer, float, complex, boolean, string, bytes, null or ellipsis.
                kind Kind: Symbol => Symbol;
                /// True only for a parsed numeric zero.
                zero Zero: Flag => Flag;
                /// True only for an empty decoded string or bytes value.
                empty Empty: Flag => Flag;
                /// True only for the parsed `True` literal.
                boolean Boolean: Flag => Flag;
                /// The source file containing the occurrence.
                file File: FileId => File;
                /// The original source interval.
                span Span: Span => Span;
            }
        }
    };
}

crate::literal_properties!(crate::relation_row);
