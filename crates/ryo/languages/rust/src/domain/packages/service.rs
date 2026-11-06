//! The `packages` service: the manifests of a repository read as Cargo reads them, with no IO and no ports: which workspace holds
//! each package, the source file of each target, the dependencies between packages by path, and where the outputs go.

use crate::domain::workspaces::types::Exclusion;
use std::collections::{BTreeMap, BTreeSet};

use super::constants::OUTPUTS;
use super::errors::PackagesError;
use super::types::{
    Analysis, Cause, Dependency, Diagnosed, Packages, Refusal, Stage, WorkspacePackageMembership,
};
use crate::domain::dependencies::service::declared_of;
use crate::domain::dependencies::types::{DeclaredDependency, Shared};
use crate::domain::package_index::types::ListIndex;
use crate::domain::paths::constants::{MANIFEST, SEPARATOR};
use crate::domain::paths::service::{directory_of, lexical_join, name_of};
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{Placement, Relative, RepoPath};
use crate::domain::targets::service::targets_of;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::Manifest;
use crate::domain::workspaces::service::{
    configurations_above, excluded, load, members_of, root_of,
};
use crate::domain::workspaces::types::{Looking, Reason, WorkspaceConfiguration};
use ryo::languages;

/// Where the outputs of the workspace in `directory` go, relative to the root when they go inside it: where the nearest configuration
/// that names a directory for them says, else `target` in the workspace's directory.
fn output_of(looking: &mut Looking, directory: &RepoPath) -> Option<RepoPath> {
    let placed = configurations_above(looking, directory)
        .into_iter()
        .find_map(|WorkspaceConfiguration { target_dir }| match target_dir {
            Section::Present(placement) => Some(placement),
            Section::Absent => None,
        })
        .unwrap_or_else(|| Placement::Inside(lexical_join(directory, &Relative(OUTPUTS.into()))));
    match placed {
        Placement::Inside(path) => Some(path),
        Placement::Outside => None,
    }
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

/// The dependency a member's manifest declares, as the packages hold it: from the member's manifest.
fn dependency_of(member: &RepoPath, dependency: DeclaredDependency) -> Dependency {
    Dependency {
        source: member.clone(),
        target: dependency.target,
        origin: dependency.origin,
        binding: dependency.binding,
        package: dependency.package,
        kind: dependency.kind,
        condition: dependency.condition,
        optional: dependency.optional,
        requirement: dependency.requirement,
        checkout: dependency.checkout,
        inheritance: dependency.inheritance,
        replacement: dependency.replacement,
    }
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
        covered.extend(members.iter().cloned());
        for member in &members {
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
            packages.members.push(WorkspacePackageMembership {
                workspace: root.clone(),
                package: member.clone(),
            });
            let shared = Shared {
                directory: &root_directory,
                dependencies: match &root_manifest.workspace {
                    Section::Present(workspace) => &workspace.dependencies,
                    Section::Absent => &[],
                },
                patches: &root_manifest.patches,
            };
            for dependency in declared_of(&directory, &each, shared, &mut looking.platforms) {
                packages
                    .dependencies
                    .push(dependency_of(member, dependency));
            }
        }
        covered.extend([root]);
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

/// Whether a refused manifest leaves which workspace holds it unread: one a workspace above it neither holds nor leaves out
/// is in no workspace, which is known; any other refusal leaves it unread.
#[must_use]
pub(crate) fn refused_membership(refusal: &Refusal) -> Analysis {
    if *refusal.reason.0 == *PackagesError::NotAMember.to_string() {
        Analysis::Analyzed
    } else {
        Analysis::Unanalyzed
    }
}

/// The manifests refused that are among the `files` the run lists, which are in the order of their paths, each with the stage it was
/// refused at and what the reader said: a refusal of the workspace not holding the package, or of a value taken from a workspace the
/// package is not in, is of the validation; any other is of the reading.
#[must_use]
pub(crate) fn diagnoses_in(files: &[languages::FilePath], packages: &Packages) -> Vec<Diagnosed> {
    let validated = [
        PackagesError::NotAMember.to_string(),
        PackagesError::NoWorkspaceToInheritFrom.to_string(),
    ];
    packages
        .refused
        .iter()
        .filter_map(|refusal| {
            let Ok(at) = files.binary_search_by(|each| (*each.0).cmp(&*refusal.manifest.0)) else {
                return None;
            };
            Some(Diagnosed {
                file: languages::FileId(u32::try_from(at).unwrap_or(u32::MAX)),
                stage: if validated.iter().any(|said| **said == *refusal.reason.0) {
                    Stage::Validation
                } else {
                    Stage::Parse
                },
                reason: refusal.reason.clone(),
            })
        })
        .collect()
}

/// Whether the package a dependency is on is held by no workspace that was read: what it declares is missing.
#[must_use]
pub(crate) fn is_unanalyzed(packages: &Packages, dependency: &Dependency) -> Analysis {
    if dependency.target.as_ref().is_none_or(|target| {
        packages
            .members
            .iter()
            .any(|member| &member.package == target)
    }) {
        Analysis::Analyzed
    } else {
        Analysis::Unanalyzed
    }
}

/// The manifests the packages name: the workspaces and their members, the packages that depend and those depended on, and the
/// package of each directory.
#[must_use]
pub(crate) fn named_manifests(packages: &Packages) -> BTreeSet<RepoPath> {
    let members = packages
        .members
        .iter()
        .flat_map(|member| [member.workspace.clone(), member.package.clone()]);
    let dependencies = packages.dependencies.iter().flat_map(|dependency| {
        std::iter::once(dependency.source.clone()).chain(dependency.target.clone())
    });
    let directories = packages
        .roots
        .keys()
        .map(|directory| lexical_join(directory, &Relative(MANIFEST.into())));
    members.chain(dependencies).chain(directories).collect()
}

/// The manifests the packages name that are not among the `files` the run lists, which are in the order of their paths: in the
/// order of their paths, they are numbered after the last of the files.
#[must_use]
pub(crate) fn unlisted_among(files: &[languages::FilePath], packages: &Packages) -> Vec<RepoPath> {
    named_manifests(packages)
        .into_iter()
        .filter(|manifest| {
            files
                .binary_search_by(|each| (*each.0).cmp(&*manifest.0))
                .is_err()
        })
        .collect()
}

/// The number of each manifest the packages name: its place among the `files` the run lists, which are in the order of their paths,
/// or, for a manifest the run does not list, the place it has after them.
#[must_use]
pub(crate) fn numbered(
    files: &[languages::FilePath],
    packages: &Packages,
) -> BTreeMap<RepoPath, languages::FileId> {
    let number = |at: ListIndex| languages::FileId(u32::try_from(at.0).unwrap_or(u32::MAX));
    let listed = named_manifests(packages)
        .into_iter()
        .filter_map(|manifest| {
            match files.binary_search_by(|each| (*each.0).cmp(&*manifest.0)) {
                Ok(at) => Some((manifest, number(ListIndex(at)))),
                // The search answers where the manifest would go: the run does not list it.
                Err(_) => None,
            }
        });
    let unlisted = unlisted_among(files, packages)
        .into_iter()
        .enumerate()
        .map(|(after, manifest)| (manifest, number(ListIndex(files.len() + after))));
    listed.chain(unlisted).collect()
}

/// Why a dependency on a package that no workspace holds leaves the rows partly read.
#[must_use]
pub(crate) fn unanalyzed_cause(dependency: &Dependency) -> Cause {
    Cause::Unanalyzed {
        depender: dependency.source.clone(),
        target: dependency.target.clone().unwrap_or_default(),
    }
}
