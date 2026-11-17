//! The types of the `packages` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! A package holds definitions by kind, name and version; an entry names one release of one of them.

/// A package's name: `idiomatic.arch`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageName(pub Box<str>);

/// A definition's name within its package: `crate`, `file-length`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionName(pub Box<str>);

/// A version as written: `0.12.0`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionText(pub Box<str>);

/// One number of a version.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionPart(pub u64);

/// A version read as its three numbers, ordered as semantic versions are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionNumber {
    /// The major number.
    pub major: VersionPart,
    /// The minor number.
    pub minor: VersionPart,
    /// The patch number.
    pub patch: VersionPart,
}

/// A definition of a package by what it is and its name, whatever its version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionKey {
    /// What it is.
    pub kind: DefinitionKind,
    /// Its name.
    pub name: DefinitionName,
}

/// What a definition is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefinitionKind {
    /// One check on code.
    Rule,
    /// The shape and checks of one kind of directory.
    Spec,
    /// The shape of one file.
    Template,
}

/// Whether a definition is its package's to share, or owned by one version of one spec.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ownership {
    /// A shared definition, at `<kind>s/<name>/<version>` in its package.
    Shared,
    /// Owned by this spec, at `specs/<spec>/<version>/<kind>s/<name>`.
    Owned(DefinitionName),
}

/// One release of one definition: what an entry names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Release {
    /// The package that holds it.
    pub package: PackageName,
    /// What it is.
    pub kind: DefinitionKind,
    /// Its name.
    pub name: DefinitionName,
    /// The version: its own, or its owning spec's.
    pub version: VersionText,
    /// Shared or owned.
    pub ownership: Ownership,
}

/// Where a package comes from, as a dependency writes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageSource {
    /// A directory, relative to the file that names it.
    Path(SourcePath),
    /// A git repository at a revision.
    Git(GitSource),
    /// The default registry.
    Registry,
}

/// A directory a dependency names, as written.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath(pub Box<str>);

/// The URL of a git repository a dependency names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepositoryUrl(pub Box<str>);

/// The revision of a git repository a dependency names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(pub Box<str>);

/// A git repository and the revision a dependency names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitSource {
    /// The repository.
    pub url: RepositoryUrl,
    /// The revision.
    pub revision: Revision,
}

/// One dependency: a package and its source.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dependency {
    /// The package.
    pub package: PackageName,
    /// Where it comes from.
    pub source: PackageSource,
}

/// A package's own manifest, `package.toml`: its name and its own dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageManifest {
    /// Its name.
    pub name: PackageName,
    /// The packages it depends on.
    pub dependencies: Vec<Dependency>,
}

/// A directory on the machine, as text: where a package was found.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageRoot(pub Box<str>);

/// A package found: where it is, how the lock names its source, and its manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPackage {
    /// Its directory on the machine.
    pub directory: PackageRoot,
    /// Its source as the lock records it: the directory from the workspace, for a package read from a path.
    pub pin: PackageSource,
    /// Its manifest.
    pub manifest: PackageManifest,
    /// Whether the workspace names it, or a package it names does.
    pub reach: Reach,
}

/// How a package is reached from the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// The workspace names it.
    Direct,
    /// A package the workspace uses names it.
    Transitive,
}

/// One release a lock records.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LockedRelease {
    /// What it is.
    pub kind: DefinitionKind,
    /// Its name.
    pub name: DefinitionName,
    /// Its version.
    pub version: VersionText,
}

/// One package a lock records, with the releases of it the workspace uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedPackage {
    /// Its name.
    pub name: PackageName,
    /// Its source as pinned.
    pub source: PackageSource,
    /// The packages it depends on.
    pub dependencies: Vec<PackageName>,
    /// The releases used, sorted.
    pub releases: Vec<LockedRelease>,
}

/// What `tastemaker.lock` records: the packages resolved, direct and transitive, and the releases used.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lock {
    /// The packages the workspace names directly.
    pub dependencies: Vec<PackageName>,
    /// Every package, sorted by name.
    pub packages: Vec<LockedPackage>,
}

/// How the lock on disk compares with the one this load makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockStatus {
    /// They are the same.
    Current,
    /// There is none on disk.
    Missing,
    /// They differ.
    Stale,
}

/// The directory a kind's definitions sit in within a package: `rules`, `specs`, `templates`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindDirectory(pub &'static str);

/// The definition file of a kind: `rule.toml`, `spec.toml`, `template.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindFile(pub &'static str);

/// What a kind reads as, as the lock and `create` write it: `rule`, `spec`, `template`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindWord<'a>(pub &'a str);

/// A release's directory within its package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseDirectory(pub String);

/// A release's definition file within its package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseFile(pub String);

/// The id a release's findings carry: `package/name`, or `package/spec/name` for an owned definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseId(pub String);

impl std::fmt::Display for KindDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::fmt::Display for KindFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::fmt::Display for KindWord<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::fmt::Display for ReleaseDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for ReleaseFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for ReleaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The Markdown changelog sections between two asset versions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeLog(pub Box<str>);

/// A definition, whatever its version: what it is, the package that holds it and its name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlacedDefinition {
    /// Rule, spec or template.
    pub kind: DefinitionKind,
    /// The package it is in.
    pub package: PackageName,
    /// Its name.
    pub name: DefinitionName,
}
