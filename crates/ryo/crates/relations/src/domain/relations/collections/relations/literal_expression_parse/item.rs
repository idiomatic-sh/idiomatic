//! Typed source-carrier expression parse outcomes.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// The optional literal-expression-parse relation.
#[derive(Clone, Copy, Debug)]
pub struct LiteralExpressionParse;
impl super::super::interface::sealed::Sealed for LiteralExpressionParse {}

/// Declare the source carrier, dialect and syntactic outcome columns.
#[macro_export]
#[doc(hidden)]
macro_rules! literal_expression_parse {
    ($callback:path) => {
        $callback! {
            item LiteralExpressionParse,
            row { /// One source-local expression parse outcome.
                LiteralExpressionParseRow },
            column { /// Columns of literal-expression-parse.
                LiteralExpressionParseColumn },
            expected { /// One independently expected expression parse outcome.
                LiteralExpressionParseExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// Existing original string-expression identity.
                id Id: NodeId => Node;
                /// Syntactic grammar.
                dialect Dialect: Symbol => Symbol;
                /// Parsed, malformed, or unavailable; independent of type position.
                outcome Outcome: Symbol => Symbol;
                /// Containing source file.
                file File: FileId => File;
                /// Full physical source-carrier range, including prefixes and quotes.
                span Span: Span => Span;
            }
        }
    };
}
crate::literal_expression_parse!(crate::relation_row);
