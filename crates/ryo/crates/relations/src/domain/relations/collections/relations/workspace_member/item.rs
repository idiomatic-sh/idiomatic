//! The `workspace_member` relation: one row for each package a workspace holds.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::FileId;

/// `workspace_member`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct WorkspaceMember;

impl super::super::interface::sealed::Sealed for WorkspaceMember {}

/// The declaration of `workspace_member`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! workspace_member {
    ($callback:path) => {
        $callback! {
            item WorkspaceMember,
            row {
                /// One row of `workspace_member`.
                WorkspaceMemberRow
            },
            column {
                /// The columns of `workspace_member`.
                WorkspaceMemberColumn
            },
            expected {
                /// One row of `workspace_member` as a fixture states it.
                WorkspaceMemberExpectedRow
            },
            class Manifest,
            columns {
                /// The workspace: its manifest.
                workspace Workspace: FileId => File;
                /// The member package: its manifest.
                package Package: FileId => File;
            }
        }
    };
}

crate::workspace_member!(crate::relation_row);
