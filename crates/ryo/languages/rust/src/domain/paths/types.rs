//! The types of the `paths` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::{BTreeMap, BTreeSet};

/// Whether a path is a directory or lies below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lying {
    /// It is the directory, or below it.
    Within,
    /// It is elsewhere.
    Apart,
}

/// Whether a file owns the directory it is in as a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ownership {
    /// It does: it is `mod.rs`.
    Owns,
    /// It does not.
    Shares,
}

/// A path written relative to a directory: a name, a few names joined with `/`, or a path that climbs with `..`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Relative(pub Box<str>);

impl std::ops::Deref for Relative {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A part of a manifest, or of what a file system holds, that may be absent.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Section<T> {
    /// The manifest has none.
    #[default]
    Absent,
    /// The manifest has it.
    Present(T),
}

/// One thing in a directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    /// Its name.
    pub name: EntryName,
    /// Whether it is a directory.
    pub kind: EntryKind,
}

/// A place where a file might be, and what was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Look {
    /// Where: how far above the directory looked from.
    pub height: Height,
    /// The text of the file; none when it is not there.
    pub text: Section<FileText>,
}

/// How many directories above a directory one is: none for the directory itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Height(pub u32);

/// Where a path lands: in the repository, at a path from its root, or outside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// In the repository.
    Inside(RepoPath),
    /// Outside it.
    Outside,
}

/// The text of a file the machine holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FileText(pub Box<str>);

impl std::ops::Deref for FileText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for FileText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name of one thing in a directory.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EntryName(pub Box<str>);

impl std::ops::Deref for EntryName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for EntryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A place in a repository, written from its root with `/`; the root itself is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RepoPath(pub Box<str>);

impl std::ops::Deref for RepoPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for RepoPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether a thing in a directory is a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    /// A directory.
    Directory,
    /// Anything else.
    File,
}

/// Whether a path matches a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Matching {
    /// It does.
    Matches,
    /// It does not.
    Differs,
}

/// What a pass has been told of the things it asks about, and what it still wants told: the domain never reads, so a pass that meets
/// something unseen records it as wanted and goes on with a stand-in; whoever drives the pass reads what is wanted and runs it again.
#[derive(Clone, Debug)]
pub(crate) struct Asked<K: Ord, V> {
    /// What has been told, by what it is about.
    pub seen: BTreeMap<K, V>,
    /// What the pass met unseen since it began.
    pub wanted: BTreeSet<K>,
}

impl<K: Ord, V> Default for Asked<K, V> {
    fn default() -> Self {
        Self {
            seen: BTreeMap::new(),
            wanted: BTreeSet::new(),
        }
    }
}

/// Whether a pass met something unseen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settlement {
    /// It did not: the pass answered from what it was told.
    Settled,
    /// It did: what it answered stands on stand-ins, and what it wants is to be read.
    Wanting,
}

/// Whether the manifests a run supplies include a manifest of the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Supply {
    /// One is supplied.
    Manifest,
    /// None is.
    NoManifest,
}

/// Whether a run asks for relations that link the source files to one another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathLinking {
    /// It does.
    Asked,
    /// It does not.
    NotAsked,
}
