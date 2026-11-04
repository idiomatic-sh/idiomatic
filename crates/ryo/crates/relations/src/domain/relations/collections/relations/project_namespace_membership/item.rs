//! Whether an imported root belongs to the host-declared project search scope.
use super::super::interface::RelationsItem;
use crate::domain::{relations::types::FileId, terms::types::Symbol};

/// Project-produced namespace membership, independent of import resolution.
#[derive(Clone, Copy, Debug)]
pub struct ProjectNamespaceMembership;
impl super::super::interface::sealed::Sealed for ProjectNamespaceMembership {}

#[macro_export]
#[doc(hidden)]
macro_rules! project_namespace_membership {
    ($callback:path) => {
        $callback! {
            item ProjectNamespaceMembership,
            row { /// One imported root's project membership.
                ProjectNamespaceMembershipRow },
            column { /// Namespace membership columns.
                ProjectNamespaceMembershipColumn },
            expected { /// One independently expected membership row.
                ProjectNamespaceMembershipExpectedRow },
            class Manifest,
            columns {
                /// The file that imports the root.
                file File: FileId => File;
                /// The first component of an absolute import.
                root Root: Symbol => Symbol;
                /// Present, absent, or unknown within the declared project scope.
                membership Membership: Symbol => Symbol;
            }
        }
    };
}
crate::project_namespace_membership!(crate::relation_row);
