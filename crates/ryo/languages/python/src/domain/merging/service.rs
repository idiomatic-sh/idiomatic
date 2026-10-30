//! The `merging` service: free functions named by verb that make the packages of a repository from what each manifest declared, with no IO
//! and no ports.

use ryo::languages;

use crate::domain::packages::service::directory_of;
use crate::domain::packages::types::{Declared, Naming, Package};

/// The packages of the manifests: the manifests of one directory are one package, whatever tools wrote them. Its manifest is the first
/// that names a package, or else the first by path; its name is the first any of them gives; its members, requirements and redirects are
/// all of theirs, the manifest's own first and then those of the others in the order of their paths, each once. A directory that names
/// no package and holds no workspace and needs nothing but to build is not a package.
#[must_use]
pub(crate) fn merged(mut found: Vec<Package>) -> Vec<Package> {
    found.sort_by(|left, right| left.manifest.cmp(&right.manifest));
    let mut groups: Vec<Vec<Package>> = Vec::new();
    for package in found {
        let directory = directory_of(&package.manifest).0.to_owned();
        match groups.iter_mut().find(|group| {
            group
                .first()
                .is_some_and(|first| directory_of(&first.manifest).0 == directory)
        }) {
            Some(group) => group.push(package),
            None => groups.push(vec![package]),
        }
    }
    let mut packages: Vec<Package> = groups.into_iter().filter_map(united).collect();
    packages.retain(|package| {
        package.declared.name != Naming::Unnamed
            || !package.declared.members.is_empty()
            || package
                .declared
                .requirements
                .iter()
                .any(|requirement| requirement.kind != languages::DependencyKind::Build)
    });
    packages.sort_by(|left, right| left.manifest.cmp(&right.manifest));
    packages
}

/// The one package the manifests of a directory declare together, `group` being in the order of their paths; none for a group with no
/// manifest.
fn united(mut group: Vec<Package>) -> Option<Package> {
    let first = group
        .iter()
        .find(|package| package.declared.name != Naming::Unnamed)
        .or_else(|| group.first())?
        .manifest
        .clone();
    group.sort_by_key(|package| package.manifest != first);
    let mut declared = Declared {
        name: Naming::Unnamed,
        members: Vec::new(),
        excludes: Vec::new(),
        requirements: Vec::new(),
        sources: Vec::new(),
        overrides: Vec::new(),
    };
    for package in group {
        if declared.name == Naming::Unnamed {
            declared.name = package.declared.name;
        }
        extend_once(&mut declared.members, package.declared.members);
        extend_once(&mut declared.excludes, package.declared.excludes);
        extend_once(&mut declared.requirements, package.declared.requirements);
        extend_once(&mut declared.sources, package.declared.sources);
        extend_once(&mut declared.overrides, package.declared.overrides);
    }
    Some(Package {
        manifest: first,
        declared,
    })
}

/// `more` after `all`, each item not already there.
fn extend_once<Item>(all: &mut Vec<Item>, more: Vec<Item>)
where
    Item: PartialEq,
{
    for item in more {
        if !all.contains(&item) {
            all.push(item);
        }
    }
}
