//! Import-root identities and their explicitly three-state membership.
use ryo::languages::FileId;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportedRoot {
    pub file: FileId,
    pub root: Box<str>,
    pub path: Box<str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MembershipState {
    Present,
    Absent,
    Unknown,
}

/// The observed Python entry at an imported dotted path, independent of sort policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Module,
    Package,
    Namespace,
    Absent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Membership {
    pub file: FileId,
    pub root: Box<str>,
    pub path: Box<str>,
    pub state: MembershipState,
    pub kind: EntryKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NamespaceMemberships {
    pub rows: Vec<Membership>,
    pub incomplete_notes: Vec<FileId>,
}
