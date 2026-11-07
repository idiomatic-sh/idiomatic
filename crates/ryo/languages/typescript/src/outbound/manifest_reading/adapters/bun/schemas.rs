//! The catalogs Bun writes in the root package manifest: at the top level, or beside the workspace packages.

use crate::domain::catalogs::service::catalog;
use crate::domain::packages::types::{CatalogName, Catalogs, Declared};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use ryo::languages::ManifestFile;
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::BTreeMap;

/// `workspaces`: the member globs alone, or an object that may carry catalogs beside them.
#[derive(Deserialize)]
#[serde(untagged)]
enum WorkspacesSchema {
    /// The member globs, which this adapter leaves to npm's.
    List(Vec<IgnoredAny>),
    /// The object form, whose catalogs Bun reads.
    Object {
        /// The default catalog written beside the members.
        #[serde(default)]
        catalog: BTreeMap<String, String>,
        /// The named catalogs written beside the members.
        #[serde(default)]
        catalogs: BTreeMap<String, BTreeMap<String, String>>,
    },
}

/// The parts of a root package manifest that hold Bun's catalogs.
#[derive(Default, Deserialize)]
struct PackageSchema {
    /// The workspace, which may hold the catalogs.
    workspaces: Option<WorkspacesSchema>,
    /// The default catalog written at the top level.
    #[serde(default)]
    catalog: BTreeMap<String, String>,
    /// The named catalogs written at the top level.
    #[serde(default)]
    catalogs: BTreeMap<String, BTreeMap<String, String>>,
}

/// The manifest to read, as the adapter hands it over.
pub(super) struct ManifestSchema<'a> {
    /// The manifest's path and text.
    pub manifest: &'a ManifestFile,
}

impl TryFrom<ManifestSchema<'_>> for Declared {
    type Error = ManifestReadingError;

    fn try_from(input: ManifestSchema<'_>) -> Result<Self, Self::Error> {
        let package: PackageSchema = serde_json::from_str(&input.manifest.text.0)
            .map_err(|error| ManifestReadingError::Unreadable(error.to_string().into()))?;
        let mut default = package.catalog;
        let mut named = package.catalogs;
        match package.workspaces {
            Some(WorkspacesSchema::Object {
                catalog: beside_default,
                catalogs: beside_named,
            }) => {
                default.extend(beside_default);
                named.extend(beside_named);
            }
            // A workspace list holds no catalog; its patterns belong to the npm adapter.
            Some(WorkspacesSchema::List(patterns)) => drop(patterns),
            None => {}
        }
        Ok(Declared {
            catalogs: Catalogs {
                default: catalog(default),
                named: named
                    .into_iter()
                    .map(|(name, entries)| (CatalogName(name.into()), catalog(entries)))
                    .collect(),
            },
            ..Declared::default()
        })
    }
}

/// Read the package manifest's catalogs into the domain's declaration.
pub(super) fn read(manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
    Declared::try_from(ManifestSchema { manifest })
}
