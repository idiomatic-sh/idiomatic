//! The types of the `check_store` port: plain data, public fields and derives, with no constructor and no inherent impl.
//! What a check keeps for the next: its answer and the files it rests on.

/// What a check keeps for the next: the digest of every file its answer rests on, and the answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// Every file the answer rests on, a repository's by its path in the repository, a package's by its path on the
    /// machine.
    pub files: Vec<CheckedFile>,
    /// The findings.
    pub findings: Vec<crate::domain::findings::types::Finding>,
    /// The findings in-source ignores accepted, with their reasons.
    pub suppressed: Vec<crate::domain::findings::types::Suppression>,
    /// Every rule and check that ran.
    pub evaluated: std::collections::BTreeSet<crate::domain::findings::types::FindingRule>,
    /// The rules the ignores were read against, as one digest: the files in `sourced` hold only while it is the same.
    pub known: crate::domain::digests::types::Digest,
    /// What each file settled against its text had before its ignores were applied: while the file and that are as they were, the
    /// next check does not read it, and its findings are those kept.
    pub sourced: Vec<SourcedFile>,
}

/// A file a check settled against its text, and what it had before its ignores were applied, as one fingerprint.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcedFile {
    /// Its path in the repository.
    pub path: CheckedPath,
    /// What it had before its ignores were applied.
    pub incoming: crate::domain::findings::types::Fingerprint,
}

/// A file a check's answer rests on, and the digest of its text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckedFile {
    /// Its path: in the repository, or on the machine for a package's file.
    pub path: CheckedPath,
    /// The digest of its text; empty for a symbolic link, which is never read.
    pub digest: crate::domain::digests::types::Digest,
    /// Its size and modification time when it was digested: while they do not move, it is not read again.
    pub stamp: crate::domain::files::types::Stamp,
}

/// The path of a file a check's answer rests on: in the repository, or on the machine for a package's file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckedPath(pub Box<str>);
