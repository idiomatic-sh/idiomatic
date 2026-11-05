//! The shape of the tables Poetry reads in a `pyproject.toml`, which the language does not control, and their conversion into the
//! port's own types: `[tool.poetry]` for the package, `dependencies`, `group.<name>.dependencies` and the legacy `dev-dependencies` for
//! its requirements. A requirement is a version, a table (`version`, `optional`, `path`, `git` with `branch`, `tag` or `rev`, `url`,
//! `markers`, `platform`, `python`, `extras`), or a list
//! of tables.

use std::collections::BTreeMap;

use crate::domain::dependencies::service::poetry_exactness;
use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};
use serde::Deserialize;

use crate::domain::packages::service::{directory_of, locating};
use crate::domain::packages::types::{
    Checkout, Declared, Interpreter, ManifestPath, Naming, PackageName, Reference, Requirement,
    Revision, Specifier, Written,
};
use crate::domain::patterns::types::{Directory, Part};
use crate::outbound::manifest_reading::errors::ManifestReadingError;

/// The name Poetry gives the dependencies of the package itself.
const MAIN: &str = "main";

/// The requirement on the interpreter, which names no package.
const INTERPRETER: &str = "python";

/// A `pyproject.toml`.
#[derive(Debug, Default, Deserialize)]
// the shape is the tables of pyproject.toml, read table for table: a flatter shape reads a different file
struct PyprojectSchema {
    /// The `[tool]` table, which holds Poetry's.
    tool: Option<ToolSchema>,
}

/// A `pyproject.toml` and the directory it is in, which its paths are from.
#[derive(Debug)]
struct ManifestSchema {
    /// The directory the `pyproject.toml` is in, from the repository root.
    directory: Box<str>,
    /// What the `pyproject.toml` says.
    pyproject: PyprojectSchema,
}

/// A `[tool]`.
#[derive(Debug, Default, Deserialize)]
struct ToolSchema {
    /// The `[tool.poetry]` table.
    poetry: Option<PoetrySchema>,
}

/// A `[tool.poetry]`.
#[derive(Debug, Default, Deserialize)]
struct PoetrySchema {
    /// The package's name.
    name: Option<String>,
    /// The packages it runs with, by name.
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
    /// The packages it is developed with, in Poetry's older form.
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, toml::Value>,
    /// The groups of dependencies, by name.
    #[serde(default)]
    group: BTreeMap<String, GroupSchema>,
}

/// A `[tool.poetry.group.<name>]`.
#[derive(Debug, Default, Deserialize)]
struct GroupSchema {
    /// The packages the group holds, by name.
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
}

/// A dependency written as a table.
#[derive(Debug, Default, Deserialize)]
struct DependencyTableSchema {
    /// Whether the package is only installed for an extra.
    #[serde(default)]
    optional: bool,
    /// A directory the package is installed from.
    path: Option<String>,
    /// A git repository the package is installed from.
    git: Option<String>,
    /// The branch of the git repository the package follows.
    branch: Option<String>,
    /// The tag of the git repository the package is taken at.
    tag: Option<String>,
    /// The revision of the git repository the package is taken at.
    rev: Option<String>,
    /// The versions of Python the package is needed under, as written.
    python: Option<String>,
    /// A URL the package is installed from.
    url: Option<String>,
    /// The environment markers the package is needed under.
    markers: Option<String>,
    /// The platform the package is needed on, as Python names it in `sys.platform` (`linux`, `darwin`, `win32`).
    platform: Option<String>,
    /// The version constraint, as written.
    version: Option<String>,
}

/// The requirements a dependency is: one for each table it is written as.
fn requirements(
    name: &str,
    dependency: toml::Value,
    kind: DependencyKind,
    directory: Directory<'_>,
) -> Vec<Requirement> {
    // A version alone is a string, the constraint of a table with no other key; a table is one constraint, and a list of tables one
    // for each.
    let tables: Vec<DependencyTableSchema> = match dependency {
        toml::Value::Table(table) => vec![table.try_into().unwrap_or_default()],
        toml::Value::Array(each) => each
            .into_iter()
            .map(|table| table.try_into().unwrap_or_default())
            .collect(),
        toml::Value::String(version) => vec![DependencyTableSchema {
            version: Some(version),
            ..DependencyTableSchema::default()
        }],
        toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => vec![DependencyTableSchema::default()],
    };
    tables
        .into_iter()
        .map(|table| {
            // Poetry's `platform` is the marker `sys_platform`; with `markers` beside it, both must hold.
            let platform = table
                .platform
                .as_ref()
                .map(|platform| format!("sys_platform == '{platform}'"));
            let written = match (&table.markers, platform) {
                (Some(markers), Some(platform)) => format!("{name} ; ({markers}) and {platform}"),
                (Some(markers), None) => format!("{name} ; {markers}"),
                (None, Some(platform)) => format!("{name} ; {platform}"),
                (None, None) => name.to_owned(),
            };
            let optional = if table.optional {
                Optionality::Optional
            } else {
                Optionality::Required
            };
            let origin = if table.path.is_some() {
                DependencyOrigin::Path
            } else if table.git.is_some() {
                DependencyOrigin::Git
            } else if table.url.is_some() {
                DependencyOrigin::Url
            } else {
                DependencyOrigin::Registry
            };
            let specifier = Specifier(table.version.unwrap_or_default().into());
            let checkout = Checkout {
                branch: Revision(table.branch.unwrap_or_default().into()),
                tag: Revision(table.tag.unwrap_or_default().into()),
                revision: Revision(table.rev.unwrap_or_default().into()),
            };
            Requirement {
                checkout,
                interpreter: Interpreter(table.python.unwrap_or_default().into()),
                exact: poetry_exactness(&specifier),
                specifier,
                written: Written(written.into()),
                kind,
                optional,
                origin,
                reference: table
                    .path
                    .map_or(Reference::Named, |path| locating(directory, Part(&path))),
            }
        })
        .collect()
}

impl From<ManifestSchema> for Declared {
    fn from(schema: ManifestSchema) -> Self {
        let poetry = schema
            .pyproject
            .tool
            .and_then(|tool| tool.poetry)
            .unwrap_or_default();
        let directory = Directory(&schema.directory);
        let mut all = Vec::new();
        let mut table = |dependencies: BTreeMap<String, toml::Value>, kind: DependencyKind| {
            for (name, dependency) in dependencies {
                if name != INTERPRETER {
                    all.extend(requirements(&name, dependency, kind, directory));
                }
            }
        };
        table(poetry.dependencies, DependencyKind::Normal);
        for (name, group) in poetry.group {
            let kind = if name == MAIN {
                DependencyKind::Normal
            } else {
                DependencyKind::Dev
            };
            table(group.dependencies, kind);
        }
        table(poetry.dev_dependencies, DependencyKind::Dev);
        Self {
            name: poetry.name.map_or(Naming::Unnamed, |name| {
                Naming::Named(PackageName(name.into()))
            }),
            members: Vec::new(),
            excludes: Vec::new(),
            requirements: all,
            sources: Vec::new(),
            overrides: Vec::new(),
        }
    }
}

/// What the Poetry tables of the text of the `pyproject.toml` at `manifest` declare.
///
/// # Errors
///
/// `Unreadable` when the text is not TOML.
pub(super) fn declared(
    manifest: &ManifestPath,
    text: &str,
) -> Result<Declared, ManifestReadingError> {
    let pyproject = toml::from_str(text).map_err(|error: toml::de::Error| {
        ManifestReadingError::Unreadable {
            reason: error.message().to_owned(),
        }
    })?;
    Ok(ManifestSchema {
        directory: directory_of(manifest).0.into(),
        pyproject,
    }
    .into())
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
