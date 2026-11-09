//! The `targets` service: the source file of every target of a package, with no IO and no ports. What a directory holds is what the
//! pass has been told; what it has not been told, it wants.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{
    BENCHES, BIN, BUILD, CRATE_SEPARATOR, EXAMPLES, EXTENSION, HIDDEN, LIBRARY, MAIN,
    PACKAGE_SEPARATOR, SOURCE, TESTS,
};
use crate::domain::paths::service::{join, told, written};
use crate::domain::paths::types::Entry;
use crate::domain::paths::types::EntryKind;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{Asked, Relative};
use crate::domain::targets::types::Build;
use crate::domain::targets::types::Discovery;
use crate::domain::targets::types::LibrarySection;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::PackageSection;
use crate::domain::targets::types::Presence;
use crate::domain::targets::types::TargetName;
use crate::domain::targets::types::TargetSection;

/// What the pass has been told of what directories hold.
type Entries<'a> = &'a mut Asked<RepoPath, Vec<Entry>>;

/// Whether the pass was told that `source` holds the file `file`.
fn in_source(
    entries: &mut Asked<RepoPath, Vec<Entry>>,
    source: &RepoPath,
    file: &EntryName,
) -> Presence {
    if told(entries, source, Vec::new)
        .iter()
        .any(|each| each.name == *file && each.kind == EntryKind::File)
    {
        Presence::Present
    } else {
        Presence::Missing
    }
}

/// The root file of the library of a package, when it has one: where `[lib]` puts it, or `src/lib.rs` when Cargo finds it there.
pub(crate) fn library_of(
    directory: &RepoPath,
    manifest: &Manifest,
    package: &PackageSection,
    entries: Entries<'_>,
) -> Option<RepoPath> {
    let source = join(directory, &Relative(SOURCE.into())).unwrap_or_default();
    match &manifest.lib {
        Section::Present(lib) => Some(match &lib.path {
            Section::Present(path) => written(directory, &Relative(path.0.clone())),
            Section::Absent => written(&source, &Relative(LIBRARY.into())),
        }),
        Section::Absent => (package.autolib == Discovery::Automatic
            && in_source(entries, &source, &EntryName(LIBRARY.into())) == Presence::Present)
            .then(|| written(&source, &Relative(LIBRARY.into()))),
    }
}

/// The name the code of a dependent writes for the library of a package it does not rename: `[lib] name`, or else the package's
/// name with each `-` as `_`.
#[must_use]
pub(crate) fn library_name(manifest: &Manifest, package: &PackageSection) -> TargetName {
    match &manifest.lib {
        Section::Present(LibrarySection {
            name: Section::Present(name),
            ..
        }) => name.clone(),
        _ => crate_name(&package.name),
    }
}

/// The name a crate's code writes for the package `name`, or for a dependency a manifest binds to `name`: its `-` written `_`, as
/// Cargo passes it to the compiler.
#[must_use]
pub(crate) fn crate_name(name: &PackageName) -> TargetName {
    TargetName(name.0.replace(PACKAGE_SEPARATOR, CRATE_SEPARATOR).into())
}

/// The targets Cargo finds by their place in a directory of the package: `<sub>/*.rs` and `<sub>/*/main.rs`, by name.
fn inferred(
    directory: &RepoPath,
    sub: &[Relative],
    entries: &mut Asked<RepoPath, Vec<Entry>>,
) -> BTreeMap<TargetName, RepoPath> {
    let mut folder = directory.clone();
    for part in sub {
        let Some(next) = join(&folder, part) else {
            return BTreeMap::new();
        };
        folder = next;
    }
    told(entries, &folder, Vec::new)
        .into_iter()
        .filter(|entry| !entry.name.starts_with(HIDDEN))
        .filter_map(|Entry { name, kind }| {
            if kind == EntryKind::Directory {
                let inside = written(&folder, &Relative(name.0.clone()));
                told(entries, &inside, Vec::new)
                    .iter()
                    .any(|each| &*each.name == MAIN && each.kind == EntryKind::File)
                    .then(|| {
                        (
                            TargetName(name.0.clone()),
                            written(&inside, &Relative(MAIN.into())),
                        )
                    })
            } else {
                name.strip_suffix(EXTENSION).map(|stem| {
                    (
                        TargetName(stem.into()),
                        written(&folder, &Relative(name.0.clone())),
                    )
                })
            }
        })
        .collect()
}

/// The source files of the targets of one kind: the ones declared, then the ones found, unless the package leaves them out.
fn of_kind(
    directory: &RepoPath,
    declared: &[TargetSection],
    found: &BTreeMap<TargetName, RepoPath>,
    default_name: &PackageName,
    discovery: Discovery,
) -> BTreeSet<RepoPath> {
    let named: BTreeSet<TargetName> = declared
        .iter()
        .map(|each| target_name(each, default_name))
        .collect();
    let mut paths: BTreeSet<RepoPath> = declared
        .iter()
        .filter_map(|each| match &each.path {
            Section::Present(path) => Some(written(directory, &Relative(path.0.clone()))),
            Section::Absent => found.get(&target_name(each, default_name)).cloned(),
        })
        .collect();
    if discovery == Discovery::Automatic {
        paths.extend(
            found
                .iter()
                .filter(|(name, _)| !named.contains(*name))
                .map(|(_, path)| path.clone()),
        );
    }
    paths
}

/// The name a declared target goes by: its own, or the package's when it gives none.
fn target_name(each: &TargetSection, default_name: &PackageName) -> TargetName {
    match &each.name {
        Section::Present(name) => name.clone(),
        Section::Absent => TargetName(default_name.0.clone()),
    }
}

/// The source file of every target of the package in `directory`.
#[must_use]
pub(crate) fn targets_of(
    directory: &RepoPath,
    manifest: &Manifest,
    package: &PackageSection,
    entries: Entries<'_>,
) -> BTreeSet<RepoPath> {
    let source = join(directory, &Relative(SOURCE.into())).unwrap_or_default();
    let mut targets = BTreeSet::new();
    // The library.
    targets.extend(library_of(directory, manifest, package, &mut *entries));
    // The binaries: `src/main.rs` is the one named for the package.
    let mut bins = inferred(
        directory,
        &[Relative(SOURCE.into()), Relative(BIN.into())],
        &mut *entries,
    );
    if in_source(entries, &source, &EntryName(MAIN.into())) == Presence::Present {
        bins.extend([(
            TargetName(package.name.0.clone()),
            written(&source, &Relative(MAIN.into())),
        )]);
    }
    targets.extend(of_kind(
        directory,
        &manifest.bins,
        &bins,
        &package.name,
        package.autobins,
    ));
    targets.extend(of_kind(
        directory,
        &manifest.examples,
        &inferred(directory, &[Relative(EXAMPLES.into())], &mut *entries),
        &package.name,
        package.autoexamples,
    ));
    targets.extend(of_kind(
        directory,
        &manifest.tests,
        &inferred(directory, &[Relative(TESTS.into())], &mut *entries),
        &package.name,
        package.autotests,
    ));
    targets.extend(of_kind(
        directory,
        &manifest.benches,
        &inferred(directory, &[Relative(BENCHES.into())], &mut *entries),
        &package.name,
        package.autobenches,
    ));
    // The build script.
    match &package.build {
        Build::Disabled => {}
        Build::At(path) => {
            targets.extend([written(directory, &Relative(path.0.clone()))]);
        }
        Build::Found => {
            if told(entries, directory, Vec::new)
                .iter()
                .any(|each| &*each.name == BUILD && each.kind == EntryKind::File)
            {
                targets.extend([written(directory, &Relative(BUILD.into()))]);
            }
        }
    }
    targets
}
