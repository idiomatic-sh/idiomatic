//! The types of the `packages` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::targets::types::Replacement;
use std::collections::{BTreeMap, BTreeSet};

use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};

use crate::domain::conditions::types::PlatformCondition;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::BindingName;
use crate::domain::targets::types::Checkout;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::Version;
use crate::domain::workspaces::types::Reason;

/// A package that belongs to a workspace, each by its manifest, relative to the repository root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct WorkspacePackageMembership {
    /// The workspace's manifest.
    pub workspace: RepoPath,
    /// The member package's manifest.
    pub package: RepoPath,
}

/// A dependency of a package, on another package of the repository or on one outside it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Dependency {
    /// The depending package's manifest.
    pub source: RepoPath,
    /// The manifest of the package depended on; none when the repository does not hold it.
    pub target: Option<RepoPath>,
    /// Where the manifest says the package comes from.
    pub origin: DependencyOrigin,
    /// The name the dependency is bound to.
    pub binding: BindingName,
    /// The name of the package depended on.
    pub package: PackageName,
    /// How it is depended on.
    pub kind: DependencyKind,
    /// The platform condition, empty when there is none.
    pub condition: PlatformCondition,
    /// Whether it is needed only when asked for.
    pub optional: Optionality,
    /// The version it asks for, as the manifest writes it; absent when it asks for none.
    pub requirement: Section<Version>,
    /// The revision of a git repository it names (the workspace's entry's for one taken from the workspace).
    pub checkout: Checkout,
    /// Whether it takes its requirement from the workspace's `[workspace.dependencies]` (`workspace = true`).
    pub inheritance: Inheritance,
    /// Where the root manifest's `[patch]` or `[replace]` takes its package from, a directory or a git repository, as written; absent
    /// when none does.
    pub replacement: Section<Replacement>,
}

/// A manifest that cannot be read as Cargo reads it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Refusal {
    /// The manifest, relative to the repository root.
    pub manifest: RepoPath,
    /// Why.
    pub reason: Reason,
}

/// The stage at which a manifest was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// The reader could not read the manifest.
    Parse,
    /// The manifest was read and its workspace is not consistent with it.
    Validation,
}

/// A manifest refused, as the relation `manifest_diagnostic` says of it: the file, the stage and what the reader said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Diagnosed {
    /// The manifest, by its place among the files of the run.
    pub file: ryo::languages::FileId,
    /// The stage.
    pub stage: Stage,
    /// What the reader said.
    pub reason: Reason,
}

/// What the manifests of a repository declare.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Packages {
    /// Each workspace with its members, in order.
    pub members: Vec<WorkspacePackageMembership>,
    /// Each dependency on a package of the repository, in order.
    pub dependencies: Vec<Dependency>,
    /// The source file of every target Cargo declares for each package, by the directory of the package.
    pub roots: BTreeMap<RepoPath, BTreeSet<RepoPath>>,
    /// The directories the workspaces write their outputs to: no package holds what lies in them.
    pub outputs: Vec<RepoPath>,
    /// The manifests that could not be read, with why: the others were read.
    pub refused: Vec<Refusal>,
}

/// Why a relation is read only in part, as a sentence a finding shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// A manifest was refused.
    Refused {
        /// The manifest.
        manifest: RepoPath,
        /// Why.
        reason: Reason,
    },
    /// A dependency is on a package no workspace holds.
    Unanalyzed {
        /// The depending package's manifest.
        depender: RepoPath,
        /// The manifest of the package depended on.
        target: RepoPath,
    },
}

impl std::fmt::Display for Cause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused { manifest, reason } => {
                write!(f, "`{manifest}` was refused: {reason}")
            }
            Self::Unanalyzed { depender, target } => {
                write!(
                    f,
                    "{depender}: dependency package {target} was not analyzed"
                )
            }
        }
    }
}

/// What the manifests declared, with the files of the manifests they name among the files the run lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Located {
    /// What the manifests declared.
    pub packages: Packages,
    /// The number of the file of each manifest `packages` names: its place among the files the run lists, or after them.
    pub listed: BTreeMap<RepoPath, ryo::languages::FileId>,
}

/// Whether the package a dependency is on was read by some workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Analysis {
    /// No workspace that was read holds it: what it declares is missing.
    Unanalyzed,
    /// A workspace that was read holds it.
    Analyzed,
}
