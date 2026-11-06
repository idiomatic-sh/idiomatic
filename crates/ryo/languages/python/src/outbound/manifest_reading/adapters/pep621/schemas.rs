//! The shape of the tables of a `pyproject.toml` that the standards define, which the language does not control, and their conversion
//! into the port's own types: PEP 621 for the package and its requirements, PEP 735 for the groups of requirements, PEP 518 for what
//! building needs.

use std::collections::BTreeMap;

use crate::domain::dependencies::service::pep440_exactness;
use crate::domain::manifest_validation::types::{
    Location, ManifestDiagnostic, MetadataDemand, Phase, Validation,
};
use crate::domain::packages::types::Reason;
use crate::outbound::manifest_reading::types::ManifestRead;
use ryo::languages::{DependencyKind, Optionality};
use ryo::languages::{Offset, Span};
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

/// Parse once, retaining borrowed spelling and source ranges across the two deserializer views.
pub(super) fn read(text: &str, demand: MetadataDemand) -> ManifestRead {
    let document = match toml::de::DeTable::parse(text) {
        Ok(document) => document,
        Err(error) => {
            let validation = (demand != MetadataDemand::Projection)
                .then(|| validation_error(text, Phase::Parse, &error));
            return ManifestRead {
                declaration: (demand != MetadataDemand::Diagnostics).then(|| {
                    Err(ManifestReadingError::Unreadable {
                        reason: error.message().to_owned(),
                    })
                }),
                validation,
            };
        }
    };
    let validation = if demand != MetadataDemand::Projection {
        Some(
            match pyproject_toml::PyProjectToml::deserialize(toml::de::Deserializer::from(
                document.clone(),
            )) {
                Ok(_) => Validation::default(),
                Err(error) => validation_error(text, Phase::Validation, &error),
            },
        )
    } else {
        None
    };
    let declaration = if demand == MetadataDemand::Diagnostics {
        None
    } else {
        Some(
            PyprojectSchema::deserialize(toml::de::Deserializer::from(document))
                .map_err(|error| ManifestReadingError::Unreadable {
                    reason: error.message().to_owned(),
                })
                .and_then(projected),
        )
    };
    ManifestRead {
        declaration,
        validation,
    }
}

/// PEP dependency fields must contain real requirements; retain their original spelling after validation.
fn projected(schema: PyprojectSchema) -> Result<Declared, ManifestReadingError> {
    let project = schema.project.as_ref();
    let normal = project
        .into_iter()
        .flat_map(|project| project.dependencies.iter());
    let optional = project
        .into_iter()
        .flat_map(|project| project.optional_dependencies.values().flatten());
    let build = schema
        .build_system
        .iter()
        .flat_map(|build| build.requires.iter());
    let groups = schema
        .dependency_groups
        .values()
        .flatten()
        .filter_map(|entry| match entry {
            GroupEntrySchema::Requirement(text) => Some(text),
            GroupEntrySchema::Inclusion {} => None,
        });
    for text in normal.chain(optional).chain(build).chain(groups) {
        text.parse::<pep508_rs::Requirement<pep508_rs::VerbatimUrl>>()
            .map_err(|error| ManifestReadingError::Unreadable {
                reason: error.to_string(),
            })?;
    }
    Ok(Declared::from(schema))
}

fn validation_error(text: &str, phase: Phase, error: &toml::de::Error) -> Validation {
    let location = match error.span() {
        None => Location::FileOrigin,
        Some(range) => {
            let offsets = match (u32::try_from(range.start), u32::try_from(range.end)) {
                (Ok(start), Ok(end)) => Some((start, end)),
                _ => None,
            };
            let Some((start, end)) =
                offsets.filter(|_| range.start <= range.end && text.get(range.clone()).is_some())
            else {
                return Validation {
                    diagnostics: Vec::new(),
                    incomplete: Some(Reason(
                        "manifest diagnostic has no valid source mapping".into(),
                    )),
                };
            };
            Location::Span(Span {
                start: Offset(start),
                end: Offset(end),
            })
        }
    };
    Validation {
        diagnostics: vec![ManifestDiagnostic {
            phase,
            location,
            message: Reason(error.message().into()),
        }],
        incomplete: None,
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
