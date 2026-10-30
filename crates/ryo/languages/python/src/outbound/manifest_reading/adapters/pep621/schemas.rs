//! The shape of the tables of a `pyproject.toml` that the standards define, which the language does not control, and their conversion
//! into the port's own types: PEP 621 for the package and its requirements, PEP 735 for the groups of requirements, PEP 518 for what
//! building needs.

use std::collections::BTreeMap;

use crate::domain::dependencies::service::pep440_exactness;
use ryo::languages::{DependencyKind, Optionality};
use serde::Deserialize;

use crate::domain::dependencies::service::checkout_of;
use crate::domain::packages::service::{origin_of, specifier_of};
use crate::domain::packages::types::{
    Declared, Interpreter, Naming, PackageName, Reference, Requirement, Written,
};
use crate::outbound::manifest_reading::errors::ManifestReadingError;

/// A `pyproject.toml`.
#[derive(Debug, Default, Deserialize)]
struct PyprojectSchema {
    /// The `[project]` table: the package and its requirements.
    project: Option<ProjectSchema>,
    /// The `[dependency-groups]` table: named groups of requirements.
    #[serde(default, rename = "dependency-groups")]
    dependency_groups: BTreeMap<String, Vec<GroupEntrySchema>>,
    /// The `[build-system]` table: what building the package needs.
    #[serde(rename = "build-system")]
    build_system: Option<BuildSystemSchema>,
}

/// A `[project]`.
#[derive(Debug, Default, Deserialize)]
struct ProjectSchema {
    /// The package's name.
    name: Option<String>,
    /// The requirements the package runs with.
    #[serde(default)]
    dependencies: Vec<String>,
    /// The extras, each with the requirements it adds.
    #[serde(default, rename = "optional-dependencies")]
    optional_dependencies: BTreeMap<String, Vec<String>>,
}

/// An entry of a group of requirements: a requirement, or the inclusion of another group, which names no requirement of its own: the
/// included group's are counted once, as that group's.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GroupEntrySchema {
    /// A requirement, as PEP 508 writes it.
    Requirement(String),
    /// An `{include-group = "..."}` table, which brings in another group's requirements.
    #[expect(
        clippy::empty_enum_variants_with_brackets,
        reason = "serde's untagged form reads a table into a variant with braces; the unit variant `Inclusion` would only read an empty value, and the table would be refused"
    )]
    Inclusion {},
}

/// A `[build-system]`.
#[derive(Debug, Default, Deserialize)]
struct BuildSystemSchema {
    /// The requirements building the package needs.
    #[serde(default)]
    requires: Vec<String>,
}

/// The requirements `texts` are, all of one kind.
fn requirements(
    texts: Vec<String>,
    kind: DependencyKind,
    optional: Optionality,
) -> impl Iterator<Item = Requirement> {
    texts.into_iter().map(move |text| {
        let written = Written(text.into());
        let specifier = specifier_of(&written);
        Requirement {
            exact: pep440_exactness(&specifier),
            specifier,
            origin: origin_of(&written),
            checkout: checkout_of(&written),
            interpreter: Interpreter::default(),
            written,
            kind,
            optional,
            reference: Reference::Named,
        }
    })
}

impl From<PyprojectSchema> for Declared {
    fn from(schema: PyprojectSchema) -> Self {
        let project = schema.project.unwrap_or_default();
        let name = project.name.map_or(Naming::Unnamed, |name| {
            Naming::Named(PackageName(name.into()))
        });
        let mut all: Vec<Requirement> = requirements(
            project.dependencies,
            DependencyKind::Normal,
            Optionality::Required,
        )
        .collect();
        for texts in project.optional_dependencies.into_values() {
            all.extend(requirements(
                texts,
                DependencyKind::Normal,
                Optionality::Optional,
            ));
        }
        for entries in schema.dependency_groups.into_values() {
            let texts = entries
                .into_iter()
                .filter_map(|entry| match entry {
                    GroupEntrySchema::Requirement(text) => Some(text),
                    GroupEntrySchema::Inclusion {} => None,
                })
                .collect();
            all.extend(requirements(
                texts,
                DependencyKind::Dev,
                Optionality::Required,
            ));
        }
        all.extend(requirements(
            schema.build_system.unwrap_or_default().requires,
            DependencyKind::Build,
            Optionality::Required,
        ));
        Self {
            name,
            members: Vec::new(),
            excludes: Vec::new(),
            requirements: all,
            sources: Vec::new(),
            overrides: Vec::new(),
        }
    }
}

/// What the PEP tables of the text of a `pyproject.toml` declare.
///
/// # Errors
///
/// `Unreadable` when the text is not TOML.
pub(super) fn declared(text: &str) -> Result<Declared, ManifestReadingError> {
    toml::from_str::<PyprojectSchema>(text)
        .map(Declared::from)
        .map_err(|error| ManifestReadingError::Unreadable {
            reason: error.message().to_owned(),
        })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
