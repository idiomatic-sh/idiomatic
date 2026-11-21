//! For each read or deletion of a name, the namespaces in which the name is looked up, nearest first.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// For each read or deletion of a name, the namespaces in which the name is looked up, nearest first: an optional relation,
/// which a language promises or says it does not read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::OptionalCapability;
/// use ryo_relations::domain::relations::collections::relations::lookup_scope::item::LookupScope;
/// use ryo_relations::domain::relations::service::optional_capability;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let _item = LookupScope;
/// assert_eq!(optional_capability(Relation::LookupScope), Some(OptionalCapability::LookupScope));
/// ```
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
        size PerElement,
        keys [Reference],
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
