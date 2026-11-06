//! Import-root identities and their explicitly three-state membership.
use ryo::languages::FileId;

/// The first component of an absolute import path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportRootName(pub Box<str>);

/// The dotted path an import names, or one dotted prefix of it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportPath(pub Box<str>);

/// The filesystem spelling of a dotted path, below a search root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FilesystemPath(pub String);

/// One dotted prefix of an import with its filesystem spelling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Prefix {
    /// The dotted spelling.
    pub dotted: ImportPath,
    /// The `/` spelling.
    pub filesystem: FilesystemPath,
}

impl std::ops::Deref for ImportRootName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ImportRootName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ImportRootName {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl From<String> for ImportRootName {
    fn from(text: String) -> Self {
        Self(text.into())
    }
}

impl std::ops::Deref for ImportPath {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ImportPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ImportPath {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl From<String> for ImportPath {
    fn from(text: String) -> Self {
        Self(text.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportedRoot {
    pub file: FileId,
    pub root: ImportRootName,
    pub path: ImportPath,
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
pub(crate) struct NamespaceMembership {
    pub file: FileId,
    pub root: ImportRootName,
    pub path: ImportPath,
    pub state: MembershipState,
    pub kind: EntryKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NamespaceMemberships {
    pub rows: Vec<NamespaceMembership>,
    pub incomplete_notes: Vec<FileId>,
}

/// The path of one candidate entry below a search root, before it is joined to the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EntryName(pub String);

/// What one candidate path is asked to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryShape {
    /// A file.
    File,
    /// A directory.
    Directory,
}

/// What the engine observed at one candidate path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Presence {
    /// The entry exists with the asked shape.
    Present,
    /// The entry does not exist with the asked shape.
    Absent,
}

impl From<bool> for Presence {
    fn from(exists: bool) -> Self {
        if exists { Self::Present } else { Self::Absent }
    }
}
