//! Dependency specs, catalogs, and `catalog:` specs resolved to the catalog's version.

use super::constants::{
    ALIAS_PREFIX, ANYWHERE, CATALOG_PREFIX, DEFAULT, FILE_MANIFEST, FILE_PREFIX, GIT_PREFIXES,
    GIT_SUFFIX, LINK_PREFIX, PATH_STARTS, SEMVER_PREFIX, URL_PREFIXES, WORKSPACE_PREFIX,
};
use crate::domain::packages::constants::{SEPARATOR, UNRESOLVED_CATALOG};
use crate::domain::packages::service::{directory, relative, within};
use crate::domain::packages::types::{
    Binding, Catalog, CatalogName, Catalogs, Containment, Directory, ManifestPath, PackageName,
    Packages, RepositoryPath, Requirement, Spec, Version,
};
use ryo::languages;
use std::collections::BTreeMap;

/// The directory `spec` names: after `file:` or `link:`, or the spec itself when it starts `./` or `../`.
fn directory_of(spec: Spec<'_>) -> Option<RepositoryPath<'_>> {
    let text = spec.0.trim();
    text.strip_prefix(FILE_PREFIX)
        .or_else(|| text.strip_prefix(LINK_PREFIX))
        .or_else(|| {
            PATH_STARTS
                .iter()
                .any(|start| text.starts_with(start))
                .then_some(text)
        })
        .map(RepositoryPath)
}

/// Where `spec` says its package comes from: a `workspace:` member, a `file:` or `link:` directory, a git repository (a git URL, a
/// host's shorthand or `owner/repository`), an archive's URL, or else the registry (a version range, a tag, an `npm:` alias).
fn origin_of(spec: Spec<'_>) -> languages::DependencyOrigin {
    let text = spec.0.trim();
    if text.starts_with(WORKSPACE_PREFIX) {
        languages::DependencyOrigin::Workspace
    } else if directory_of(spec).is_some() {
        languages::DependencyOrigin::Path
    } else if GIT_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
        languages::DependencyOrigin::Git
    } else if URL_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
        if text.split('#').next().unwrap_or(text).ends_with(GIT_SUFFIX) {
            languages::DependencyOrigin::Git
        } else {
            languages::DependencyOrigin::Url
        }
    } else if !text.starts_with(ALIAS_PREFIX)
        && !text.starts_with('@')
        && text.split('#').next().is_some_and(|repository| {
            repository.split('/').count() == 2 && !repository.contains(' ')
        })
    {
        languages::DependencyOrigin::Git
    } else {
        languages::DependencyOrigin::Registry
    }
}

/// The requirement a manifest in `base` writes for `binding` with `spec`: a catalog spec waits for its catalog, any other names its package and directory itself.
pub(crate) fn requirement(
    base: Directory<'_>,
    binding: Binding<'_>,
    spec: Spec<'_>,
    kind: languages::DependencyKind,
    optional: languages::Optionality,
) -> Requirement {
    if let Some(name) = spec.0.strip_prefix(CATALOG_PREFIX) {
        return Requirement {
            binding: PackageName(binding.0.into()),
            package: PackageName(binding.0.into()),
            target: None,
            kind,
            optional,
            origin: languages::DependencyOrigin::Registry,
            catalog: Some(CatalogName(name.trim().into())),
            spec: Version(spec.0.into()),
            written_catalog: Version(spec.0.into()),
        };
    }
    let name = spec
        .0
        .strip_prefix(ALIAS_PREFIX)
        .map_or(binding.0, |alias| {
            alias
                .rsplit_once('@')
                .filter(|(name, _)| !name.is_empty())
                .map_or(alias, |(name, _)| name)
        });
    Requirement {
        binding: PackageName(binding.0.into()),
        package: PackageName(name.into()),
        origin: origin_of(spec),
        target: directory_of(spec).map(|path| {
            relative(
                base,
                RepositoryPath(&format!(
                    "{}{SEPARATOR}{FILE_MANIFEST}",
                    path.0.trim_end_matches(SEPARATOR)
                )),
            )
        }),
        kind,
        optional,
        catalog: None,
        spec: Version(spec.0.into()),
        written_catalog: Version(Box::default()),
    }
}

/// The revision a git spec names after its `#` (`github:owner/repository#v1.2.0` names `v1.2.0`), which npm reads as a commit-ish:
/// a branch, a tag or a commit, not said which. Empty for a spec of another origin, one that names none, and a `#semver:` range,
/// which is a requirement.
#[must_use]
pub(crate) fn revision_of(spec: &Version, origin: languages::DependencyOrigin) -> Spec<'_> {
    if origin != languages::DependencyOrigin::Git {
        return Spec(<&str>::default());
    }
    Spec(
        spec.0
            .split_once('#')
            .map(|(_, fragment)| fragment.trim())
            .filter(|fragment| !fragment.starts_with(SEMVER_PREFIX))
            .unwrap_or_default(),
    )
}

/// Whether an override's key names a package alone, as every dependency on it is replaced: `name`, `@scope/name`, or Yarn's
/// `**/name`. A key that selects by version (`name@1`), by parent (`parent>name`, `parent/name`) or by another pattern replaces only
/// some of them, which a dependency's own row cannot say, and is left out.
#[must_use]
pub(crate) fn overridden(key: Binding<'_>) -> Option<PackageName> {
    let name = key.0.strip_prefix(ANYWHERE).unwrap_or(key.0);
    let rest = match name.strip_prefix('@') {
        Some(scoped) => match scoped.split_once('/') {
            Some((scope, rest)) if !scope.is_empty() => rest,
            Some(_) | None => return None,
        },
        None => name,
    };
    let plain = !rest.is_empty() && !rest.contains(['@', '/', '>', '*', ' ']);
    plain.then(|| PackageName(name.into()))
}

/// Whether `spec` admits one version only: its version, after a `workspace:` or an `npm:<name>@` before it, is a whole version
/// (`1.2.3`, `=1.2.3`, `v1.2.3`, a pre-release or a build included). A range, a tag, a wildcard, a directory, a git repository and an
/// archive's URL admit more than one, or none by a version.
#[must_use]
pub(crate) fn exactness(spec: &Version) -> languages::Flag {
    let text = spec.0.trim();
    let version = text
        .strip_prefix(WORKSPACE_PREFIX)
        .or_else(|| {
            text.strip_prefix(ALIAS_PREFIX).map(|alias| {
                alias
                    .rsplit_once('@')
                    .filter(|(name, _)| !name.is_empty())
                    .map_or(<&str>::default(), |(_, version)| version)
            })
        })
        .unwrap_or(text)
        .trim();
    let version = version.strip_prefix('=').unwrap_or(version).trim_start();
    let version = version.strip_prefix('v').unwrap_or(version);
    let core = version.split_once('+').map_or(version, |(core, _)| core);
    let core = core.split_once('-').map_or(core, |(core, _)| core);
    let parts: Vec<&str> = core.split('.').collect();
    let whole = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && !version.contains(char::is_whitespace);
    if whole {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// A catalog of the entries a manifest writes.
pub(crate) fn catalog<Name, Range>(entries: BTreeMap<Name, Range>) -> Catalog
where
    Name: Into<Box<str>>,
    Range: Into<Box<str>>,
{
    Catalog(
        entries
            .into_iter()
            .map(|(name, range)| (PackageName(name.into()), Version(range.into())))
            .collect(),
    )
}

/// The version range the catalog `name` gives `binding`: the default catalog for an empty name or `default`, else the named one.
fn spec_of<'a>(
    catalogs: &'a Catalogs,
    name: &CatalogName,
    binding: &PackageName,
) -> Option<&'a Version> {
    let named = |catalog: &CatalogName| {
        let each = catalogs.named.get(catalog)?;
        each.0.get(binding)
    };
    if name.0.is_empty() || &*name.0 == DEFAULT {
        catalogs
            .default
            .0
            .get(binding)
            .or_else(|| named(&CatalogName(DEFAULT.into())))
    } else {
        named(name)
    }
}

/// Give every `catalog:` requirement the version its nearest enclosing workspace catalog writes for it. A requirement no catalog
/// answers stays a catalog reference, which no edge is drawn from, and is returned as refused.
pub(crate) fn resolved(packages: &mut Packages) -> Vec<languages::Refused> {
    let roots: Vec<(ManifestPath, Catalogs)> = packages
        .entries
        .iter()
        .filter(|package| package.declared.catalogs != Catalogs::default())
        .map(|package| (package.manifest.clone(), package.declared.catalogs.clone()))
        .collect();
    let mut refused = Vec::new();
    for package in &mut packages.entries {
        let manifest = package.manifest.clone();
        let nearest = roots
            .iter()
            .filter(|(root, _)| {
                within(
                    directory(RepositoryPath(&root.0)),
                    RepositoryPath(&manifest.0),
                ) == Containment::Inside
            })
            .max_by_key(|(root, _)| directory(RepositoryPath(&root.0)).0.len());
        for requirement in &mut package.declared.requirements {
            let Some(name) = requirement.catalog.clone() else {
                continue;
            };
            let version = nearest.and_then(|(root, each)| {
                spec_of(each, &name, &requirement.binding).map(|found| (root, found))
            });
            if let Some((root, version)) = version {
                // A path in a catalog is from the directory of the file that writes the catalog, not of the package that uses it. The
                // manifest's own `catalog:` spelling stays with it.
                let written = requirement.written_catalog.clone();
                *requirement = self::requirement(
                    directory(RepositoryPath(&root.0)),
                    Binding(&requirement.binding.0),
                    Spec(&version.0),
                    requirement.kind,
                    requirement.optional,
                );
                requirement.written_catalog = written;
            } else {
                refused.push(languages::Refused {
                    path: languages::FilePath(manifest.0.clone()),
                    reason: languages::ToolMessage(
                        format!(
                            "{UNRESOLVED_CATALOG}{}` has an entry for `{}`: its version is unknown",
                            if name.0.is_empty() { DEFAULT } else { &name.0 },
                            requirement.binding.0
                        )
                        .into(),
                    ),
                });
            }
        }
    }
    refused
}
