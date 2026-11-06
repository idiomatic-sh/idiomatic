//! The types of the `packages` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages;

use crate::domain::patterns::types::Pattern;

/// The path of a manifest, relative to the root of the repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ManifestPath(pub Box<str>);

/// A package's name, as the manifest writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageName(pub Box<str>);

/// A package's name as PEP 503 normalizes it: the name two spellings of one package share.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Normalized(pub Box<str>);

/// A requirement as the manifest writes it: `name[extras] (>=1) ; marker`. A requirement that writes a URL or a path and no name of a
/// distribution writes an empty name: ` ; marker`, or nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Written(pub Box<str>);

/// The version a requirement asks for, as the manifest writes it, operators included (`==0.115.0`, `>=1,<2`, Poetry's `^1.2`);
/// empty when it asks for none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Specifier(pub Box<str>);

/// A revision of a git repository, as the manifest writes it: a branch's name, a tag, or a commit; empty when it names none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Revision(pub Box<str>);

/// The revision of a git repository a requirement or a source names, by what the manifest says it is: a `branch`, a `tag`, or a
/// `rev` (Pipenv's `ref`, the `@` of a git URL) that may be any of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Checkout {
    /// The branch.
    pub branch: Revision,
    /// The tag.
    pub tag: Revision,
    /// A revision not said to be a branch or a tag.
    pub revision: Revision,
}

/// The versions of Python a requirement applies under, as the manifest writes them on it (Poetry's `python = "^3.8"`); empty when it
/// writes none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Interpreter(pub Box<str>);

/// What the manifest puts in the place of a requirement, as it writes it: a requirement of uv's `override-dependencies`; empty when
/// nothing does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Replacement(pub Box<str>);

/// The condition a requirement carries after its `;`, as written; empty when it carries none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Marker(pub Box<str>);

/// What the tool said of a manifest it could not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reason(pub Box<str>);

/// Why a relation is read only in part: a manifest that was refused, with what the tool said, or an import that names a module of the
/// repository no file is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cause(pub Box<str>);

/// What a manifest says of the package it declares, if it declares one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Naming {
    /// The package's name, as written.
    Named(PackageName),
    /// The manifest declares no package: it holds settings, or a workspace alone.
    Unnamed,
}

/// A directory of the repository, relative to its root and normalized (no `.`, no `..`); empty for the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageLocation(pub Box<str>);

/// Where a requirement says the package it needs is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reference {
    /// By its name, which the written requirement holds.
    Named,
    /// In a directory of the repository.
    Located(PackageLocation),
    /// Somewhere the manifest does not let the repository name: a URL, a path outside the repository. It matches no package.
    Unresolvable,
}

/// A package whose requirement the manifest sends to a directory: the name stays, the place it is found at changes. A package with
/// several sources has a redirect for each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Redirect {
    /// The package it is about, as written.
    pub name: PackageName,
    /// The directory it is found in; none when the source is no path, or a path outside the repository, which matches no package of it.
    pub location: Option<PackageLocation>,
    /// Where the source says the package comes from.
    pub origin: languages::DependencyOrigin,
    /// The condition under which the source is the one used, as written; empty when it carries none.
    pub marker: Marker,
    /// The revision of its git repository the source names.
    pub checkout: Checkout,
}

/// A requirement of a package, as the manifest writes it, with the table it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Requirement {
    /// The requirement, as the manifest writes it.
    pub written: Written,
    /// How the package needs it, from the table it is in.
    pub kind: languages::DependencyKind,
    /// Whether it is needed only when asked for.
    pub optional: languages::Optionality,
    /// Where it says the package is.
    pub reference: Reference,
    /// Where it says the package comes from, before a source the manifest gives it elsewhere sends it on.
    pub origin: languages::DependencyOrigin,
    /// The version it asks for, as the manifest writes it.
    pub specifier: Specifier,
    /// Whether that version requirement admits one version only, read in the grammar of the manifest that writes it.
    pub exact: languages::Flag,
    /// The revision of a git repository it names.
    pub checkout: Checkout,
    /// The versions of Python it applies under, as written on it.
    pub interpreter: Interpreter,
}

/// What one manifest declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Declared {
    /// The package it declares.
    pub name: Naming,
    /// The patterns of the directories of the workspace's members, from the manifest's directory.
    pub members: Vec<Pattern>,
    /// The patterns of the directories the workspace leaves out.
    pub excludes: Vec<Pattern>,
    /// What the package needs.
    pub requirements: Vec<Requirement>,
    /// The requirements the manifest sends to a directory.
    pub sources: Vec<Redirect>,
    /// The requirements the manifest puts in the place of every requirement of the same name of its workspace, as written.
    pub overrides: Vec<Written>,
}

/// A package of the repository: where its manifest is and what it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Package {
    /// The manifest.
    pub manifest: ManifestPath,
    /// What the manifest declares.
    pub declared: Declared,
}

/// A manifest that could not be read, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Refusal {
    /// The manifest.
    pub manifest: ManifestPath,
    /// What the tool said.
    pub reason: Reason,
}

/// The packages of a repository, in the order of their manifests' paths, and the manifests that could not be read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Packages {
    /// The manifests that declare a package or a workspace.
    pub packages: Vec<Package>,
    /// The manifests the tool refused.
    pub refused: Vec<Refusal>,
}

/// A requirement of a package, on another package of the repository or on one outside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Dependency {
    /// The manifest of the package that needs.
    pub source: ManifestPath,
    /// The manifest of the package needed; none when the repository does not hold it.
    pub target: Option<ManifestPath>,
    /// The name as the manifest writes it; none for a requirement that writes no name.
    pub binding: Option<PackageName>,
    /// The name, normalized; none for a requirement that writes no name.
    pub package: Option<Normalized>,
    /// Where the manifest says the package comes from.
    pub origin: languages::DependencyOrigin,
    /// How it is needed.
    pub kind: languages::DependencyKind,
    /// The marker, as written.
    pub condition: Marker,
    /// Whether it is needed only when asked for.
    pub optional: languages::Optionality,
    /// The version it asks for, as the manifest writes it; empty when it asks for none.
    pub requirement: Specifier,
    /// Whether that version requirement admits one version only.
    pub exact: languages::Flag,
    /// The revision of a git repository it names, or its source names.
    pub checkout: Checkout,
    /// The versions of Python it applies under, as written on it.
    pub interpreter: Interpreter,
    /// What its workspace puts in its place, as written.
    pub replacement: Replacement,
}

/// A package a workspace holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageMembership {
    /// The manifest of the workspace.
    pub workspace: ManifestPath,
    /// The manifest of the member.
    pub package: ManifestPath,
}

/// What a requirement says: the name as written, the same normalized, and the marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Parsed {
    /// The name as written.
    pub written: PackageName,
    /// The name, normalized.
    pub name: Normalized,
    /// The marker as written.
    pub marker: Marker,
}

/// Where a requirement sends its package: a package of the repository, or none, with the condition its source adds to the requirement's
/// own and where that source says the package comes from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PackageTarget<'a> {
    /// The package of the repository the requirement resolves to; none for one outside it.
    pub package: Option<&'a Package>,
    /// The environment marker the source adds to the requirement's own, when it adds one.
    pub condition: Option<&'a Marker>,
    /// Where the source says the package comes from.
    pub origin: languages::DependencyOrigin,
    /// The revision of a git repository the source names, when the requirement has one.
    pub checkout: Option<&'a Checkout>,
}

/// A requirement between two manifests the run lists, as the relation `package_dependency` writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wiring {
    /// The file of the manifest of the package that needs.
    pub source: languages::FileId,
    /// The file of the manifest of the package needed.
    pub target: languages::FileId,
    /// What the requirement is.
    pub dependency: Dependency,
}

/// A workspace and one of its members, as the relation `workspace_member` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceMembership {
    /// The file of the manifest of the workspace.
    pub workspace: languages::FileId,
    /// The file of the manifest of the member.
    pub package: languages::FileId,
}

/// A file of the run and the manifest of its package, as the relation `file_package` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FilePlacement {
    /// The file.
    pub path: languages::FileId,
    /// The file of the manifest of its package.
    pub package: languages::FileId,
}

/// What a reader of the project writes its rows from: the rows themselves, and the manifests that were refused, for which the relation is
/// read only in part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wired<Row> {
    /// The rows.
    pub rows: Vec<Row>,
    /// Why each manifest the tool refused is missing from them.
    pub refused: Vec<Cause>,
}
