//! External vocabulary of project namespace membership.
use super::item::ProjectNamespaceMembershipColumn;

#[must_use]
/// The relation's name.
pub fn name() -> &'static str {
    "project-namespace-membership"
}

#[must_use]
/// What the relation holds.
pub fn summary() -> &'static str {
    "Observed module, package, and namespace entry kinds for each imported dotted prefix under host-declared ordered project search roots; unknown rows make the relation incomplete."
}

#[must_use]
/// The name of one column.
pub fn column_name(column: ProjectNamespaceMembershipColumn) -> &'static str {
    match column {
        ProjectNamespaceMembershipColumn::File => "file",
        ProjectNamespaceMembershipColumn::Root => "root",
        ProjectNamespaceMembershipColumn::Path => "path",
        ProjectNamespaceMembershipColumn::Membership => "membership",
        ProjectNamespaceMembershipColumn::Kind => "kind",
    }
}
