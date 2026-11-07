// ignore: idiomatic.arch/file-length - `packages` is one concept, the layout of a package and its lock: where each kind and release sits, how versions compare, what the lock pins and what a package holds; its names are in constants.rs and its values in types.rs, and every function left is a pure read over the same release and lock types that the use cases call by this one path.
//! The `packages` service: free functions named by verb over packages, releases and the lock, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{
    CHANGELOG_HEADING, PACKAGE_FILE, RULE_FILE, RULE_SKELETON, RULE_WORD, RULES, SPEC_FILE,
    SPEC_SKELETON, SPEC_WORD, SPECS, TEMPLATE_FILE, TEMPLATE_SKELETON, TEMPLATE_WORD, TEMPLATES,
};
use super::types::{
    DefinitionKey, DefinitionKind, DefinitionName, Lock, LockStatus, LockedPackage, LockedRelease,
    Ownership, PackageName, PlacedDefinition, Release, ResolvedPackage, VersionNumber, VersionPart,
    VersionText,
};
use super::types::{KindDirectory, KindFile, KindWord, ReleaseDirectory, ReleaseFile, ReleaseId};

/// The directory of a kind's definitions within a package.
#[must_use]
pub fn kind_directory(kind: DefinitionKind) -> KindDirectory {
    KindDirectory(match kind {
        DefinitionKind::Rule => RULES,
        DefinitionKind::Spec => SPECS,
        DefinitionKind::Template => TEMPLATES,
    })
}

/// The definition file of a kind.
#[must_use]
pub fn kind_file(kind: DefinitionKind) -> KindFile {
    KindFile(match kind {
        DefinitionKind::Rule => RULE_FILE,
        DefinitionKind::Spec => SPEC_FILE,
        DefinitionKind::Template => TEMPLATE_FILE,
    })
}

/// The definition file `create` writes for a kind: the least it must hold, to be filled.
#[must_use]
pub fn kind_skeleton(kind: DefinitionKind) -> crate::domain::files::types::FileText {
    crate::domain::files::types::FileText(
        match kind {
            DefinitionKind::Rule => RULE_SKELETON,
            DefinitionKind::Spec => SPEC_SKELETON,
            DefinitionKind::Template => TEMPLATE_SKELETON,
        }
        .into(),
    )
}

/// What a kind reads as: `rule`, `spec`, `template`.
#[must_use]
pub fn kind_word(kind: DefinitionKind) -> KindWord<'static> {
    KindWord(match kind {
        DefinitionKind::Rule => RULE_WORD,
        DefinitionKind::Spec => SPEC_WORD,
        DefinitionKind::Template => TEMPLATE_WORD,
    })
}

/// The kind a word names, as the lock and `create` write it.
#[must_use]
pub fn kind_of(word: KindWord<'_>) -> Option<DefinitionKind> {
    [
        DefinitionKind::Rule,
        DefinitionKind::Spec,
        DefinitionKind::Template,
    ]
    .into_iter()
    .find(|kind| kind_word(*kind).0 == word.0)
}

/// A release's directory within its package: `<kind>s/<name>/<version>`, or
/// `specs/<spec>/<version>/<kind>s/<name>` for an owned definition.
#[must_use]
pub fn release_directory(release: &Release) -> ReleaseDirectory {
    ReleaseDirectory(match &release.ownership {
        Ownership::Shared => format!(
            "{}/{}/{}",
            kind_directory(release.kind),
            release.name.0,
            release.version.0
        ),
        Ownership::Owned(spec) => format!(
            "{SPECS}/{}/{}/{}/{}",
            spec.0,
            release.version.0,
            kind_directory(release.kind),
            release.name.0
        ),
    })
}

/// A release's definition file within its package.
#[must_use]
pub fn release_file(release: &Release) -> ReleaseFile {
    ReleaseFile(format!(
        "{}/{}",
        release_directory(release),
        kind_file(release.kind)
    ))
}

/// The id a release's findings carry: `package/name`, or `package/spec/name` for an owned definition.
#[must_use]
pub fn release_id(release: &Release) -> ReleaseId {
    ReleaseId(match &release.ownership {
        Ownership::Shared => format!("{}/{}", release.package.0, release.name.0),
        Ownership::Owned(spec) => format!("{}/{}/{}", release.package.0, spec.0, release.name.0),
    })
}

/// A version's three numbers; none when it is not `major.minor.patch`.
#[must_use]
pub fn version_number(version: &VersionText) -> Option<VersionNumber> {
    let parts = version
        .0
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Vec<_>>();
    let [Ok(major), Ok(minor), Ok(patch)] = parts.as_slice() else {
        return None;
    };
    Some(VersionNumber {
        major: VersionPart(*major),
        minor: VersionPart(*minor),
        patch: VersionPart(*patch),
    })
}

/// The highest of the `offered` versions, the release's own included.
#[must_use]
pub fn latest<'a>(
    own: &VersionText,
    offered: impl IntoIterator<Item = &'a VersionText>,
) -> VersionText {
    let mut best = (version_number(own).unwrap_or_default(), own.clone());
    for candidate in offered {
        let candidate = candidate.clone();
        if let Some(number) = version_number(&candidate)
            && number > best.0
        {
            best = (number, candidate);
        }
    }
    best.1
}

/// The definition `release` is a version of: its package, kind and name.
fn definition_of(release: &Release) -> PlacedDefinition {
    PlacedDefinition {
        kind: release.kind,
        package: release.package.clone(),
        name: release.name.clone(),
    }
}

/// The versions on `disk` no lock reads and that are not their definition's latest: each is there for no one, so a
/// package keeps only what a workspace pins and what it offers next. A definition is told by its package, kind and name.
#[must_use]
pub fn unread<'a>(disk: &'a [Release], locks: &[Lock]) -> Vec<&'a Release> {
    let pinned = locks
        .iter()
        .flat_map(|lock| &lock.packages)
        .flat_map(|package| {
            package
                .releases
                .iter()
                .map(move |release| (&package.name, release.kind, &release.name, &release.version))
        })
        .collect::<BTreeSet<_>>();
    let mut newest: BTreeMap<PlacedDefinition, VersionNumber> = BTreeMap::new();
    for release in disk {
        if let Some(number) = version_number(&release.version) {
            let best = newest.entry(definition_of(release)).or_insert(number);
            if number > *best {
                *best = number;
            }
        }
    }
    disk.iter()
        .filter(|release| {
            let read = pinned.contains(&(
                &release.package,
                release.kind,
                &release.name,
                &release.version,
            ));
            let latest =
                version_number(&release.version) == newest.get(&definition_of(release)).copied();
            !read && !latest
        })
        .collect()
}

/// The lock `packages` and the `releases` used make: each package with the shared releases of it the workspace uses.
#[must_use]
pub fn lock<'a>(
    direct: &[PackageName],
    packages: &BTreeMap<PackageName, ResolvedPackage>,
    releases: impl IntoIterator<Item = &'a Release>,
) -> Lock {
    let mut used: BTreeMap<&PackageName, BTreeSet<LockedRelease>> = BTreeMap::new();
    for release in releases {
        if release.ownership != Ownership::Shared {
            continue;
        }
        let _: bool = used
            .entry(&release.package)
            .or_default()
            .insert(LockedRelease {
                kind: release.kind,
                name: release.name.clone(),
                version: release.version.clone(),
            });
    }
    let mut dependencies = direct.to_vec();
    dependencies.sort();
    dependencies.dedup();
    Lock {
        dependencies,
        packages: packages
            .iter()
            .map(|(name, package)| LockedPackage {
                name: name.clone(),
                source: package.pin.clone(),
                dependencies: {
                    let mut names = package
                        .manifest
                        .dependencies
                        .iter()
                        .map(|dependency| dependency.package.clone())
                        .collect::<Vec<_>>();
                    names.sort();
                    names
                },
                releases: used
                    .get(name)
                    .map(|locked| locked.iter().cloned().collect())
                    .unwrap_or_default(),
            })
            .collect(),
    }
}

/// The version `lock` pins for the definition `release` names, the highest when it records more than one.
#[must_use]
pub fn locked_version(lock: &Lock, release: &Release) -> Option<VersionText> {
    lock.packages
        .iter()
        .filter(|package| package.name == release.package)
        .flat_map(|package| package.releases.iter())
        .filter(|locked| locked.kind == release.kind && locked.name == release.name)
        .map(|locked| locked.version.clone())
        .max_by(|left, right| version_number(left).cmp(&version_number(right)))
}

/// How the lock on disk, when there is one, compares with `made`.
#[must_use]
pub fn lock_status(prior: Option<&Lock>, made: &Lock) -> LockStatus {
    match prior {
        None => LockStatus::Missing,
        Some(prior) if prior == made => LockStatus::Current,
        Some(_) => LockStatus::Stale,
    }
}

/// The version sections crossed by a move, as written in the asset's changelog. Version headings use
/// `## major.minor.patch`, the package assets' convention; unrelated and out-of-range sections are omitted.
#[must_use]
pub fn changes(
    text: &super::types::ChangeLog,
    from: &VersionText,
    to: &VersionText,
) -> super::types::ChangeLog {
    let Some(from) = version_number(from) else {
        return super::types::ChangeLog(Box::default());
    };
    let Some(to) = version_number(to) else {
        return super::types::ChangeLog(Box::default());
    };
    let (low, high) = if from < to { (from, to) } else { (to, from) };
    let mut included = false;
    let mut selected = String::new();
    for line in text.0.split_inclusive('\n') {
        if let Some(heading) = line.strip_prefix(CHANGELOG_HEADING) {
            included = version_number(&VersionText(heading.trim().into()))
                .is_some_and(|version| version > low && version <= high);
        }
        if included {
            selected.push_str(line);
        }
    }
    super::types::ChangeLog(selected.trim_end().into())
}

/// The package lookup root for a directory of the repository: the repository's own root, with no trailing separator, for
/// its top directory.
#[must_use]
pub fn root_at(
    repository: &crate::domain::files::types::MachinePath,
    directory: &crate::domain::paths::types::RepoPath,
) -> super::types::PackageRoot {
    if directory.0.is_empty() {
        return super::types::PackageRoot(repository.0.clone());
    }
    super::types::PackageRoot(
        std::path::Path::new(&*repository.0)
            .join(&*directory.0)
            .display()
            .to_string()
            .into(),
    )
}

/// An explicitly offered semantic version, or the latest offered version without downgrading the current pin.
#[must_use]
pub fn offered_version(
    current: &VersionText,
    requested: Option<&VersionText>,
    offered: &[VersionText],
) -> Option<VersionText> {
    match requested {
        Some(version) if version_number(version).is_some() && offered.contains(version) => {
            Some(version.clone())
        }
        Some(_) => None,
        None => Some(latest(current, offered.iter())),
    }
}

/// The files of a package, relative to its directory, that belong to nothing it holds: a package holds its
/// `package.toml`, the packages nested in it (each a directory with a `package.toml` of its own), and
/// its definitions' versions, each a `<kind>/<name>/<version>/` directory and the files within it.
#[must_use]
pub fn strays(files: &[ReleaseFile]) -> Vec<&ReleaseFile> {
    let nested = files
        .iter()
        .filter_map(|file| file.0.strip_suffix(PACKAGE_FILE))
        .filter(|directory| !directory.is_empty())
        .collect::<Vec<_>>();
    files
        .iter()
        .filter(|file| file.0 != PACKAGE_FILE)
        .filter(|file| !nested.iter().any(|directory| file.0.starts_with(directory)))
        .filter(|file| {
            let mut parts = file.0.split('/');
            let (kind, _name, version, rest) =
                (parts.next(), parts.next(), parts.next(), parts.next());
            !(kind.is_some_and(|kind| [RULES, SPECS, TEMPLATES].contains(&kind))
                && version
                    .is_some_and(|version| version_number(&VersionText(version.into())).is_some())
                && rest.is_some())
        })
        .collect()
}

/// The versions the files of a package hold, by kind and definition, each definition's as text in order: every
/// `<kind>/<name>/<version>/` directory a file of the package is in. The layout is fixed, so the files say it all and
/// no directory is read for it.
#[must_use]
pub fn held_versions(files: &[ReleaseFile]) -> BTreeMap<DefinitionKey, Vec<VersionText>> {
    let mut held = BTreeMap::new();
    for file in files {
        let mut parts = file.0.splitn(4, '/');
        let (Some(kind), Some(name), Some(version), Some(_)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Some(kind) = [
            DefinitionKind::Rule,
            DefinitionKind::Spec,
            DefinitionKind::Template,
        ]
        .into_iter()
        .find(|each| kind_directory(*each).0 == kind) else {
            continue;
        };
        let _: bool = held
            .entry(DefinitionKey {
                kind,
                name: DefinitionName(name.into()),
            })
            .or_insert_with(BTreeSet::new)
            .insert(VersionText(version.into()));
    }
    held.into_iter()
        .map(|(definition, versions)| (definition, versions.into_iter().collect()))
        .collect()
}

/// The rules and then the templates the spec version directory `scope` owns, each by name: every
/// `<scope>/<rules|templates>/<name>/` directory a file of the package is in.
#[must_use]
pub fn owned_assets(files: &[ReleaseFile], scope: &ReleaseDirectory) -> Vec<DefinitionKey> {
    let mut owned = Vec::new();
    for kind in [DefinitionKind::Rule, DefinitionKind::Template] {
        let prefix = format!("{}/{}/", scope.0, kind_directory(kind).0);
        let names = files
            .iter()
            .filter_map(|file| file.0.strip_prefix(&*prefix))
            .filter_map(|rest| rest.split_once('/').map(|(name, _)| name))
            .collect::<BTreeSet<_>>();
        owned.extend(names.into_iter().map(|name| DefinitionKey {
            kind,
            name: DefinitionName(name.into()),
        }));
    }
    owned
}
