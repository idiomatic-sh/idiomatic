//! Proven lexical origins of expressions, independent of lint policy.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// The canonical imported or builtin name of an expression.
#[derive(Clone, Copy, Debug)]
pub struct QualifiedName;
impl super::super::interface::sealed::Sealed for QualifiedName {}
/// Declare canonical provenance and numeric language-version components.
#[macro_export]
#[doc(hidden)]
macro_rules! qualified_name {
    ($callback:path) => {
        $callback! {
            item QualifiedName,
            row { /// One expression's lexical origin.
                QualifiedNameRow },
            column { /// Columns describing lexical provenance.
                QualifiedNameColumn },
            expected { /// Independently stated lexical provenance.
                QualifiedNameExpectedRow },
            class Code,
            columns {
                /// The load expression, including nested attribute prefixes.
                id Id: NodeId => Node;
                /// Canonical imported/builtin name; unknown for dynamic or ambiguous identities.
                path Path: Symbol => Symbol;
                /// Origin category: import, builtin, local, unresolved, or ambiguous.
                provenance Provenance: Symbol => Symbol;
                /// First major language version supplying the builtin root binding. Zero for non-builtin provenance.
                since_major SinceMajor: Integer => Number;
                /// First minor language version supplying the builtin root binding. Zero for non-builtin provenance.
                since_minor SinceMinor: Integer => Number;
                /// The expression's file.
                file File: FileId => File;
                /// The expression's source interval.
                span Span: Span => Span;
            }
        }
    };
}
crate::qualified_name!(crate::relation_row);
