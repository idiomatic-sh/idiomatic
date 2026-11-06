//! Pure repository package edges and nearest-package file ownership.

use super::constants::{CURRENT, EMPTY, PACKAGE, PARENT, SEPARATOR};
use super::types::{
    Containment, Declared, Dependency, Directory, Fact, ManifestKind, ManifestPath, Package,
    PackageName, Packages, Projection, RepositoryPath, Requirement, Version,
};
use ryo::languages;

/// The directory a repository path is in; empty at the repository root.
pub(crate) fn directory(path: RepositoryPath<'_>) -> Directory<'_> {
    Directory(
        path.0
            .rsplit_once(SEPARATOR)
            .map_or(EMPTY, |(directory, _)| directory),
    )
}

/// The path `path` names when read from `base`, with `.` and `..` resolved; an absolute path stays as written.
pub(crate) fn relative(base: Directory<'_>, path: RepositoryPath<'_>) -> ManifestPath {
    if path.0.starts_with(SEPARATOR) {
        return ManifestPath(path.0.into());
    }
    let mut parts = base
        .0
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    for part in path.0.split(SEPARATOR) {
        match part {
            EMPTY | CURRENT => {}
            PARENT => {
                if parts.last().is_some_and(|last| *last != PARENT) {
                    let _: Option<&str> = parts.pop();
                } else {
                    parts.push(PARENT);
                }
            }
            part => parts.push(part),
        }
    }
    ManifestPath(parts.join(&SEPARATOR.to_string()).into())
}

/// Whether the manifest at `path` is a package manifest or a project configuration.
pub(crate) fn is_package(path: RepositoryPath<'_>) -> ManifestKind {
    if path.0.rsplit(SEPARATOR).next() == Some(PACKAGE) {
        ManifestKind::Package
    } else {
        ManifestKind::Configuration
    }
}

/// Whether `path` is inside `directory`; every path is inside the repository root.
pub(crate) fn within(directory: Directory<'_>, path: RepositoryPath<'_>) -> Containment {
    if directory.0.is_empty()
        || path
            .0
            .strip_prefix(directory.0)
            .is_some_and(|rest| rest.starts_with(SEPARATOR))
    {
        Containment::Inside
    } else {
        Containment::Outside
    }
}

/// The package a file belongs to: the nearest package manifest in a directory above it.
pub(crate) fn owner<'a>(packages: &'a Packages, path: RepositoryPath<'_>) -> Option<&'a Package> {
    packages
        .entries
        .iter()
        .filter(|package| {
            is_package(RepositoryPath(&package.manifest.0)) == ManifestKind::Package
                && within(directory(RepositoryPath(&package.manifest.0)), path)
                    == Containment::Inside
        })
        .max_by_key(|package| directory(RepositoryPath(&package.manifest.0)).0.len())
}

/// The file id of the manifest at `path`, by its place in the files; none for a path the run did not list.
pub(crate) fn file(
    files: &[languages::FilePath],
    path: &ManifestPath,
) -> Option<languages::FileId> {
    files
        .iter()
        .position(|file| file.0 == path.0)
        .and_then(|place| {
            // a place past 2^32 files has no file number: the file is not one the rows can name
            let Ok(number) = u32::try_from(place) else {
                return None;
            };
            Some(number)
        })
        .map(languages::FileId)
}

/// The dependencies of the repository's packages, on one another and on packages it does not hold, and why a project reference into a package with no name gives no row: the
/// row needs the name of the package depended on, and the provider invents none.
pub(crate) fn dependencies(
    files: &[languages::FilePath],
    packages: &Packages,
) -> Projection<Dependency> {
    let mut rows = Vec::new();
    let mut refused = packages.refused.clone();
    for package in &packages.entries {
        if is_package(RepositoryPath(&package.manifest.0)) == ManifestKind::Package {
            for requirement in package
                .declared
                .requirements
                .iter()
                .filter(|requirement| requirement.catalog.is_none())
            {
                let Some(source) = file(files, &package.manifest) else {
                    continue;
                };
                let replacement = replacement_of(packages, package, &requirement.package);
                let found = packages.entries.iter().find(|candidate| {
                    is_package(RepositoryPath(&candidate.manifest.0)) == ManifestKind::Package
                        && requirement.target.as_ref().map_or_else(
                            || candidate.declared.name.as_ref() == Some(&requirement.package),
                            |path| path == &candidate.manifest,
                        )
                });
                // A package the repository does not hold is a row too, with no manifest to point at.
                let Some(target) = found else {
                    rows.push(Dependency {
                        source,
                        target: languages::NO_FILE,
                        requirement: requirement.clone(),
                        replacement,
                    });
                    continue;
                };
                if let Some(target_id) = file(files, &target.manifest) {
                    let mut requirement = requirement.clone();
                    if let Some(name) = &target.declared.name {
                        requirement.package = name.clone();
                    }
                    rows.push(Dependency {
                        source,
                        target: target_id,
                        requirement,
                        replacement,
                    });
                }
            }
        }
        for reference in &package.declared.references {
            if let (Some(source), Some(target)) = (
                owner(packages, RepositoryPath(&package.manifest.0)),
                owner(packages, RepositoryPath(&reference.0)),
            ) && source.manifest != target.manifest
                && let (Some(source_id), Some(target_id)) =
                    (file(files, &source.manifest), file(files, &target.manifest))
            {
                let Some(name) = &target.declared.name else {
                    refused.push(languages::Refused {
                        path: languages::FilePath(package.manifest.0.clone()),
                        reason: languages::ToolMessage(
                            format!(
                                "the project reference `{}` is into `{}`, a package with no name: the dependency on it has no package name to be a row",
                                reference.0, target.manifest.0
                            )
                            .into(),
                        ),
                    });
                    continue;
                };
                let row = Dependency {
                    source: source_id,
                    target: target_id,
                    requirement: Requirement {
                        binding: name.clone(),
                        package: name.clone(),
                        target: None,
                        kind: languages::DependencyKind::Normal,
                        optional: languages::Optionality::Required,
                        origin: languages::DependencyOrigin::Path,
                        catalog: None,
                        spec: Version(Box::default()),
                        written_catalog: Version(Box::default()),
                    },
                    replacement: Version(Box::default()),
                };
                if !rows.contains(&row) {
                    rows.push(row);
                }
            }
        }
    }
    Projection { rows, refused }
}

/// The spec that takes the place of a dependency of `package` on `name`: the override the root of its install writes for that name — a
/// workspace manifest that holds it as a member, or its own manifest when no workspace holds it; empty when none does. npm, Yarn and
/// pnpm read overrides from the root alone: a member's own are not applied.
fn replacement_of(packages: &Packages, package: &Package, name: &PackageName) -> Version {
    let member = packages
        .entries
        .iter()
        .any(|each| each.declared.members.contains(&package.manifest));
    packages
        .entries
        .iter()
        .filter(|each| {
            (each.manifest == package.manifest && !member)
                || each.declared.members.contains(&package.manifest)
        })
        .find_map(|each| each.declared.overrides.get(name))
        .cloned()
        .unwrap_or_else(|| Version(Box::default()))
}

/// The `workspace-member` rows: each member a workspace manifest declares that the repository holds.
pub(crate) fn members(
    files: &[languages::FilePath],
    packages: &Packages,
) -> Vec<languages::WorkspaceMemberRow> {
    packages
        .entries
        .iter()
        .flat_map(|package| {
            package.declared.members.iter().filter_map(|member| {
                if !packages
                    .entries
                    .iter()
                    .any(|entry| &entry.manifest == member)
                {
                    return None;
                }
                Some(languages::WorkspaceMemberRow {
                    workspace: file(files, &package.manifest)?,
                    package: file(files, member)?,
                })
            })
        })
        .collect()
}

/// Two declarations of one package manifest, as two adapters read it, as one.
pub(crate) fn combined(mut left: Declared, right: Declared) -> Declared {
    left.name = left.name.or(right.name);
    left.members.extend(right.members);
    left.members.sort();
    left.members.dedup();
    left.requirements.extend(right.requirements);
    left.references.extend(right.references);
    left.catalogs.default.0.extend(right.catalogs.default.0);
    left.catalogs.named.extend(right.catalogs.named);
    left.base_url = left.base_url.or(right.base_url);
    left.aliases.extend(right.aliases);
    left.extends.extend(right.extends);
    left.overrides.extend(right.overrides);
    left
}

/// The packages the facts declare: a manifest read by several adapters is one package; facts about a manifest nothing else read declare nothing.
pub(crate) fn assembled(facts: Vec<Fact>) -> Vec<Package> {
    let (own, foreign): (Vec<_>, Vec<_>) = facts
        .into_iter()
        .partition(|fact| fact.manifest == fact.about);
    let mut packages: Vec<Package> = Vec::new();
    for fact in own.into_iter().chain(foreign) {
        match packages
            .iter_mut()
            .find(|package| package.manifest == fact.about)
        {
            Some(package) => {
                package.declared = combined(std::mem::take(&mut package.declared), fact.declared);
            }
            None if fact.manifest == fact.about => packages.push(Package {
                manifest: fact.about,
                declared: fact.declared,
            }),
            // A fact about a manifest that declares no package of its own is about nothing the repository holds.
            None => {}
        }
    }
    packages
}

/// The `file-package` rows: each file and the package manifest nearest above it.
pub(crate) fn ownership(
    files: &[languages::FilePath],
    packages: &Packages,
) -> Vec<languages::FilePackageRow> {
    files
        .iter()
        .enumerate()
        .filter_map(|(index, path)| {
            let package = owner(packages, RepositoryPath(&path.0))?;
            // a place past 2^32 files has no file number: the file is not one the rows can name
            let Ok(number) = u32::try_from(index) else {
                return None;
            };
            Some(languages::FilePackageRow {
                path: languages::FileId(number),
                package: file(files, &package.manifest)?,
            })
        })
        .collect()
}

/// The symbol a package name is written as in a row, interned from the name alone.
pub(crate) fn named(symbols: &mut languages::Symbols, name: &PackageName) -> languages::Symbol {
    languages::intern_derived(
        symbols,
        languages::ByteSlice(&[]),
        languages::NameText(&name.0),
    )
}

/// The rows of a relation over the packages, with the manifests refused while they were read.
pub(crate) fn projected<Row>(packages: &Packages, rows: Vec<Row>) -> Projection<Row> {
    Projection {
        rows,
        refused: packages.refused.clone(),
    }
}
