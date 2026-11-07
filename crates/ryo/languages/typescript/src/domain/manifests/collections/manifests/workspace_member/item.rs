//! The repository packages selected by each workspace's globs.

use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::packages::service::members;
use crate::domain::packages::service::projected;
use crate::domain::packages::types::Projection;
use ryo::languages;

/// Answers the `workspace-member` rows of the repository's TypeScript manifests and files.
pub(crate) struct WorkspaceMemberItem;

ryo::reads! {
    WorkspaceMemberItem => languages::WorkspaceMember, manifests(Linked) |input, symbols|
    model projection: Projection<languages::WorkspaceMemberRow> = (Some(projected(packages_in(input), members(&input.request.files, packages_in(input))))) {
        each (&projection.refused) as refused {
            partial refused.reason.0 => languages::Missing::Any;
        }
        each (&projection.rows) as membership {
            row { workspace: value(membership.workspace), package: value(membership.package) }
        }
    }
}
