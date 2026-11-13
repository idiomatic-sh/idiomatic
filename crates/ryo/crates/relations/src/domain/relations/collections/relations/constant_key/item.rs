//! Supported parser-derived constant-expression equivalence classes.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};

/// `constant-key`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct ConstantKey;

impl super::super::interface::sealed::Sealed for ConstantKey {}

/// The declaration of `constant-key`.
#[macro_export]
#[doc(hidden)]
macro_rules! constant_key {
    ($callback:path) => {
        $callback! {
            item ConstantKey,
            row {
                /// One row of `constant-key`.
                ConstantKeyRow
            },
            column {
                /// The columns of `constant-key`.
                ConstantKeyColumn
            },
            expected {
                /// One row of `constant-key` as a fixture states it.
                ConstantKeyExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// An original supported constant-expression occurrence.
                id Id: NodeId => Node;
                /// Its original canonical representative, collision-checked structurally.
                key Key: NodeId => Node;
                /// The source file containing the occurrence.
                file File: FileId => File;
                /// The original source interval.
                span Span: Span => Span;
            }
        }
    };
}

crate::constant_key!(crate::relation_row);
