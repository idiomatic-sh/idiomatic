//! Parser-decoded literal text fragments inside interpolated strings.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `interpolated-string-literal`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct InterpolatedStringLiteral;

impl super::super::interface::sealed::Sealed for InterpolatedStringLiteral {}

/// The declaration of `interpolated-string-literal`.
#[macro_export]
#[doc(hidden)]
macro_rules! interpolated_string_literal {
    ($callback:path) => {
        $callback! {
            item InterpolatedStringLiteral,
            row { /// One decoded literal fragment and its owning interpolated string.
                InterpolatedStringLiteralRow },
            column { /// The columns of `interpolated-string-literal`.
                InterpolatedStringLiteralColumn },
            expected { /// One row of `interpolated-string-literal` as a fixture states it.
                InterpolatedStringLiteralExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The literal element occurrence.
                id Id: NodeId => Node;
                /// The nearest interpolated string that owns the fragment.
                owner Owner: NodeId => Node;
                /// The parser-decoded fragment text.
                value Value: Symbol => Symbol;
                /// The source file containing the fragment.
                file File: FileId => File;
                /// The original source interval of the fragment.
                span Span: Span => Span;
            }
        }
    };
}

crate::interpolated_string_literal!(crate::relation_row);
