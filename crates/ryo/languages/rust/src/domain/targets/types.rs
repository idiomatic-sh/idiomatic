//! The types of the `targets` concept: what a manifest declares, as the packages and their targets need it. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::paths::types::Section;
use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};

/// What a `Cargo.toml` declares, as far as the packages and their targets are concerned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// The `[package]`.
    pub package: Section<PackageSection>,
    /// The `[workspace]`.
    pub workspace: Section<WorkspaceSection>,
    /// The `[lib]`.
    pub lib: Section<LibrarySection>,
    /// The `[[bin]]`.
    pub bins: Vec<TargetSection>,
    /// The `[[example]]`.
    pub examples: Vec<TargetSection>,
    /// The `[[test]]`.
    pub tests: Vec<TargetSection>,
    /// The `[[bench]]`.
    pub benches: Vec<TargetSection>,
    /// Every dependency, whatever table, kind or platform declares it.
    pub dependencies: Vec<DependencySection>,
    /// The packages its `[patch]` and `[replace]` put in directories: Cargo reads them in the root manifest alone.
    pub patches: Vec<Patch>,
    /// Whether any value of the manifest is taken from its workspace.
    pub inherits: Inheritance,
}

/// The `[package]` of a manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PackageSection {
    /// Its name.
    pub name: PackageName,
    /// The workspace root it says it belongs to, when it says.
    pub workspace: Section<WrittenPath>,
    /// Its build script.
    pub build: Build,
    /// Whether the library is found by its place.
    pub autolib: Discovery,
    /// Whether the binaries are found by their place.
    pub autobins: Discovery,
    /// Whether the examples are found by their place.
    pub autoexamples: Discovery,
    /// Whether the tests are found by their place.
    pub autotests: Discovery,
    /// Whether the benchmarks are found by their place.
    pub autobenches: Discovery,
}

/// Where a package's build script is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Build {
    /// `build.rs` if there is one.
    #[default]
    Found,
    /// None.
    Disabled,
    /// This file.
    At(WrittenPath),
}

/// The `[lib]` of a manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LibrarySection {
    /// Where it is, when it says.
    pub path: Section<WrittenPath>,
    /// The name the code of a dependent writes for it, when it says.
    pub name: Section<TargetName>,
}

/// A `[[bin]]`, `[[example]]`, `[[test]]` or `[[bench]]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TargetSection {
    /// Its name, when it says.
    pub name: Section<TargetName>,
    /// Where it is, when it says.
    pub path: Section<WrittenPath>,
}

/// A dependency a manifest declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DependencySection {
    /// The key it is declared under.
    pub key: BindingName,
    /// How it is depended on.
    pub kind: DependencyKind,
    /// The platform it is for, as written; none when it is for all.
    pub platform: Section<Platform>,
    /// The version it asks for, when it asks.
    pub version: Section<Version>,
    /// The directory it names, when it names one.
    pub path: Section<WrittenPath>,
    /// The package it names, when it renames one.
    pub package: Section<PackageName>,
    /// Where it is taken from when it names no directory.
    pub source: Source,
    /// Where the table says the package comes from: a `path`, a `git` repository, or else a registry.
    pub origin: DependencyOrigin,
    /// Whether it is needed only when asked for.
    pub optional: Optionality,
    /// Whether it takes the rest from the workspace's.
    pub workspace: Inheritance,
    /// The revision of its git repository it names, when it names one.
    pub checkout: Checkout,
}

/// The revision of a git repository a dependency names, by the key it is written under: `branch`, `tag` or `rev`; each absent when
/// it is not written.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Checkout {
    /// The `branch`.
    pub branch: Section<Revision>,
    /// The `tag`.
    pub tag: Section<Revision>,
    /// The `rev`.
    pub rev: Section<Revision>,
}

/// A revision of a git repository, as the manifest writes it: a branch's name, a tag, or a commit.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Revision(pub Box<str>);

/// The `[workspace]` of a manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceSection {
    /// The directories it holds, as written: patterns among them.
    pub members: Vec<Pattern>,
    /// The directories it leaves out.
    pub exclude: Vec<Pattern>,
    /// The dependencies its members may take.
    pub dependencies: Vec<WorkspaceDependency>,
}

/// A dependency a workspace declares for its members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceDependency {
    /// The key it is declared under.
    pub key: BindingName,
    /// The version it asks for, when it asks.
    pub version: Section<Version>,
    /// The directory it names, when it names one.
    pub path: Section<WrittenPath>,
    /// The package it names, when it renames one.
    pub package: Section<PackageName>,
    /// Where it is taken from when it names no directory.
    pub source: Source,
    /// Where the table says the package comes from: a `path`, a `git` repository, or else a registry.
    pub origin: DependencyOrigin,
    /// The revision of its git repository it names, when it names one.
    pub checkout: Checkout,
}

/// Where a dependency that names no directory is taken from, as `[patch]` names it: `crates-io`, the name of another registry, or the
/// URL of a git repository.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Source(pub Box<str>);

/// A package the root manifest's `[patch]` or `[replace]` puts in a directory or takes from a git repository, in the place of the one a
/// source has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Patch {
    /// The source whose package it takes the place of.
    pub source: Source,
    /// The package.
    pub package: PackageName,
    /// Where the package is taken from instead.
    pub replacement: Replacement,
}

/// Where `[patch]` or `[replace]` takes a package from instead of its source.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Replacement {
    /// A directory, as written.
    Directory(WrittenPath),
    /// A git repository, by its URL as written.
    Repository(Source),
}

/// What a Cargo configuration says that the answer depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Config {
    /// Where Cargo writes its outputs, as written.
    pub target_dir: Section<WrittenPath>,
}

/// The name of a package.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PackageName(pub Box<str>);

impl std::ops::Deref for PackageName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name a dependency is bound to: the key it is declared under.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BindingName(pub Box<str>);

impl std::ops::Deref for BindingName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for BindingName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The platform a dependency is for, as the manifest writes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Platform(pub Box<str>);

impl std::ops::Deref for Platform {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The version a dependency asks for, as the manifest writes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Version(pub Box<str>);

impl std::ops::Deref for Version {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A path as a manifest writes it: relative to the manifest, `..` and all.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WrittenPath(pub Box<str>);

impl std::ops::Deref for WrittenPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for WrittenPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A directory or a pattern of directories, as a manifest writes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Pattern(pub Box<str>);

impl std::ops::Deref for Pattern {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name of a target.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TargetName(pub Box<str>);

impl std::ops::Deref for TargetName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for TargetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether a value is taken from the workspace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Inheritance {
    /// It is the manifest's own.
    #[default]
    Own,
    /// It is the workspace's.
    Workspace,
}

/// Whether Cargo finds the targets of a kind by where their files are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Discovery {
    /// Cargo finds them: the ones declared and the ones whose files are where Cargo looks.
    Automatic,
    /// Only the ones declared.
    #[default]
    Declared,
}

/// Whether a file is in a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Presence {
    /// It is.
    Present,
    /// It is not.
    Missing,
}
