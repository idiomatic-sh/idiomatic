//! Project membership answers from an explicitly declared search scope and observed candidates.
use crate::domain::links::types::Linked;
use crate::domain::project_namespace::{
    constants,
    types::{EntryKind, MembershipState, NamespaceMemberships},
};
use ryo::languages;

/// Reads the membership of imported roots, including explicit uncertainty.
pub(crate) struct ProjectNamespaceMembershipItem;
ryo::reads! {
    ProjectNamespaceMembershipItem => languages::ProjectNamespaceMembership, manifests(Linked) |input, symbols|
    model memberships: NamespaceMemberships = (Some(input.derived.namespace_memberships.clone())) {
        each (&memberships.incomplete_notes) as file {
            partial_at *file, constants::UNKNOWN_NOTES => languages::Missing::Any;
        }
        each (&memberships.rows) as membership {
            each ((membership.state == MembershipState::Unknown).then_some(membership.file)) as file {
                partial_at file, constants::UNKNOWN_CONTEXT => languages::Missing::Any;
            }
            row {
                file: value(membership.file),
                root: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&membership.root))),
                path: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&membership.path))),
                membership: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match membership.state {
                    MembershipState::Present => constants::PRESENT,
                    MembershipState::Absent => constants::ABSENT,
                    MembershipState::Unknown => constants::UNKNOWN,
                }))),
                kind: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match membership.kind {
                    EntryKind::Module => "module",
                    EntryKind::Package => "package",
                    EntryKind::Namespace => "namespace",
                    EntryKind::Absent => "absent",
                    EntryKind::Unknown => "unknown",
                }))),
            }
        }
    }
}
