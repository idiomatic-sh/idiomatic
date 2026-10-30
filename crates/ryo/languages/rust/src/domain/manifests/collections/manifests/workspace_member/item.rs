//! The `workspace_member` relation of the language: each package a workspace holds.

use crate::domain::links::types::Linked;

use crate::domain::link_state::service::located_of;
use crate::domain::packages::service::{refusal_cause, refused_membership};
use crate::domain::packages::types::{Analysis, Located};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `workspace_member` relation's rows to.
pub(crate) struct WorkspaceMemberItem;

// A manifest Cargo refused declares nothing, so what it would have said is missing, unless it was refused as in no workspace,
// which is what this relation says of it: no row; a manifest the run does not list is numbered after the files it lists.
ryo::reads! {
    WorkspaceMemberItem => languages::WorkspaceMember, manifests(Linked) |input, symbols|
    model located: Located = (input.derived.packages.as_ref().map(|packages| located_of(&input.request.files, packages))) {
        each (&located.packages.refused) as refused {
            when refused_membership(refused) {
                Analysis::Unanalyzed => {
                    partial refusal_cause(refused).to_string() => languages::Missing::Any;
                }
                Analysis::Analyzed => {}
            }
        }
        each (&located.packages.members) as member {
            given workspace = located.listed.get(&member.workspace).copied()?;
            given package = located.listed.get(&member.package).copied()?;
            row {
                workspace: value(workspace),
                package: value(package),
            }
        }
    }
}
