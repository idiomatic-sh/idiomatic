//! The `workspace_member` relation of the language: each package a `uv` workspace holds, by the patterns of its members less those of
//! its exclusions.

use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::packages::service::holdings_in;
use crate::domain::packages::types::{Wired, WorkspaceMembership};
use ryo::languages;

/// Answers the `workspace-member` rows of the repository's Python manifests and files.
pub(crate) struct WorkspaceMemberItem;

// A manifest the tool refused declares nothing, so what it would have said is missing.
ryo::reads! {
    WorkspaceMemberItem => languages::WorkspaceMember, manifests(Linked) |input, symbols|
    model wired: Wired<WorkspaceMembership> = (Some(holdings_in(&input.request.files, packages_in(input)))) {
        each (&wired.refused) as refused {
            partial refused.0 => languages::Missing::Any;
        }
        each (&wired.rows) as holding {
            row { workspace: value(holding.workspace), package: value(holding.package) }
        }
    }
}
