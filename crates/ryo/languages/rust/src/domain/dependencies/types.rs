//! The types of the `dependencies` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::targets::types::Replacement;
use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};

use crate::domain::conditions::types::PlatformCondition;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::BindingName;
use crate::domain::targets::types::Checkout;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::Patch;
use crate::domain::targets::types::Version;
use crate::domain::targets::types::WorkspaceDependency;

/// A dependency a package declares, on a package of the repository or outside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeclaredDependency {
    /// The manifest of the package depended on, when it lies in a directory of the repository; none for a package from a registry or a
    /// git repository that no `[patch]` puts in one, or a directory outside the repository.
    pub target: Option<RepoPath>,
    /// Where the manifest says the package comes from; a dependency taken from the workspace says what the workspace's entry says.
    pub origin: DependencyOrigin,
    /// The name the dependency is bound to.
    pub binding: BindingName,
    /// The name of the package depended on.
    pub package: PackageName,
    /// How it is depended on.
    pub kind: DependencyKind,
    /// The platform condition, as Cargo shows it; empty when there is none.
    pub condition: PlatformCondition,
    /// Whether it is needed only when asked for.
    pub optional: Optionality,
    /// The version it asks for, as the manifest writes it (the workspace's entry for one taken from the workspace); absent when it
    /// asks for none.
    pub requirement: Section<Version>,
    /// The revision of a git repository it names (the workspace's entry's for one taken from the workspace).
    pub checkout: Checkout,
    /// Whether it takes its requirement from the workspace's `[workspace.dependencies]` (`workspace = true`).
    pub inheritance: Inheritance,
    /// Where the root manifest's `[patch]` or `[replace]` takes its package from, a directory or a git repository, as written; absent
    /// when none does.
    pub replacement: Section<Replacement>,
}

/// What the root manifest declares for its members to take: where it is, the dependencies its `[workspace]` declares, and the
/// packages its `[patch]` and `[replace]` put in directories.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shared<'a> {
    /// The root manifest's directory.
    pub directory: &'a RepoPath,
    /// The dependencies its workspace declares; none when it is no workspace.
    pub dependencies: &'a [WorkspaceDependency],
    /// The packages it puts in directories.
    pub patches: &'a [Patch],
}
