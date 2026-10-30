//! The packages of a repository as extents of directories: which package holds a file, and which files belong to a package at all.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::MANIFEST;
use super::types::{Index, ListIndex, Package, PackageDirectory, Reach};
use crate::domain::paths::service::{directory_of, join, within};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::{Lying, Relative};
use ryo::languages::{Count, FilePath};

/// The packages among `files`: the manifests, each with the roots Cargo declares for it, `roots` by package directory.
#[must_use]
pub(crate) fn index(
    files: &BTreeSet<RepoPath>,
    roots: &BTreeMap<RepoPath, BTreeSet<RepoPath>>,
) -> Index {
    let mut found = Index::default();
    for manifest in files
        .iter()
        .filter(|path| path.rsplit('/').next() == Some(MANIFEST))
    {
        let package = directory_of(manifest);
        let declared: Vec<RepoPath> = roots
            .get(&package)
            .into_iter()
            .flatten()
            .filter_map(|root| join(&RepoPath(Box::default()), &Relative(root.0.clone())))
            .filter(|root| files.contains(root))
            .collect();
        let at = ListIndex(found.directories.len());
        found.extents.push(PackageDirectory {
            directory: package.clone(),
            package: at,
            reach: Reach::Own,
        });
        found.extents.extend(
            declared
                .iter()
                .filter(|root| within(root, &package) == Lying::Apart)
                .map(|root| PackageDirectory {
                    directory: directory_of(root),
                    package: at,
                    reach: Reach::Target,
                }),
        );
        found.directories.push(Package {
            directory: package,
            roots: declared,
        });
    }
    found
}

/// Whether `file` is in the directory `package`.
/// How many directories deep a path is.
fn depth(path: &RepoPath) -> Count {
    Count(path.split('/').filter(|part| !part.is_empty()).count())
}

/// The package `file` belongs to: the innermost one whose directory holds it. A target a package declares outside its own
/// directory brings that target's directory into the package, as deep as it lies; a package's own directory wins a tie.
#[must_use]
pub(crate) fn package_of<'a>(index: &'a Index, file: &RepoPath) -> Option<&'a RepoPath> {
    index
        .extents
        .iter()
        .filter(|extent| within(file, &extent.directory) == Lying::Within)
        .max_by_key(|extent| (depth(&extent.directory), extent.reach))
        .and_then(|extent| index.directories.get(extent.package.0))
        .map(|package| &package.directory)
}

/// The files that belong to some package: all but those under a directory a workspace writes its outputs to.
#[must_use]
pub(crate) fn kept_files(files: &[FilePath], outputs: &[RepoPath]) -> BTreeSet<RepoPath> {
    files
        .iter()
        .map(|file| RepoPath(file.0.clone()))
        .filter(|file| {
            !outputs
                .iter()
                .any(|each| !each.is_empty() && within(file, each) == Lying::Within)
        })
        .collect()
}
