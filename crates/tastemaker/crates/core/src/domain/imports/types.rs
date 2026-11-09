//! The types of the `imports` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::comments::types::ByteSpan;
use crate::domain::entries::types::{EntryIdentity, ImportDefault};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{EntryIndex, ScopeIndex};

/// What the imports of one scope are held to: the names of its entries, which entries each file is of, and the entries
/// whose `imports` hold their instances.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScopeImports {
    /// Every entry of the scope by the name other entries refer to it by.
    pub names: BTreeSet<EntryName>,
    /// The entries of the scope each file is of, by name.
    pub entries_of: BTreeMap<RepoPath, Vec<EntryName>>,
    /// The entries with an `imports`.
    pub checks: Vec<EntryImports>,
}

/// One entry's `imports` and the instances it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryImports {
    /// The entry's identity: its findings are `<identity>#imports`.
    pub identity: EntryIdentity,
    /// What is imported unless listed.
    pub default: ImportDefault,
    /// What it allows: entry names, or module patterns.
    pub allow: Vec<AllowItem>,
    /// Each instance of the entry.
    pub members: Vec<Member>,
}

/// One instance of an entry: how it is named, the files it is, and what its directory's `imports-allow` adds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// The instance: its directory, its file, or the rule entry's name.
    pub instance: InstanceName,
    /// Its files.
    pub files: BTreeSet<RepoPath>,
    /// The module patterns its directory's `imports-allow` adds to the entry's `allow`.
    pub extra: Vec<AllowItem>,
}

/// An import a file makes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import {
    /// The file that makes it.
    pub source: RepoPath,
    /// Where in the file.
    pub span: ImportPlace,
    /// The path as written.
    pub written: WrittenPath,
    /// What it reaches.
    pub target: Target,
}

/// What an import reaches.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// A file of the repository: a module, or the manifest of a package of the repository.
    File(RepoPath),
    /// A module outside the repository.
    Module,
}

/// A name an import brings into scope: the `static-import` relation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name {
    /// The file that imports it.
    pub source: RepoPath,
    /// The path as written.
    pub written: WrittenPath,
    /// The name it binds in the file, when it binds one: a path written at a use site that starts at it, where the
    /// import binds, is this import's. An import that binds no name (a side-effect `import "x"`) is still an import.
    pub binding: Option<ModuleName>,
    /// Where.
    pub span: ImportPlace,
}

/// A block that holds a `use`, or an inline module, of a file: what a `use` written in it binds within.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scope {
    /// Where.
    pub span: ImportPlace,
    /// Which.
    pub kind: ScopeKind,
}

/// Which scope a [`Scope`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeKind {
    /// A block: what is written within it sees the `use`s of the scopes around it.
    Block,
    /// An inline module: what is written within it sees none of the `use`s written outside it.
    Module,
}

/// A package a manifest depends on: the `package-dependency` relation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    /// The manifest.
    pub source: RepoPath,
    /// The dependency's manifest.
    pub target: RepoPath,
    /// The name code imports it by.
    pub binding: PackageBinding,
}

/// Which import relation a built-in rule reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Relation {
    /// `source-dependency`: the paths that name another file.
    Edges,
    /// `static-import`: the names a `use` brings in.
    Names,
    /// `package-dependency`: the packages a manifest depends on.
    Packages,
    /// `file-package`: the package each file belongs to.
    Owners,
    /// `syntax-tree`: the blocks and inline modules a `use` binds within.
    Scopes,
}

/// The rows the imports are read from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportRows {
    /// Every `source-dependency` row: a path one source file writes, as the file writes it at its span with its whitespace
    /// left out, that names a file of the repository another source file owns.
    pub edges: Vec<Import>,
    /// Every `static-import` row.
    pub names: Vec<Name>,
    /// Every `package-dependency` row.
    pub packages: Vec<Package>,
    /// Every `file-package` row.
    pub owners: Vec<Owner>,
    /// Each file's blocks that hold a `use` and its inline modules, the file kept once.
    pub scopes: BTreeMap<RepoPath, Vec<Scope>>,
}

/// The package a file belongs to: the `file-package` relation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Owner {
    /// The file.
    pub path: RepoPath,
    /// The manifest of its package.
    pub package: RepoPath,
}

/// Why an import breaks an entry's `imports`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breach {
    /// The entry allows only what it lists, and nothing listed matches.
    NotAllowed,
}

/// An import an entry's `imports` refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    /// The entry's identity.
    pub identity: EntryIdentity,
    /// The instance that makes it.
    pub instance: InstanceName,
    /// The import.
    pub import: Import,
    /// Why.
    pub breach: Breach,
}

/// The name an entry of a scope is referred to by: its alias, or its definition's name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryName(pub Box<str>);

impl std::ops::Deref for EntryName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EntryName {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for EntryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::borrow::Borrow<str> for EntryName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// What an entry's `imports` allows: the name of an entry of its scope, or a module pattern.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllowItem(pub Box<str>);

impl std::ops::Deref for AllowItem {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AllowItem {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for AllowItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An instance of an entry, as a finding names it: its directory, its file, or the rule entry's name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceName(pub Box<str>);

impl std::ops::Deref for InstanceName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for InstanceName {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for InstanceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A module path as a file writes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WrittenPath(pub Box<str>);

impl std::ops::Deref for WrittenPath {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WrittenPath {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for WrittenPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name code imports a package by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageBinding(pub Box<str>);

/// One name of a module path or of a package's binding, as the path is read: whatever separates two names, and each
/// joiner read as the first.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleName(pub Box<str>);

impl std::ops::Deref for PackageBinding {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PackageBinding {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for PackageBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A module a written path lies in, written as the matching port reads a path: `a/b`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModulePrefix(pub Box<str>);

impl std::ops::Deref for ModulePrefix {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ModulePrefix {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for ModulePrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An `imports` item as the matching port reads it: its segments as a path's, `**` for a `*`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModulePattern(pub Box<str>);

impl std::ops::Deref for ModulePattern {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ModulePattern {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for ModulePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where in its file an import is written, when the row says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImportPlace {
    /// These bytes.
    At(ByteSpan),
    /// The row gives no place.
    Unplaced,
}

/// Whether a module pattern of an `imports` matches a written path or a module it lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternMatch {
    /// It matches.
    Matches,
    /// It does not.
    Misses,
}

/// Whether a file is test code, whose imports no `imports` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
    /// A test, a bench or a `_test` twin.
    Test,
    /// Production code.
    Production,
}

/// The name of a column of a built-in import rule's row, as the rows are read by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportColumn<'a>(pub &'a str);

/// The place of an entry in its scope, by the scope's place and its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryPlace {
    /// The scope's place among the placement's scopes.
    pub scope: ScopeIndex,
    /// The entry's place among the scope's entries of its kind.
    pub entry: EntryIndex,
}

/// What `checks` looks up: the `imports-allow` of each directory, the files each template entry placed, by its scope and
/// its place, and the instances each spec entry made, likewise.
#[derive(Debug)]
pub struct Indexes<'p> {
    /// The module patterns each directory's `imports-allow` adds.
    pub extra: BTreeMap<RepoPath, Vec<AllowItem>>,
    /// The files each template entry placed, by its scope and its place.
    pub placed_by: std::collections::HashMap<EntryPlace, Vec<&'p RepoPath>>,
    /// The instances each spec entry made, likewise.
    pub instances_of: std::collections::HashMap<EntryPlace, Vec<ScopeIndex>>,
}
