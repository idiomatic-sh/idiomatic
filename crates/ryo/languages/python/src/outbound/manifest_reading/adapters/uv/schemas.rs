//! The shape of the tables `uv` reads in a `pyproject.toml`, which the language does not control, and their conversion into the port's
//! own types: `[tool.uv.workspace]` for the members of a workspace and what it leaves out, `[tool.uv.sources]` for where a package
//! of the repository is found, and at which revision of a git repository, and `override-dependencies` for the requirements that take
//! the place of others.

use std::collections::BTreeMap;

use ryo::languages::DependencyOrigin;
use serde::Deserialize;

use crate::domain::packages::service::{directory_of, locating};
use crate::domain::packages::types::{
    Checkout, Declared, ManifestPath, Marker, Naming, PackageName, Redirect, Reference, Revision,
    Written,
};
use crate::domain::patterns::types::{Directory, PatternPart, Pattern};
use crate::outbound::manifest_reading::errors::ManifestReadingError;

/// A `pyproject.toml`.
#[derive(Debug, Default, Deserialize)]
// the shape is the tables of pyproject.toml, read table for table: a flatter shape reads a different file
struct PyprojectSchema {
    /// The `[tool]` table, which holds uv's.
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
// the shape is the tables of pyproject.toml, read table for table: a flatter shape reads a different file
struct ToolSchema {
    /// The `[tool.uv]` table.
    uv: Option<UvSchema>,
}

/// A `[tool.uv]`.
#[derive(Debug, Default, Deserialize)]
struct UvSchema {
    /// The workspace the project roots, when it roots one.
    workspace: Option<WorkspaceSchema>,
    /// Where each package comes from, by name.
    #[serde(default)]
    sources: BTreeMap<String, SourceSchema>,
    /// The requirements that take the place of every requirement of the same name, as written.
    #[serde(default, rename = "override-dependencies")]
    override_dependencies: Vec<String>,
}

/// A `[tool.uv.workspace]`.
#[derive(Debug, Default, Deserialize)]
struct WorkspaceSchema {
    /// The globs of the workspace's member directories.
    #[serde(default)]
    members: Vec<String>,
    /// The globs of the directories the members leave out.
    #[serde(default)]
    exclude: Vec<String>,
}

/// The source of a package: one, or one for each marker.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SourceSchema {
    /// One source for every environment.
    One(SourceEntrySchema),
    /// One source for each environment marker.
    Each(Vec<SourceEntrySchema>),
}

/// One source. A `workspace = true` source names a member, which the requirement's own name finds; `path` sends a requirement to a
/// directory, or, outside the repository, away from every package of it; a `git`, `url` or `index` source names no directory of it.
#[derive(Debug, Default, Deserialize)]
struct SourceEntrySchema {
    /// A directory the package comes from.
    path: Option<String>,
    /// Whether the package is a member of the workspace.
    workspace: Option<bool>,
    /// A git repository the package comes from.
    git: Option<String>,
    /// The branch of the git repository the package follows.
    branch: Option<String>,
    /// The tag of the git repository the package is taken at.
    tag: Option<String>,
    /// The revision of the git repository the package is taken at.
    rev: Option<String>,
    /// A URL the package comes from.
    url: Option<String>,
    /// A package index the package comes from.
    index: Option<String>,
    /// The environment marker the source applies under.
    marker: Option<String>,
}

impl From<ManifestSchema> for Declared {
    fn from(schema: ManifestSchema) -> Self {
        let uv = schema
            .pyproject
            .tool
            .and_then(|tool| tool.uv)
            .unwrap_or_default();
        let workspace = uv.workspace.unwrap_or_default();
        let directory = Directory(&schema.directory);
        let mut sources = Vec::new();
        for (name, source) in uv.sources {
            let entries = match source {
                SourceSchema::One(entry) => vec![entry],
                SourceSchema::Each(entries) => entries,
            };
            for entry in entries {
                let origin = if entry.path.is_some() {
                    DependencyOrigin::Path
                } else if entry.workspace == Some(true) {
                    DependencyOrigin::Workspace
                } else if entry.git.is_some() {
                    DependencyOrigin::Git
                } else if entry.url.is_some() {
                    DependencyOrigin::Url
                } else if entry.index.is_some() {
                    DependencyOrigin::Registry
                } else {
                    // A source that says none of these sends the requirement nowhere: uv refuses it.
                    continue;
                };
                sources.push(Redirect {
                    name: PackageName(name.clone().into()),
                    location: entry
                        .path
                        .and_then(|path| match locating(directory, PatternPart(&path)) {
                            Reference::Located(location) => Some(location),
                            Reference::Named | Reference::Unresolvable => None,
                        }),
                    origin,
                    marker: Marker(entry.marker.unwrap_or_default().into()),
                    checkout: Checkout {
                        branch: Revision(entry.branch.unwrap_or_default().into()),
                        tag: Revision(entry.tag.unwrap_or_default().into()),
                        revision: Revision(entry.rev.unwrap_or_default().into()),
                    },
                });
            }
        }
        Self {
            name: Naming::Unnamed,
            members: workspace
                .members
                .into_iter()
                .map(|pattern| Pattern(pattern.into()))
                .collect(),
            excludes: workspace
                .exclude
                .into_iter()
                .map(|pattern| Pattern(pattern.into()))
                .collect(),
            requirements: Vec::new(),
            sources,
            overrides: uv
                .override_dependencies
                .into_iter()
                .map(|each| Written(each.into()))
                .collect(),
        }
    }
}

/// What the `uv` tables of the text of the `pyproject.toml` at `manifest` declare.
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
