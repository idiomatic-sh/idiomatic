//! Basic expression type classifications.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

#[derive(Clone, Copy, Debug)]
/// Basic parser-derived expression type classifications.
pub struct ExpressionType;
impl super::super::interface::sealed::Sealed for ExpressionType {}

#[macro_export]
#[doc(hidden)]
macro_rules! expression_type {
    ($callback:path) => {
        $callback! {
            item ExpressionType,
            row {
                /// One basic expression type classification.
                ExpressionTypeRow
            },
            column {
                /// Columns of `expression-type`.
                ExpressionTypeColumn
            },
            expected {
                /// Expected `expression-type` row.
                ExpressionTypeExpectedRow
            },
            class Code,
            columns {
                /// Original expression node.
                id Id: NodeId => Node;
                /// `atom`, `union`, `unknown` or `type-error`.
                resolution Resolution: Symbol => Symbol;
                /// One of boolean, integer, float, complex, string, bytes, null, ellipsis, dict, list, set, tuple, generator; empty for unknown or type-error.
                type_ Type: Symbol => Symbol;
                /// Containing file.
                file File: FileId => File;
                /// Original expression span.
                span Span: Span => Span;
            }
        }
    };
}
crate::expression_type!(crate::relation_row);
