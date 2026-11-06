//! Repository package declarations and the paths they connect.

use ryo::languages;
use std::collections::BTreeMap;

/// A manifest's path from the repository root, as the rows and the facts name it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ManifestPath(pub Box<str>);

/// The name a package is published or depended on under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PackageName(pub Box<str>);

/// The name of a catalog a dependency spec points at; empty for the default catalog.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CatalogName(pub Box<str>);

/// A version range a catalog gives a dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Version(pub Box<str>);

/// One catalog: the version range it gives each dependency, by the dependency's name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Catalog(pub BTreeMap<PackageName, Version>);

/// A dependency name as the manifest writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Binding<'a>(pub &'a str);

/// A dependency version spec as the manifest writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Spec<'a>(pub &'a str);

/// The version ranges a workspace names once: a default catalog and named catalogs, each keyed by dependency name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Catalogs {
    /// The catalog a bare `catalog:` spec points at.
    pub default: Catalog,
    /// The catalogs a `catalog:<name>` spec points at, by name.
    pub named: BTreeMap<CatalogName, Catalog>,
}

/// One dependency a manifest declares: what it binds, on which package, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Requirement {
    /// The name the dependent imports the package by, the manifest's key.
    pub binding: PackageName,
    /// The package the dependency resolves to; an `npm:` alias names another than the binding.
    pub package: PackageName,
    /// The manifest a path or workspace spec points at, when the spec names one.
    pub target: Option<ManifestPath>,
    /// The section the manifest declares it in: dependencies, dev, peer or optional.
    pub kind: languages::DependencyKind,
    /// Whether the dependent works without it.
    pub optional: languages::Optionality,
    /// Where the spec says the package comes from; a `catalog:` spec, until it is resolved, says a registry.
    pub origin: languages::DependencyOrigin,
    /// The catalog the spec points at, until a workspace's catalogs resolve it.
    pub catalog: Option<CatalogName>,
    /// The version spec as the manifest writes it (`^4.6.0`, `workspace:*`, `npm:other@1.2.3`); for a `catalog:` spec, once resolved,
    /// the range the catalog writes. Empty for a dependency no spec declares: a project reference.
    pub spec: Version,
    /// The `catalog:` spec as the manifest writes it (`catalog:`, `catalog:react18`), kept once the catalog resolves it; empty for a
    /// spec the manifest writes itself.
    pub written_catalog: Version,
}

/// What one manifest declares, as one adapter or several read it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Declared {
    /// The package's name, when the manifest gives one.
    pub name: Option<PackageName>,
    /// The manifests of the workspace's member packages.
    pub members: Vec<ManifestPath>,
    /// The dependencies the manifest declares.
    pub requirements: Vec<Requirement>,
    /// The version ranges a workspace manifest declares for its members.
    pub catalogs: Catalogs,
    /// The specs a root manifest puts in the place of every dependency on a package, by the package's name, as written: npm's
    /// `overrides`, Yarn's and Bun's `resolutions`, pnpm's `overrides`, each a key that names a package alone.
    pub overrides: BTreeMap<PackageName, Version>,
}

/// What one adapter read from one manifest, and the package manifest it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fact {
    /// The manifest the adapter read.
    pub manifest: ManifestPath,
    /// The package manifest what was read belongs to; the manifest itself when it is one.
    pub about: ManifestPath,
    /// What the adapter read from it.
    pub declared: Declared,
}

/// A package of the repository: its manifest and everything read from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Package {
    /// The package manifest the package is declared by.
    pub manifest: ManifestPath,
    /// What every adapter read of it, combined.
    pub declared: Declared,
}

/// Every package of the repository, and the manifests that could not be read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Packages {
    /// The packages, in the order their facts came.
    pub entries: Vec<Package>,
    /// The manifests refused, with what the reader said.
    pub refused: Vec<languages::Refused>,
}

/// The stage at which a manifest was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// The reader could not read the manifest.
    Parse,
    /// The manifest was read and a reference in it does not hold.
    Validation,
}

/// The name a manifest is read as, which is the name of its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadAs(pub Box<str>);

/// A manifest refused, as the relation `manifest-diagnostic` says of it: the file, the schema it is read as, the stage and what was said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Diagnosed {
    /// The manifest, by its place among the files of the run.
    pub file: languages::FileId,
    /// The name the manifest is read as.
    pub read_as: ReadAs,
    /// The stage.
    pub stage: Stage,
    /// What was said.
    pub reason: languages::ToolMessage,
}

/// One `package-dependency` row before it is written: the two manifests and the requirement between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Dependency {
    /// The manifest that declares the dependency.
    pub source: languages::FileId,
    /// The manifest of the package depended on; no file for a package the repository does not hold.
    pub target: languages::FileId,
    /// What the dependent declared.
    pub requirement: Requirement,
    /// The spec the manifest, or the workspace that holds it, puts in the dependency's place, as written; empty when none does.
    pub replacement: Version,
}

/// The rows a manifest relation answers, and the manifests refused on the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Projection<Row> {
    /// The rows, in order.
    pub rows: Vec<Row>,
    /// The manifests no row could be read from, with why.
    pub refused: Vec<languages::Refused>,
}

/// A path relative to the root of the repository, borrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RepositoryPath<'a>(pub &'a str);

/// A directory, relative to the root of the repository; empty for the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Directory<'a>(pub &'a str);
/// Whether a file is an npm package manifest or another file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ManifestKind {
    /// A `package.json`: a package of its own.
    Package,
    /// Any other file, which is no package of its own.
    Other,
}
/// Whether a path belongs under a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Containment {
    /// The path is in the directory or below it.
    Inside,
    /// The path is somewhere else.
    Outside,
}
