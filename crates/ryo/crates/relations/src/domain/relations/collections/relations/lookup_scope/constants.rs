//! Query vocabulary for lookup-scope.
use super::item::LookupScopeColumn;
use crate::domain::relations::constants::{CONTEXT, REFERENCE};
/// Query relation name.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::labels::types::NameRef;
/// use ryo_relations::domain::relations::collections::relations::lookup_scope::constants::name;
/// use ryo_relations::domain::relations::service::relation_named;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(relation_named(NameRef(name())), Some(Relation::LookupScope));
/// ```
#[must_use]
pub fn name() -> &'static str {
    "lookup-scope"
}
/// The static semantic contract, independent of lint policy.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::lookup_scope::constants::summary;
///
/// assert!(summary().ends_with('.'));
/// ```
#[must_use]
pub fn summary() -> &'static str {
    "For each read or deletion of a name, the namespaces in which the name is looked up, nearest first: the block of the reference, then each enclosing function, lambda or comprehension block (class blocks are not visible from nested blocks), then the module; after a block that declares the name global, the module directly; a type-parameter scope of the name between its owner and the owner's block. Which binding the name has in a namespace is not stated here."
}
/// Query column name.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::lookup_scope::constants::column_name;
/// use ryo_relations::domain::relations::collections::relations::lookup_scope::item::LookupScopeColumn;
///
/// assert_eq!(column_name(LookupScopeColumn::Depth), "depth");
/// ```
#[must_use]
pub fn column_name(column: LookupScopeColumn) -> &'static str {
    match column {
        LookupScopeColumn::Reference => REFERENCE,
        LookupScopeColumn::Name => "name",
        LookupScopeColumn::Scope => "scope",
        LookupScopeColumn::Depth => "depth",
        LookupScopeColumn::Context => CONTEXT,
        LookupScopeColumn::File => "file",
        LookupScopeColumn::Span => "span",
    }
}
