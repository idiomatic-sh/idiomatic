//! The `workspace_member` relation of the language: each module a `go.work` uses, by the directories it lists.

use crate::domain::links::service::modules_in;
use crate::domain::links::types::Linked;
use crate::domain::modules::service::holdings_in;
use crate::domain::modules::types::{Wired, WorkspaceMembership};
use ryo::languages;

/// The reader the `project` contract hands the `workspace_member` relation to, which reads its rows from the repository's manifests.
pub(crate) struct WorkspaceMemberItem;

// A manifest the tool refused declares nothing, so what it would have said is missing.
ryo::reads! {
    WorkspaceMemberItem => languages::WorkspaceMember, manifests(Linked) |input, symbols|
    model wired: Wired<WorkspaceMembership> = (Some(holdings_in(&input.request.files, modules_in(input)))) {
        each (&wired.refused) as refused {
            partial refused.0 => languages::Missing::Any;
        }
        each (&wired.rows) as holding {
            row { workspace: value(holding.workspace), package: value(holding.module) }
        }
    }
}
