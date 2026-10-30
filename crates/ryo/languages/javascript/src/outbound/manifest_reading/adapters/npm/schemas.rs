//! The package manifest's shape as npm and Yarn write it, with the overrides pnpm writes in it, and its conversion to a package
//! declaration.

use crate::domain::catalogs::service::{overridden, requirement};
use crate::domain::packages::service::{directory, is_package, relative};
use crate::domain::packages::types::{Binding, Declared, ManifestPath, PackageName, Spec, Version};
use crate::domain::packages::types::{ManifestKind, RepositoryPath};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use ryo::languages::{self, FilePath, ManifestFile};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The manifest to read and the files its workspace globs may match.
pub(super) struct ManifestSchema<'a> {
    /// The manifest's path and text.
    pub manifest: &'a ManifestFile,
    /// The repository's files, which the workspace globs select members from.
    pub candidates: &'a [FilePath],
}

/// `workspaces`: the member globs, as a list or under `packages`.
#[derive(Deserialize)]
#[serde(untagged)]
enum WorkspacesSchema {
    /// The member globs, written as a list.
    List(Vec<String>),
    /// The object form Yarn also accepts.
    Object {
        /// The member globs.
        #[serde(default)]
        packages: Vec<String>,
    },
}

/// The parts of a package manifest a declaration is read from.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageSchema {
    /// The package's name.
    name: Option<String>,
    /// The workspace's members.
    workspaces: Option<WorkspacesSchema>,
    /// The packages it needs to run, by name and spec.
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    /// The packages it needs to be developed.
    #[serde(default)]
    dev_dependencies: BTreeMap<String, String>,
    /// The packages it expects the dependent to provide.
    #[serde(default)]
    peer_dependencies: BTreeMap<String, String>,
    /// The packages it runs without when they fail to install.
    #[serde(default)]
    optional_dependencies: BTreeMap<String, String>,
    /// What it says of each peer dependency: whether it is optional.
    #[serde(default)]
    peer_dependencies_meta: BTreeMap<String, PeerMetaSchema>,
    /// npm's overrides: a spec for a package, or an object whose `.` is the spec for the package and whose other keys are for what
    /// it depends on.
    #[serde(default)]
    overrides: BTreeMap<String, OverrideSchema>,
    /// Yarn's resolutions, which Bun reads too: a spec for each package.
    #[serde(default)]
    resolutions: BTreeMap<String, String>,
    /// pnpm's settings in the manifest, read for its overrides.
    pnpm: Option<PnpmSchema>,
}

/// One of npm's overrides.
#[derive(Deserialize)]
#[serde(untagged)]
enum OverrideSchema {
    /// The spec that replaces the package.
    Spec(String),
    /// The spec for the package under `.`, and the overrides of what it depends on, which are not about the package.
    Nested {
        /// The spec for the package, when there is one.
        #[serde(rename = ".")]
        own: Option<String>,
    },
}

/// The `pnpm` settings of a manifest.
#[derive(Default, Deserialize)]
struct PnpmSchema {
    /// A spec for each package.
    #[serde(default)]
    overrides: BTreeMap<String, String>,
}

/// What `peerDependenciesMeta` says of one peer dependency.
#[derive(Default, Deserialize)]
struct PeerMetaSchema {
    /// Whether the dependent works without the peer.
    #[serde(default)]
    optional: bool,
}

/// The specs the manifest puts in the place of every dependency on a package, by its name: npm's `overrides`, then Yarn's
/// `resolutions`, then pnpm's `overrides`, the first that names a package kept. A key that selects only some of a package's
/// dependents is no override of a dependency's own.
fn overrides_of(
    overrides: BTreeMap<String, OverrideSchema>,
    resolutions: BTreeMap<String, String>,
    pnpm: Option<PnpmSchema>,
) -> BTreeMap<PackageName, Version> {
    let mut found = BTreeMap::new();
    let written = overrides
        .into_iter()
        .filter_map(|(key, each)| match each {
            OverrideSchema::Spec(spec) => Some((key, spec)),
            OverrideSchema::Nested { own } => own.map(|spec| (key, spec)),
        })
        .chain(resolutions)
        .chain(pnpm.unwrap_or_default().overrides);
    for (key, spec) in written {
        if let Some(name) = overridden(Binding(&key)) {
            let _: &mut Version = found.entry(name).or_insert(Version(spec.into()));
        }
    }
    found
}

impl TryFrom<ManifestSchema<'_>> for Declared {
    type Error = ManifestReadingError;

    fn try_from(input: ManifestSchema<'_>) -> Result<Self, Self::Error> {
        let base = directory(RepositoryPath(&input.manifest.path.0));
        let text = &input.manifest.text.0;
        let package: PackageSchema = serde_json::from_str(text)
            .map_err(|error| ManifestReadingError::Unreadable(error.to_string().into()))?;
        // Each member once, in the order of its path.
        let mut members = std::collections::BTreeSet::new();
        let patterns = match package.workspaces {
            Some(
                WorkspacesSchema::List(patterns) | WorkspacesSchema::Object { packages: patterns },
            ) => patterns,
            None => Vec::new(),
        };
        let mut includes = Vec::new();
        let mut excludes = Vec::new();
        for pattern in patterns {
            let (negated, pattern) = pattern
                .strip_prefix('!')
                .map_or((false, pattern.as_str()), |pattern| (true, pattern));
            let pattern = relative(base, RepositoryPath(pattern));
            let matcher = globset::GlobBuilder::new(&pattern.0)
                .literal_separator(true)
                .build()
                .map_err(|error| ManifestReadingError::Unreadable(error.to_string().into()))?
                .compile_matcher();
            if negated {
                excludes.push(matcher);
            } else {
                includes.push(matcher);
            }
        }
        for candidate in input.candidates {
            if candidate != &input.manifest.path
                && is_package(RepositoryPath(&candidate.0)) == ManifestKind::Package
                && includes
                    .iter()
                    .any(|pattern| pattern.is_match(directory(RepositoryPath(&candidate.0)).0))
                && !excludes
                    .iter()
                    .any(|pattern| pattern.is_match(directory(RepositoryPath(&candidate.0)).0))
            {
                let _: bool = members.insert(ManifestPath(candidate.0.clone()));
            }
        }
        let mut requirements = Vec::new();
        for (table, kind, optional) in [
            (
                &package.dependencies,
                languages::DependencyKind::Normal,
                languages::Optionality::Required,
            ),
            (
                &package.dev_dependencies,
                languages::DependencyKind::Dev,
                languages::Optionality::Required,
            ),
            (
                &package.peer_dependencies,
                languages::DependencyKind::Peer,
                languages::Optionality::Required,
            ),
            (
                &package.optional_dependencies,
                languages::DependencyKind::Normal,
                languages::Optionality::Optional,
            ),
        ] {
            for (binding, version) in table {
                if kind == languages::DependencyKind::Normal
                    && optional == languages::Optionality::Required
                    && package.optional_dependencies.contains_key(binding)
                {
                    continue;
                }
                let optional = if kind == languages::DependencyKind::Peer
                    && package
                        .peer_dependencies_meta
                        .get(binding)
                        .is_some_and(|meta| meta.optional)
                {
                    languages::Optionality::Optional
                } else {
                    optional
                };
                requirements.push(requirement(
                    base,
                    Binding(binding),
                    Spec(version),
                    kind,
                    optional,
                ));
            }
        }
        let overrides = overrides_of(package.overrides, package.resolutions, package.pnpm);
        Ok(Declared {
            name: package.name.map(|name| PackageName(name.into())),
            members: members.into_iter().collect(),
            requirements,
            overrides,
            ..Declared::default()
        })
    }
}

/// Read the external manifest into the domain's declaration.
pub(super) fn read(
    manifest: &ManifestFile,
    candidates: &[FilePath],
) -> Result<Declared, ManifestReadingError> {
    Declared::try_from(ManifestSchema {
        manifest,
        candidates,
    })
}
