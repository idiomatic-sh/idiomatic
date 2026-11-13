//! Potential implicit names of a language execution environment; file identity binds the provider catalog.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;
/// Potential implicit names, their origins and version availability, without fabricated declarations.
#[derive(Clone, Copy, Debug)]
pub struct ImplicitSymbol;
impl super::super::interface::sealed::Sealed for ImplicitSymbol {}
/// Declare the reusable builtin catalog relation.
#[macro_export]
#[doc(hidden)]
macro_rules! implicit_symbol {
    ($callback:path) => {
        $callback! {
        item ImplicitSymbol,
        row { /// One provider catalog entry, independent of the file's expressions.
        ImplicitSymbolRow },
        column { /// Canonical name and numeric availability components.
        ImplicitSymbolColumn },
        expected { /// Independently specified catalog entry.
        ImplicitSymbolExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        columns {
        /// Canonical bare name potentially supplied by the language environment.
        name Name: Symbol => Symbol;
                /// Origin: builtin fallback, supplied module global, or platform-specific builtin.
                kind Kind: Symbol => Symbol;
                /// Actual module visibility root; not a fabricated declaration or assertion of initialization.
                scope Scope: NodeId => Node;
        /// First major language version supporting this implicit name, not the configured target.
        since_major SinceMajor: Integer => Number;
        /// First minor language version supporting this implicit name, compared numerically.
        since_minor SinceMinor: Integer => Number;
        /// Source file whose provider supplies this catalog; no synthetic syntax occurrence is implied.
        file File: FileId => File;
        }
        }
    };
}
crate::implicit_symbol!(crate::relation_row);
