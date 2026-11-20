//! For each read or deletion and each namespace on its lookup path, the binding in effect there and each later event of the name.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// For each read or deletion and each namespace on its lookup path, the binding in effect there and each later event of the name.
#[derive(Clone, Copy, Debug)]
pub struct BindingLookup;
impl super::super::interface::sealed::Sealed for BindingLookup {}
/// Declare the reusable static resolution fact.
#[macro_export]
#[doc(hidden)]
macro_rules! binding_lookup {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
        item BindingLookup,
        row { /// One event a lookup of the name finds in one namespace.
        BindingLookupRow },
        column { /// Public fact columns.
        BindingLookupColumn },
        expected { /// Independently specified resolution.
        BindingLookupExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        form Filled(Many(Reference)),
        columns {
        /// Read or deletion occurrence (a binding-event of kind read or delete).
        reference Reference: NodeId => Node;
        /// The name read or deleted.
        name Name: Symbol => Symbol;
        /// Occurrence of the event: a defines id for a bind, the name of a global or nonlocal statement, the handler
        /// for a cleanup, the defines id for a type parameter.
        event Event: NodeId => Node;
        /// bind, annotation, delete, cleanup, global or nonlocal; never wildcard.
        kind Kind: Symbol => Symbol;
        /// Namespace of the row: the namespace on the lookup path that holds the event.
        scope Scope: NodeId => Node;
        /// Namespace whose code holds the event before global/nonlocal routing.
        lexical Lexical: NodeId => Node;
        /// 0 for the first namespace on the reference's lookup path; namespaces sharing a step share a depth.
        depth Depth: Integer => Number;
        /// binding-event ordinal of the event; 0 for a type parameter. Join binding-event on (occurrence = event, ordinal).
        ordinal Ordinal: Integer => Number;
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
crate::binding_lookup!(crate::relation_row);
