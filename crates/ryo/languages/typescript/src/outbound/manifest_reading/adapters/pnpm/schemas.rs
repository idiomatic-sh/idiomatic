//! The typed shape of `pnpm-workspace.yaml` and its conversion to package declarations.

use crate::domain::catalogs::service::{catalog, overridden};
use crate::domain::packages::service::{directory, is_package, relative};
use crate::domain::packages::types::{
    Binding, CatalogName, Catalogs, Declared, ManifestKind, ManifestPath, RepositoryPath, Version,
};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use ryo::languages::{FilePath, ManifestFile};
use serde::Deserialize;
use std::collections::BTreeMap;

/// What a pnpm workspace file declares: member globs, a default catalog and named catalogs.
#[derive(Default, Deserialize)]
struct WorkspaceSchema {
    /// The globs of the workspace's member packages.
    #[serde(default)]
    packages: Vec<String>,
    /// The default catalog.
    #[serde(default)]
    catalog: BTreeMap<String, String>,
    /// The named catalogs.
    #[serde(default)]
    catalogs: BTreeMap<String, BTreeMap<String, String>>,
    /// A spec for each package, which replaces every dependency on it.
    #[serde(default)]
    overrides: BTreeMap<String, String>,
}

/// The workspace file to read and the files its globs may match.
pub(super) struct ManifestSchema<'a> {
    /// The workspace file's path and text.
    pub manifest: &'a ManifestFile,
    /// The repository's files, which the member globs select from.
    pub candidates: &'a [FilePath],
}

impl TryFrom<ManifestSchema<'_>> for Declared {
    type Error = ManifestReadingError;

    fn try_from(input: ManifestSchema<'_>) -> Result<Self, Self::Error> {
        let base = directory(RepositoryPath(&input.manifest.path.0));
        // An empty file is a null document, which declares nothing.
        let workspace = yaml_serde::from_str::<Option<WorkspaceSchema>>(&input.manifest.text.0)
            .map_err(|error| ManifestReadingError::Unreadable(error.to_string().into()))?
            .unwrap_or_default();
        let mut includes = Vec::new();
        let mut excludes = Vec::new();
        for pattern in workspace.packages {
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
        let mut members = Vec::new();
        for candidate in input.candidates {
            let candidate_directory = directory(RepositoryPath(&candidate.0)).0;
            if is_package(RepositoryPath(&candidate.0)) == ManifestKind::Package
                && includes
                    .iter()
                    .any(|pattern| pattern.is_match(candidate_directory))
                && !excludes
                    .iter()
                    .any(|pattern| pattern.is_match(candidate_directory))
            {
                members.push(ManifestPath(candidate.0.clone()));
            }
        }
        members.sort();
        members.dedup();
        // A key that selects only some of a package's dependents is no override of a dependency's own.
        let overrides = workspace
            .overrides
            .into_iter()
            .filter_map(|(key, spec)| {
                overridden(Binding(&key)).map(|name| (name, Version(spec.into())))
            })
            .collect();
        Ok(Declared {
            members,
            overrides,
            catalogs: Catalogs {
                default: catalog(workspace.catalog),
                named: workspace
                    .catalogs
                    .into_iter()
                    .map(|(name, entries)| (CatalogName(name.into()), catalog(entries)))
                    .collect(),
            },
            ..Declared::default()
        })
    }
}

/// Read the external workspace manifest into the domain's declaration.
pub(super) fn read(
    manifest: &ManifestFile,
    candidates: &[FilePath],
) -> Result<Declared, ManifestReadingError> {
    Declared::try_from(ManifestSchema {
        manifest,
        candidates,
    })
}
