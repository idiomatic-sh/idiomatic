//! For each read or deletion and each namespace on its lookup path, the event in effect there in schedule order.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// For each read or deletion and each namespace on its lookup path, the event in effect there in schedule order.
#[derive(Clone, Copy, Debug)]
pub struct BindingResolution;
impl super::super::interface::sealed::Sealed for BindingResolution {}
/// Declare the reusable static resolution fact.
#[macro_export]
#[doc(hidden)]
macro_rules! binding_resolution {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
        item BindingResolution,
        row { /// One namespace's answer for one read or deletion.
        BindingResolutionRow },
        column { /// Public fact columns.
        BindingResolutionColumn },
        expected { /// Independently specified resolution.
        BindingResolutionExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        form Filled(Many(Reference)),
        columns {
        /// Read or deletion occurrence (a binding-event of kind read or delete).
        reference Reference: NodeId => Node;
        /// Occurrence of the event in effect: a defines id for a bind, the name of a global or nonlocal statement, the handler
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
crate::binding_resolution!(crate::relation_row);
