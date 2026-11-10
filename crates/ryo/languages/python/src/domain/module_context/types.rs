//! Real file identities and explicitly incomplete module context.
use ryo::languages::{FileId, FilePath, NameText};
use std::{collections::BTreeMap, path::PathBuf};
/// One project-established module name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleIdentity {
    /// Requested source file, never an initializer placeholder.
    pub file: FileId,
    /// Dotted module name with the initializer component retained.
    pub path: ModuleDottedPath,
    /// Source filename stem, retaining embedded dots and original case.
    pub module_name: ModuleName,
    /// Containing package directory basename, retaining dots and original case.
    pub package_name: PackageDirectoryName,
    /// Highest contiguous package directory basename, preserving punctuation and case.
    pub package_root_name: PackageRootName,
}
/// Module context established for this run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleContexts {
    /// Known source module identities.
    pub rows: Vec<ModuleIdentity>,
    /// A root name or required external context was not available.
    pub partial: PartialContext,
    /// Every examined marker, including the first absent boundary marker.
    pub consulted: Vec<FilePath>,
}
/// Concrete package marker observations; no filesystem access belongs to this value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Markers(pub BTreeMap<PathBuf, bool>);
/// Import identity plus its exact package-root boundary, without filesystem state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageModule {
    /// Qualified path retains every raw directory component.
    pub path: ModuleDottedPath,
    /// Raw highest contiguous package basename.
    pub package_root_name: PackageRootName,
}
/// Next outcome while resolving one file against known marker observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ModuleDecision {
    /// A module identity or a proven standalone file.
    Known(Option<PackageModule>),
    /// The next marker that must be observed before a conclusion is possible.
    Need(PathBuf),
    /// The inventory reaches an unnamed root whose package identity cannot be established.
    Unknown,
}
/// A dotted module name with the initializer component retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleDottedPath(pub String);
/// A source filename stem, retaining embedded dots and original case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleName(pub String);
/// A containing package directory basename.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageDirectoryName(pub String);
/// The basename of the highest contiguous package directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageRootName(pub String);
/// Whether a root name or required external context was not available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PartialContext(pub bool);
/// The raw source filename stem and containing directory basename of one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Basenames<'a> {
    /// The filename stem.
    pub module: NameText<'a>,
    /// The containing directory basename.
    pub package: NameText<'a>,
}
