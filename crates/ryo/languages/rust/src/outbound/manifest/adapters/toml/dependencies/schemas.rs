//! The dependencies of a `Cargo.toml`, as the manifest's tables write them, and their conversion into the port's own types.

use std::collections::BTreeMap;

use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};
use serde::Deserialize;

use crate::domain::paths::types::Section;
use crate::domain::targets::types::BindingName;
use crate::domain::targets::types::Checkout;
use crate::domain::targets::types::DependencySection;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::Patch;
use crate::domain::targets::types::Platform;
use crate::domain::targets::types::Replacement;
use crate::domain::targets::types::Revision;
use crate::domain::targets::types::Source;
use crate::domain::targets::types::Version;
use crate::domain::targets::types::WorkspaceDependency;
use crate::domain::targets::types::WrittenPath;

use super::super::constants::{CRATES_IO, REPLACED_VERSION};

/// A dependency: a version alone, or a table.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(in super::super) enum DependencySchema {
    /// `name = "1.2"`: a registry dependency by version alone.
    Version(String),
    /// `name = { .. }`: a dependency with its source, rename and flags.
    Detailed(DetailedSchema),
}

/// A dependency written as a table.
#[derive(Debug, Default, Deserialize)]
pub(in super::super) struct DetailedSchema {
    /// The version requirement.
    version: Option<String>,
    /// The directory of a dependency taken from the file system.
    path: Option<String>,
    /// The repository URL of a dependency taken from git.
    git: Option<String>,
    /// The branch of the git repository the dependency follows.
    branch: Option<String>,
    /// The tag of the git repository the dependency is taken at.
    tag: Option<String>,
    /// The revision of the git repository the dependency is taken at.
    rev: Option<String>,
    /// The registry a dependency comes from, when not crates.io.
    registry: Option<String>,
    /// The package's real name, when the dependency is renamed.
    package: Option<String>,
    /// Whether the dependency is only built behind a feature.
    optional: Option<bool>,
    /// Whether the dependency is taken from the workspace's `[workspace.dependencies]`.
    workspace: Option<bool>,
}

/// Where a dependency written as a table is taken from when it names no directory: its git repository, else its registry, else
/// crates.io.
fn source_of(each: &DetailedSchema) -> Source {
    Source(
        each.git
            .as_deref()
            .or(each.registry.as_deref())
            .unwrap_or(CRATES_IO)
            .into(),
    )
}

/// Where a dependency written as a table says its package comes from: its directory, else its git repository, else a registry.
fn origin_of(each: &DetailedSchema) -> DependencyOrigin {
    if each.path.is_some() {
        DependencyOrigin::Path
    } else if each.git.is_some() {
        DependencyOrigin::Git
    } else {
        DependencyOrigin::Registry
    }
}

/// The revision of its git repository a dependency written as a table names, by the key it writes it under.
fn checkout_of(each: &DetailedSchema) -> Checkout {
    Checkout {
        branch: text(each.branch.clone(), Revision),
        tag: text(each.tag.clone(), Revision),
        rev: text(each.rev.clone(), Revision),
    }
}

/// The packages `[patch]` and `[replace]` put in directories or take from git repositories.
pub(in super::super) fn patches(
    patch: BTreeMap<String, BTreeMap<String, DependencySchema>>,
    replace: BTreeMap<String, DependencySchema>,
) -> Vec<Patch> {
    let mut found = Vec::new();
    for (source, table) in patch {
        for (key, schema) in table {
            if let DependencySchema::Detailed(each) = schema
                && let Some(replacement) = replacement_of(each.path, each.git)
            {
                found.push(Patch {
                    source: Source(source.clone().into()),
                    package: PackageName(each.package.unwrap_or(key).into()),
                    replacement,
                });
            }
        }
    }
    // A `[replace]` key is `name:version` of a crates.io package.
    for (key, schema) in replace {
        if let DependencySchema::Detailed(each) = schema
            && let Some(replacement) = replacement_of(each.path, each.git)
        {
            let name = key
                .split_once(REPLACED_VERSION)
                .map_or(key.as_str(), |(name, _)| name);
            found.push(Patch {
                source: Source(CRATES_IO.into()),
                package: PackageName(name.into()),
                replacement,
            });
        }
    }
    found
}

/// Where a patch takes its package from: the directory it names, else the git repository; none when it names neither.
fn replacement_of(path: Option<String>, git: Option<String>) -> Option<Replacement> {
    path.map(|path| Replacement::Directory(WrittenPath(path.into())))
        .or_else(|| git.map(|url| Replacement::Repository(Source(url.into()))))
}

/// The dependencies of one table, each as the port has it.
pub(in super::super) fn dependencies(
    table: BTreeMap<String, DependencySchema>,
    kind: DependencyKind,
    platform: &Section<Platform>,
) -> Vec<DependencySection> {
    table
        .into_iter()
        .map(|(key, schema)| match schema {
            DependencySchema::Version(version) => DependencySection {
                key: BindingName(key.into()),
                kind,
                platform: platform.clone(),
                version: Section::Present(Version(version.into())),
                path: Section::Absent,
                package: Section::Absent,
                source: Source(CRATES_IO.into()),
                origin: DependencyOrigin::Registry,
                optional: Optionality::Required,
                workspace: Inheritance::Own,
                checkout: Checkout::default(),
            },
            DependencySchema::Detailed(each) => DependencySection {
                key: BindingName(key.into()),
                kind,
                platform: platform.clone(),
                source: source_of(&each),
                origin: origin_of(&each),
                checkout: checkout_of(&each),
                version: text(each.version, Version),
                path: text(each.path, WrittenPath),
                package: text(each.package, PackageName),
                optional: chosen(
                    each.optional,
                    true,
                    Optionality::Optional,
                    Optionality::Required,
                ),
                workspace: chosen(
                    each.workspace,
                    true,
                    Inheritance::Workspace,
                    Inheritance::Own,
                ),
            },
        })
        .collect()
}

/// The dependencies of the `[workspace.dependencies]` table, each as the port has it.
pub(in super::super) fn workspace_dependencies(
    table: BTreeMap<String, DependencySchema>,
) -> Vec<WorkspaceDependency> {
    table
        .into_iter()
        .map(|(key, schema)| match schema {
            DependencySchema::Version(version) => WorkspaceDependency {
                key: BindingName(key.into()),
                version: Section::Present(Version(version.into())),
                path: Section::Absent,
                package: Section::Absent,
                source: Source(CRATES_IO.into()),
                origin: DependencyOrigin::Registry,
                checkout: Checkout::default(),
            },
            DependencySchema::Detailed(each) => WorkspaceDependency {
                key: BindingName(key.into()),
                source: source_of(&each),
                origin: origin_of(&each),
                checkout: checkout_of(&each),
                version: text(each.version, Version),
                path: text(each.path, WrittenPath),
                package: text(each.package, PackageName),
            },
        })
        .collect()
}

/// A part that may be absent, as the type that holds it.
fn section<T>(value: Option<T>) -> Section<T> {
    value.map_or(Section::Absent, Section::Present)
}

/// A part that may be absent and is text, as the type that names the text.
fn text<T>(value: Option<String>, make: fn(Box<str>) -> T) -> Section<T> {
    section(value.map(|each| make(each.into())))
}

/// `yes` when the manifest gives `flag` the value `wanted`, else `no`.
fn chosen<T>(flag: Option<bool>, wanted: bool, yes: T, no: T) -> T {
    if flag == Some(wanted) { yes } else { no }
}
