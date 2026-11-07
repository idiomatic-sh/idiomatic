//! Explicit provenance for an embedded expression, independent of macros.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// A parsed expression carried by another source element.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedExpression;
impl super::super::interface::sealed::Sealed for EmbeddedExpression {}
/// Declare embedded-expression provenance and source mapping.
#[macro_export]
#[doc(hidden)]
macro_rules! embedded_expression {
    ($callback:path) => {
        $callback! {
            item EmbeddedExpression,
            row { /// One separately parsed expression.
                EmbeddedExpressionRow },
            column { /// Source carrier and semantic context.
                EmbeddedExpressionColumn },
            expected { /// Independently specified embedded provenance.
                EmbeddedExpressionExpectedRow },
            class Code,
            columns {
                /// The parsed expression's root occurrence.
                root Root: NodeId => Node;
                /// The string or other source carrier.
                host Host: NodeId => Node;
                /// Language evaluation context, such as a deferred type expression.
                context Context: Symbol => Symbol;
                /// Precise content offsets or a whole-host diagnostic mapping.
                mapping Mapping: Symbol => Symbol;
                /// The physical source file.
                file File: FileId => File;
                /// The physical diagnostic extent of the parsed root.
                span Span: Span => Span;
            }
        }
    };
}
crate::embedded_expression!(crate::relation_row);
