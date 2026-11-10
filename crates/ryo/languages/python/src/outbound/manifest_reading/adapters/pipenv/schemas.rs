//! The shape of a `Pipfile`, which the language does not control, and its conversion into the port's own types: `[packages]` are what
//! the package needs, `[dev-packages]` what developing it needs. A package is a version or a table (`version`, `markers`, `path`,
//! `editable`, `extras`, `git`).

use std::collections::BTreeMap;

use crate::domain::dependencies::service::pep440_exactness;
use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};
use serde::Deserialize;

use crate::domain::packages::service::{directory_of, locating, url_origin};
use crate::domain::packages::types::{
    Checkout, Declared, Interpreter, ManifestPath, Naming, Reference, Requirement, Revision,
    Specifier, Written,
};
use crate::domain::patterns::types::{Directory, PatternPart};
use crate::outbound::manifest_reading::errors::ManifestReadingError;

/// A `Pipfile`.
#[derive(Debug, Default, Deserialize)]
struct PipfileSchema {
    /// The packages the project runs with, by name.
    #[serde(default)]
    packages: BTreeMap<String, toml::Value>,
    /// The packages it is developed with, by name.
    #[serde(default, rename = "dev-packages")]
    dev_packages: BTreeMap<String, toml::Value>,
}

/// A `Pipfile` and the directory it is in, which its paths are from.
#[derive(Debug)]
struct ManifestSchema {
    /// The directory the `Pipfile` is in, from the repository root.
    directory: Box<str>,
    /// What the `Pipfile` says.
    pipfile: PipfileSchema,
}

/// A package written as a table.
#[derive(Debug, Default, Deserialize)]
struct PackageTableSchema {
    /// A directory the package is installed from.
    path: Option<String>,
    /// A git repository the package is installed from.
    git: Option<String>,
    /// The revision of the git repository the package is taken at: a branch, a tag or a commit.
    #[serde(rename = "ref")]
    revision: Option<String>,
    /// A file or URL the package is installed from.
    file: Option<String>,
    /// The environment markers the package is needed under.
    markers: Option<String>,
    /// The version specifier, as written: `==1.2`, or `*` for any.
    version: Option<String>,
}

/// The requirements of one table, all of one kind.
fn requirements(
    packages: BTreeMap<String, toml::Value>,
    kind: DependencyKind,
    directory: Directory<'_>,
) -> impl Iterator<Item = Requirement> {
    packages.into_iter().map(move |(name, package)| {
        // A version alone is a string; a table carries a version, a marker, a path, a git repository or a file's URL.
        let table: PackageTableSchema = if let toml::Value::String(version) = package {
            PackageTableSchema {
                version: Some(version),
                ..PackageTableSchema::default()
            }
        } else {
            package.try_into().unwrap_or_default()
        };
        let origin = if table.path.is_some() {
            DependencyOrigin::Path
        } else if table.git.is_some() {
            DependencyOrigin::Git
        } else {
            table
                .file
                .as_deref()
                .map_or(DependencyOrigin::Registry, |file| {
                    url_origin(PatternPart(file))
                })
        };
        let written = match &table.markers {
            Some(markers) => format!("{name} ; {markers}"),
            None => name,
        };
        let specifier = Specifier(table.version.unwrap_or_default().into());
        Requirement {
            exact: pep440_exactness(&specifier),
            specifier,
            written: Written(written.into()),
            kind,
            optional: Optionality::Required,
            origin,
            checkout: Checkout {
                revision: Revision(table.revision.unwrap_or_default().into()),
                ..Checkout::default()
            },
            interpreter: Interpreter::default(),
            reference: table.path.map_or(Reference::Named, |path| {
                locating(directory, PatternPart(&path))
            }),
        }
    })
}

impl From<ManifestSchema> for Declared {
    fn from(schema: ManifestSchema) -> Self {
        let directory = Directory(&schema.directory);
        let mut all: Vec<Requirement> =
            requirements(schema.pipfile.packages, DependencyKind::Normal, directory).collect();
        all.extend(requirements(
            schema.pipfile.dev_packages,
            DependencyKind::Dev,
            directory,
        ));
        Self {
            name: Naming::Unnamed,
            members: Vec::new(),
            excludes: Vec::new(),
            requirements: all,
            sources: Vec::new(),
            overrides: Vec::new(),
        }
    }
}

/// What the text of the `Pipfile` at `manifest` declares.
///
/// # Errors
///
/// `Unreadable` when the text is not TOML.
pub(super) fn declared(
    manifest: &ManifestPath,
    text: &str,
) -> Result<Declared, ManifestReadingError> {
    let pipfile = toml::from_str(text).map_err(|error: toml::de::Error| {
        ManifestReadingError::Unreadable {
            reason: error.message().to_owned(),
        }
    })?;
    Ok(ManifestSchema {
        directory: directory_of(manifest).0.into(),
        pipfile,
    }
    .into())
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
