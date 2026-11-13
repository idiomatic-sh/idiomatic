//! Observed project entry kinds for the dotted prefixes of an absolute import.
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
            row { /// One observed dotted prefix of an imported project path.
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
                /// The dotted import path prefix observed beneath the declared search roots.
                path Path: Symbol => Symbol;
                /// Present, absent, or unknown within the declared project scope.
                membership Membership: Symbol => Symbol;
                /// Module, package, namespace, absent, or unknown observed at that prefix.
                kind Kind: Symbol => Symbol;
            }
        }
    };
}
crate::project_namespace_membership!(crate::relation_row);
