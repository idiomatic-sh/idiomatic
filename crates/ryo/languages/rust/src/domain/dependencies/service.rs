//! The `dependencies` service: the dependencies of a package on packages of the repository, with no IO and no ports.

use super::constants::INHERITED;
use super::types::{DeclaredDependency, Shared};
use crate::domain::conditions::types::PlatformCondition;
use crate::domain::paths::service::{join, manifest_in, told};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{Asked, Relative};
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::Platform;
use crate::domain::targets::types::Replacement;
use crate::domain::targets::types::Revision;
use crate::domain::targets::types::Version;
use ryo::languages::{Flag, NameText};

/// The dependencies of the manifest in `directory`, each with the manifest of its package when it names a directory of the repository.
/// A dependency that takes the rest from the
/// workspace names the directory the workspace gives it, from the workspace's own; one that names none names the directory the root
/// manifest's `[patch]` or `[replace]` puts its package in, for its source.
///
/// Not seen: whether the patched package's version meets the requirement, which Cargo checks when it resolves.
#[must_use]
pub(crate) fn declared_of(
    directory: &RepoPath,
    manifest: &Manifest,
    shared: Shared<'_>,
    platforms: &mut Asked<Platform, PlatformCondition>,
) -> Vec<DeclaredDependency> {
    let mut found = Vec::new();
    for each in &manifest.dependencies {
        let (base, path, package, source, origin, requirement, checkout) =
            if each.workspace == Inheritance::Workspace {
                let Some(inherited) = shared
                    .dependencies
                    .iter()
                    .find(|entry| entry.key == each.key)
                    .map(|entry| (shared.directory, entry))
                else {
                    continue;
                };
                (
                    inherited.0,
                    &inherited.1.path,
                    &inherited.1.package,
                    &inherited.1.source,
                    inherited.1.origin,
                    &inherited.1.version,
                    &inherited.1.checkout,
                )
            } else {
                (
                    directory,
                    &each.path,
                    &each.package,
                    &each.source,
                    each.origin,
                    &each.version,
                    &each.checkout,
                )
            };
        let package = match package {
            Section::Present(name) => name.clone(),
            Section::Absent => PackageName(each.key.0.clone()),
        };
        let patching = match path {
            Section::Present(_) => None,
            Section::Absent => shared
                .patches
                .iter()
                .find(|patch| &patch.source == source && patch.package == package),
        };
        let target = match path {
            Section::Present(path) => join(base, &Relative(path.0.clone())),
            Section::Absent => patching.and_then(|patch| match &patch.replacement {
                Replacement::Directory(written) => {
                    join(shared.directory, &Relative(written.0.clone()))
                }
                Replacement::Repository(_) => None,
            }),
        };
        found.push(DeclaredDependency {
            target: target.as_ref().map(manifest_in),
            origin,
            binding: each.key.clone(),
            package,
            kind: each.kind,
            condition: match &each.platform {
                Section::Present(platform) => told(platforms, platform, || {
                    PlatformCondition(platform.0.clone())
                }),
                Section::Absent => PlatformCondition::default(),
            },
            optional: each.optional,
            requirement: requirement.clone(),
            checkout: checkout.clone(),
            inheritance: each.workspace,
            replacement: patching.map_or(Section::Absent, |patch| {
                Section::Present(patch.replacement.clone())
            }),
        });
    }
    found
}

/// The version requirement as the manifest writes it, empty when it writes none.
#[must_use]
pub(crate) fn requirement_text(requirement: &Section<Version>) -> NameText<'_> {
    NameText(match requirement {
        Section::Present(version) => &version.0,
        Section::Absent => <&str>::default(),
    })
}

/// A revision of a git repository as the manifest writes it, empty when it writes none.
#[must_use]
pub(crate) fn revision_text(revision: &Section<Revision>) -> NameText<'_> {
    NameText(match revision {
        Section::Present(revision) => &revision.0,
        Section::Absent => <&str>::default(),
    })
}

/// How the manifest spells taking the requirement from the workspace, empty when it writes its own.
#[must_use]
pub(crate) fn catalog_text(inheritance: Inheritance) -> NameText<'static> {
    NameText(match inheritance {
        Inheritance::Workspace => INHERITED,
        Inheritance::Own => <&str>::default(),
    })
}

/// Where `[patch]` or `[replace]` takes a package from, as written: its directory or the URL of its git repository; empty when
/// none does.
#[must_use]
pub(crate) fn replacement_text(replacement: &Section<Replacement>) -> NameText<'_> {
    NameText(match replacement {
        Section::Present(Replacement::Directory(path)) => &path.0,
        Section::Present(Replacement::Repository(url)) => &url.0,
        Section::Absent => <&str>::default(),
    })
}

/// Whether a Cargo version requirement admits one version only: one of the comparators it joins with commas is `=` with a major, a
/// minor and a patch (`=1.2.3`, `=1.2.3-rc.1`). A bare `1.2.3` is Cargo's caret, a range; `=1.2` admits every `1.2.x`.
#[must_use]
pub(crate) fn exactness(requirement: &Section<Version>) -> Flag {
    // A whole semantic version, its pre-release and build parts aside, is three dot-separated numbers.
    let pinned = requirement_text(requirement)
        .0
        .split(',')
        .any(|comparator| {
            comparator.trim().strip_prefix('=').is_some_and(|version| {
                let version = version.trim();
                let core = version.split_once('+').map_or(version, |(core, _)| core);
                let core = core.split_once('-').map_or(core, |(core, _)| core);
                let parts: Vec<&str> = core.split('.').collect();
                parts.len() == 3
                    && parts.iter().all(|part| {
                        !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
                    })
            })
        });
    if pinned { Flag::Raised } else { Flag::Lowered }
}
