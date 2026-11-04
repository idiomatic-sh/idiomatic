//! External vocabulary of project namespace membership.
use super::item::ProjectNamespaceMembershipColumn;

#[must_use]
pub fn name() -> &'static str {
    "project-namespace-membership"
}

#[must_use]
pub fn summary() -> &'static str {
    "Whether an imported root is present under the host-declared ordered project search roots; an unknown row also makes the relation incomplete."
}

#[must_use]
pub fn column_name(column: ProjectNamespaceMembershipColumn) -> &'static str {
    match column {
        ProjectNamespaceMembershipColumn::File => "file",
        ProjectNamespaceMembershipColumn::Root => "root",
        ProjectNamespaceMembershipColumn::Membership => "membership",
    }
}
