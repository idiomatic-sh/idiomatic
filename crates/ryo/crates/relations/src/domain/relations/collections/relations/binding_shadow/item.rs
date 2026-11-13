//! Concrete replacement and lexical-shadow associations between distinct declaration occurrences.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId};
use crate::domain::terms::types::Symbol;
/// Concrete replacement and lexical-shadow associations between distinct declaration occurrences.
#[derive(Clone, Copy, Debug)]
pub struct BindingShadow;
impl super::super::interface::sealed::Sealed for BindingShadow {}
/// Declare the reusable static binding fact.
#[macro_export]
#[doc(hidden)]
macro_rules! binding_shadow {
    ($callback:path) => {
        $callback! {
        item BindingShadow,
        row { /// One explicit binding-analysis observation.
        BindingShadowRow },
        column { /// Public fact columns.
        BindingShadowColumn },
        expected { /// Independently specified observation.
        BindingShadowExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        columns {
        /// Current actual defines.id.
        binding Binding: NodeId => Node;
        /// Previous actual defines.id; no fake absent declaration.
        previous Previous: NodeId => Node;
        /// Namespace owning the current declaration.
        scope Scope: NodeId => Node;
        /// Namespace owning the previous declaration.
        previous_scope PreviousScope: NodeId => Node;
        /// Execution boundary in which this association is established.
        phase Phase: NodeId => Node;
        /// Same-namespace replacement or lexical-ancestor shadowing.
        relationship Relationship: Symbol => Symbol;
        /// Source file defining these identities.
        file File: FileId => File;
        }
        }
    };
}
crate::binding_shadow!(crate::relation_row);
