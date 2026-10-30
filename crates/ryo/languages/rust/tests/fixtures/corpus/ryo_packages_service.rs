//! The `packages` service: the manifests of a repository read as Cargo reads them, with no IO and no ports: which workspace holds
//! each package, the source file of each target, the dependencies between packages by path, and where the outputs go.

use crate::domain::workspaces::types::Exclusion;
use std::collections::BTreeSet;
use std::path::Path;

use super::constants::OUTPUTS;
use super::errors::PackagesError;
use super::types::{Analysis, Cause, Dependency, Membership, Packages, Refusal};
use crate::domain::dependencies::service::declared_of;
use crate::domain::dependencies::types::Shared;
use crate::domain::paths::constants::{MANIFEST, SEPARATOR};
use crate::domain::paths::service::{directory_of, name_of};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::service::targets_of;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::Manifest;
use crate::domain::workspaces::service::{
    configurations_above, excluded, load, members_of, root_of,
};
use crate::domain::workspaces::types::{Configured, Looking, Reason};
use ryo::languages;

/// Where the outputs of the workspace in `directory` go, relative to the root when they go inside it.
fn output_of(looking: &mut Looking, directory: &RepoPath) -> Option<RepoPath> {
    let absolute = looking.root.clone();
    let root = Path::new(&*absolute);
    let target: std::path::PathBuf = configurations_above(looking, directory)
        .into_iter()
        .find_map(|Configured { directory, config }| match config.target_dir {
            Section::Present(target) => Some(Path::new(&*directory).join(&*target)),
            Section::Absent => None,
        })
        .unwrap_or_else(|| root.join(&**directory).join(OUTPUTS));
    target
        .strip_prefix(root)
        .ok()
        .map(|inside| RepoPath(inside.to_string_lossy().into()))
}

/// The manifest of the workspace the package at `candidate` is in: its own when it is in none or is left out of the one above it.
///
/// # Errors
///
/// `NotAMember` when a workspace above it neither holds it nor leaves it out, `NoWorkspaceToInheritFrom` when it takes a value from a
/// workspace it is not in, `Workspaces` when the workspace cannot be read.
pub(crate) fn root_for(
    looking: &mut Looking,
    candidate: &RepoPath,
    manifest: &Manifest,
) -> Result<RepoPath, PackagesError> {
    let mut root = root_of(looking, candidate).unwrap_or_else(|| candidate.clone());
    if root != *candidate {
        if members_of(looking, &root)?.contains(candidate) {
            return Ok(root);
        }
        let left_out = load(looking, &root).is_ok_and(|each| {
            matches!(&each.workspace, Section::Present(workspace)
                if excluded(workspace, &directory_of(&root), &directory_of(candidate)) == Exclusion::Excluded)
        });
        if !left_out {
            return Err(PackagesError::NotAMember);
        }
        root = candidate.clone();
    }
    if root == *candidate
        && matches!(manifest.workspace, Section::Absent)
        && manifest.inherits == Inheritance::Workspace
    {
        return Err(PackagesError::NoWorkspaceToInheritFrom);
    }
    Ok(root)
}

/// What the manifests of the repository declare. `manifests` are the paths of the files named `Cargo.toml`.
#[must_use]
pub(crate) fn read(manifests: &[RepoPath], looking: &mut Looking) -> Packages {
    let mut candidates: Vec<&RepoPath> = manifests
        .iter()
        .filter(|path| *name_of(path).0 == *MANIFEST)
        .collect();
    candidates.sort_by_key(|path| (path.split(SEPARATOR).count(), (*path).clone()));
    let mut covered: BTreeSet<RepoPath> = BTreeSet::new();
    let mut packages = Packages::default();
    for candidate in candidates {
        if covered.contains(candidate) {
            continue;
        }
        let refuse = |reason: Reason| Refusal {
            manifest: candidate.clone(),
            reason,
        };
        let manifest = match load(looking, candidate) {
            Ok(manifest) => manifest,
            Err(reason) => {
                packages.refused.push(refuse(reason));
                continue;
            }
        };
        let root = match root_for(looking, candidate, &manifest) {
            Ok(root) => root,
            Err(error) => {
                packages
                    .refused
                    .push(refuse(Reason(error.to_string().into())));
                continue;
            }
        };
        let members = match members_of(looking, &root) {
            Ok(members) => members,
            Err(error) => {
                packages
                    .refused
                    .push(refuse(Reason(error.to_string().into())));
                continue;
            }
        };
        let root_manifest: Manifest = load(looking, &root).unwrap_or_default();
        let root_directory = directory_of(&root);
        packages.outputs.extend(output_of(looking, &root_directory));
        for member in &members {
            covered.insert(member.clone());
            let Ok(each) = load(looking, member) else {
                continue;
            };
            let Section::Present(package) = &each.package else {
                continue;
            };
            let directory = directory_of(member);
            packages
                .roots
                .entry(directory.clone())
                .or_default()
                .extend(targets_of(
                    &directory,
                    &each,
                    package,
                    &mut looking.directories,
                ));
            packages.members.push(Membership {
                workspace: root.clone(),
                package: member.clone(),
            });
            let shared = match &root_manifest.workspace {
                Section::Present(workspace) => Some(Shared {
                    directory: &root_directory,
                    dependencies: &workspace.dependencies,
                }),
                Section::Absent => None,
            };
            for dependency in declared_of(&directory, &each, shared, &mut looking.platforms) {
                packages.dependencies.push(Dependency {
                    source: member.clone(),
                    target: dependency.target,
                    binding: dependency.binding,
                    package: dependency.package,
                    kind: dependency.kind,
                    condition: dependency.condition,
                    optional: dependency.optional,
                });
            }
        }
        covered.insert(root);
    }
    packages.outputs.sort();
    packages.outputs.dedup();
    packages.members.sort();
    packages.members.dedup();
    packages.dependencies.sort();
    packages.dependencies.dedup();
    packages.refused.sort();
    packages
}

/// The name `text` as a name made after the files were read: the manifests' texts are in no file's bytes.
pub(crate) fn named(
    symbols: &mut languages::Symbols,
    text: languages::NameText<'_>,
) -> languages::Symbol {
    languages::intern_derived(symbols, languages::ByteSlice(&[]), text)
}

/// Why a manifest Cargo refused leaves the rows of a relation partly read.
#[must_use]
pub(crate) fn refusal_cause(refusal: &Refusal) -> Cause {
    Cause::Refused {
        manifest: refusal.manifest.clone(),
        reason: refusal.reason.clone(),
    }
}

/// Whether the package a dependency is on is held by no workspace that was read: what it declares is missing.
#[must_use]
pub(crate) fn is_unanalyzed(packages: &Packages, dependency: &Dependency) -> Analysis {
    if packages
        .members
        .iter()
        .any(|member| member.package == dependency.target)
    {
        Analysis::Analyzed
    } else {
        Analysis::Unanalyzed
    }
}

/// Why a dependency on a package that no workspace holds leaves the rows partly read.
#[must_use]
pub(crate) fn unanalyzed_cause(dependency: &Dependency) -> Cause {
    Cause::Unanalyzed {
        depender: dependency.source.clone(),
        target: dependency.target.clone(),
    }
}
