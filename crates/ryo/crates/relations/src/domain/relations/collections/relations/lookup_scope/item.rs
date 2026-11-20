//! For each read or deletion of a name, the namespaces in which Python looks the name up, nearest first.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// For each read or deletion of a name, the namespaces in which Python looks the name up, nearest first.
#[derive(Clone, Copy, Debug)]
pub struct LookupScope;
impl super::super::interface::sealed::Sealed for LookupScope {}
/// Declare the reusable static lookup-path fact.
#[macro_export]
#[doc(hidden)]
macro_rules! lookup_scope {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
        item LookupScope,
        row { /// One namespace on one reference's lookup path.
        LookupScopeRow },
        column { /// Public fact columns.
        LookupScopeColumn },
        expected { /// Independently specified lookup path.
        LookupScopeExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        form Filled(Many(Reference)),
        columns {
        /// Read or deletion occurrence (a binding-event of kind read or delete).
        reference Reference: NodeId => Node;
        /// The name read or deleted.
        name Name: Symbol => Symbol;
        /// A namespace on the path: binding-event.scope of the events it holds; a type parameter's defines id for its scope.
        scope Scope: NodeId => Node;
        /// 0 for the first namespace on the path; namespaces sharing a step share a depth.
        depth Depth: Integer => Number;
        /// Value, type or deletion context of the reference.
        context Context: Symbol => Symbol;
        /// Source file defining these identities.
        file File: FileId => File;
        /// Reference extent.
        span Span: Span => Span;
        }
        }
    };
}
crate::lookup_scope!(crate::relation_row);
