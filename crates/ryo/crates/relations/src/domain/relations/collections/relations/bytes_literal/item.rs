//! Decoded byte-string literals, independent of their source spelling.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// Optional decoded byte-string literals.
#[derive(Clone, Copy, Debug)]
pub struct BytesLiteral;
impl super::super::interface::sealed::Sealed for BytesLiteral {}
/// The declaration of `bytes-literal`.
#[macro_export]
#[doc(hidden)]
macro_rules! bytes_literal {
    ($callback:path) => {
        $callback! {
            item BytesLiteral,
            row { /// One decoded byte-string literal.
                BytesLiteralRow },
            column { /// The columns of `bytes-literal`.
                BytesLiteralColumn },
            expected { /// Independently expected decoded bytes.
                BytesLiteralExpectedRow },
            class Literal,
            columns {
                /// The literal occurrence, including adjacent joined pieces.
                occurrence Occurrence: NodeId => Node;
                /// The enclosing scope, as for `string-literal`.
                owner Owner: NodeId => Node;
                /// Canonical lowercase hexadecimal: exactly two digits per decoded octet.
                /// Empty bytes have an empty value; this is not source spelling or Unicode text.
                hex_value HexValue: Symbol => Symbol;
                /// The containing source file.
                file File: FileId => File;
                /// The full literal source interval, including quotes and adjacent pieces.
                span Span: Span => Span;
            }
        }
    };
}
crate::bytes_literal!(crate::relation_row);
