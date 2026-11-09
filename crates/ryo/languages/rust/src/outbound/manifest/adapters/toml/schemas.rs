// one manifest wire format: the sections of Cargo.toml mirror one schema family (package, targets, dependencies, workspace, lints, config), converted together into one Manifest; the spec places one schemas.rs for an adapter.
//! The shape of a `Cargo.toml` and of a `.cargo/config.toml`, which the language does not control, and their conversion into the
//! port's own types.

use crate::domain::workspaces::types::Reason;
use std::collections::BTreeMap;

use ryo::languages::DependencyKind;
use serde::Deserialize;

use crate::domain::paths::types::Section;
use crate::domain::targets::types::Build;
use crate::domain::targets::types::Config;
use crate::domain::targets::types::Discovery;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::LibrarySection;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::PackageSection;
use crate::domain::targets::types::Pattern;
use crate::domain::targets::types::Platform;
use crate::domain::targets::types::TargetName;
use crate::domain::targets::types::TargetSection;
use crate::domain::targets::types::WorkspaceSection;
use crate::domain::targets::types::WrittenPath;
use crate::outbound::manifest::errors::ManifestError;

use super::dependencies::schemas::{
    DependencySchema, dependencies, patches, workspace_dependencies,
};

/// A `Cargo.toml`.
#[derive(Debug, Default, Deserialize)]
// the shape is the tables of Cargo.toml, read table for table: a flatter shape reads a different file
pub(super) struct ManifestSchema {
    /// The `[package]` table, absent from a virtual workspace manifest.
    package: Option<PackageSchema>,
    /// The `[workspace]` table, present on a workspace root.
    workspace: Option<WorkspaceSchema>,
    /// The `[lib]` table, which may move the library's root file.
    lib: Option<LibrarySchema>,
    /// The `[[bin]]` targets the manifest names.
    #[serde(default)]
    bin: Vec<TargetSchema>,
    /// The `[[example]]` targets the manifest names.
    #[serde(default)]
    example: Vec<TargetSchema>,
    /// The `[[test]]` targets the manifest names.
    #[serde(default)]
    test: Vec<TargetSchema>,
    /// The `[[bench]]` targets the manifest names.
    #[serde(default)]
    bench: Vec<TargetSchema>,
    /// The `[dependencies]`, by the name the package uses them under.
    #[serde(default)]
    dependencies: BTreeMap<String, DependencySchema>,
    /// The `[dev-dependencies]`, by the name the package uses them under.
    #[serde(default, rename = "dev-dependencies", alias = "dev_dependencies")]
    dev_dependencies: BTreeMap<String, DependencySchema>,
    /// The `[build-dependencies]`, by the name the package uses them under.
    #[serde(default, rename = "build-dependencies", alias = "build_dependencies")]
    build_dependencies: BTreeMap<String, DependencySchema>,
    /// The `[target.<cfg>]` tables of platform-specific dependencies, by their condition.
    #[serde(default)]
    target: BTreeMap<String, PlatformSchema>,
    /// The `[lints]` table, read only for whether it inherits the workspace's.
    lints: Option<LintsSchema>,
    /// The `[patch.<source>]` tables, which put a dependency of a source somewhere else.
    #[serde(default)]
    patch: BTreeMap<String, BTreeMap<String, DependencySchema>>,
    /// The `[replace]` table, the older way to put a dependency somewhere else.
    #[serde(default)]
    replace: BTreeMap<String, DependencySchema>,
}

/// A `[package]`.
#[derive(Debug, Default, Deserialize)]
struct PackageSchema {
    /// The package's name, empty when the manifest left it out.
    #[serde(default)]
    name: String,
    /// The path of the workspace root the package names explicitly.
    workspace: Option<String>,
    /// Where the build script is, or whether there is one.
    build: Option<BuildSchema>,
    /// Whether Cargo discovers the library at its default path.
    autolib: Option<bool>,
    /// Whether Cargo discovers binaries under `src/bin/`.
    autobins: Option<bool>,
    /// Whether Cargo discovers examples under `examples/`.
    autoexamples: Option<bool>,
    /// Whether Cargo discovers integration tests under `tests/`.
    autotests: Option<bool>,
    /// Whether Cargo discovers benches under `benches/`.
    autobenches: Option<bool>,
    /// Every other key of the package, kept to tell which values are inherited from the workspace.
    #[serde(flatten)]
    rest: BTreeMap<String, toml::Value>,
}

/// How a package says its build script is.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BuildSchema {
    /// `build = true` or `false`: the default script, or none.
    Enabled(bool),
    /// `build = "path"`: the script at that path.
    At(String),
}

/// A `[lib]`.
#[derive(Debug, Default, Deserialize)]
struct LibrarySchema {
    /// The library's root file, when not `src/lib.rs`.
    path: Option<String>,
    /// The library's crate name, when not the package's.
    name: Option<String>,
}

/// A `[[bin]]`, `[[example]]`, `[[test]]` or `[[bench]]`.
#[derive(Debug, Default, Deserialize)]
struct TargetSchema {
    /// The target's name, defaulted from its file when absent.
    name: Option<String>,
    /// The target's root file, defaulted from its name when absent.
    path: Option<String>,
}

/// The dependencies of one platform.
#[derive(Debug, Default, Deserialize)]
struct PlatformSchema {
    /// The platform's `[dependencies]`.
    #[serde(default)]
    dependencies: BTreeMap<String, DependencySchema>,
    /// The platform's `[dev-dependencies]`.
    #[serde(default, rename = "dev-dependencies", alias = "dev_dependencies")]
    dev_dependencies: BTreeMap<String, DependencySchema>,
    /// The platform's `[build-dependencies]`.
    #[serde(default, rename = "build-dependencies", alias = "build_dependencies")]
    build_dependencies: BTreeMap<String, DependencySchema>,
}

/// A `[workspace]`.
#[derive(Debug, Default, Deserialize)]
struct WorkspaceSchema {
    /// The globs of the member packages' directories.
    #[serde(default)]
    members: Vec<String>,
    /// The globs of directories that are not members.
    #[serde(default)]
    exclude: Vec<String>,
    /// The `[workspace.dependencies]` members inherit with `workspace = true`.
    #[serde(default)]
    dependencies: BTreeMap<String, DependencySchema>,
}

/// A `[lints]`.
#[derive(Debug, Default, Deserialize)]
struct LintsSchema {
    /// Whether the package takes its lints from the workspace.
    workspace: Option<bool>,
}

/// A `.cargo/config.toml`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct ConfigSchema {
    /// The `[build]` table, read for where the build output goes.
    build: Option<BuildConfigSchema>,
}

/// The `[build]` of a configuration.
#[derive(Debug, Default, Deserialize)]
struct BuildConfigSchema {
    /// The `target-dir` the configuration moves the build output to.
    #[serde(default, rename = "target-dir")]
    target_dir: Option<String>,
}

/// A part that may be absent.
fn section<T>(value: Option<T>) -> Section<T> {
    value.map_or(Section::Absent, Section::Present)
}

/// A part that may be absent and is text, as the type that names the text.
fn text<T>(value: Option<String>, make: fn(Box<str>) -> T) -> Section<T> {
    section(value.map(|each| make(each.into())))
}

/// A target table as the port has it.
fn target(schema: TargetSchema) -> TargetSection {
    TargetSection {
        name: text(schema.name, TargetName),
        path: text(schema.path, WrittenPath),
    }
}

/// A value of the package, as far as whether it is taken from the workspace is concerned: `version.workspace = true`.
#[derive(Debug, Default, Deserialize)]
struct InheritedSchema {
    /// `true` when the value is taken from the workspace.
    workspace: Option<bool>,
}

/// `yes` when the manifest gives `flag` the value `wanted`, else `no`.
fn chosen<T>(flag: Option<bool>, wanted: bool, yes: T, no: T) -> T {
    if flag == Some(wanted) { yes } else { no }
}

/// Whether a value of the package is taken from the workspace.
fn inherited(value: &toml::Value) -> bool {
    value
        .clone()
        .try_into::<InheritedSchema>()
        .is_ok_and(|each| each.workspace == Some(true))
}

/// Why a manifest or a configuration is refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ManifestSchemaError {
    /// It has neither a `[package]` nor a `[workspace]`.
    #[error("no `package` or `workspace` section")]
    Neither,
    /// Its `[package]` has no name.
    #[error("the package has no name")]
    Unnamed,
    /// A configuration names an empty directory for the outputs.
    #[error("`build.target-dir` is empty")]
    EmptyTargetDir,
}

/// The `[workspace]` of a manifest: its members, what it leaves out and the dependencies its members inherit.
fn workspace_section(workspace: WorkspaceSchema) -> WorkspaceSection {
    WorkspaceSection {
        members: workspace
            .members
            .into_iter()
            .map(|each| Pattern(each.into()))
            .collect(),
        exclude: workspace
            .exclude
            .into_iter()
            .map(|each| Pattern(each.into()))
            .collect(),
        dependencies: workspace_dependencies(workspace.dependencies),
    }
}

impl TryFrom<ManifestSchema> for Manifest {
    type Error = ManifestSchemaError;

    fn try_from(schema: ManifestSchema) -> Result<Self, Self::Error> {
        if schema.package.is_none() && schema.workspace.is_none() {
            return Err(ManifestSchemaError::Neither);
        }
        if schema
            .package
            .as_ref()
            .is_some_and(|package| package.name.is_empty())
        {
            return Err(ManifestSchemaError::Unnamed);
        }
        let everywhere = Section::Absent;
        let mut found = dependencies(schema.dependencies, DependencyKind::Normal, &everywhere);
        found.extend(dependencies(
            schema.dev_dependencies,
            DependencyKind::Dev,
            &everywhere,
        ));
        found.extend(dependencies(
            schema.build_dependencies,
            DependencyKind::Build,
            &everywhere,
        ));
        for (platform, each) in schema.target {
            let platform = Section::Present(Platform(platform.into()));
            found.extend(dependencies(
                each.dependencies,
                DependencyKind::Normal,
                &platform,
            ));
            found.extend(dependencies(
                each.dev_dependencies,
                DependencyKind::Dev,
                &platform,
            ));
            found.extend(dependencies(
                each.build_dependencies,
                DependencyKind::Build,
                &platform,
            ));
        }
        let inherits = found
            .iter()
            .any(|each| each.workspace == Inheritance::Workspace)
            || schema
                .lints
                .is_some_and(|lints| lints.workspace == Some(true))
            || schema
                .package
                .as_ref()
                .is_some_and(|package| package.rest.values().any(inherited));
        let inherits = chosen(
            Some(inherits),
            true,
            Inheritance::Workspace,
            Inheritance::Own,
        );
        let automatic =
            |flag: Option<bool>| chosen(flag, false, Discovery::Declared, Discovery::Automatic);
        Ok(Self {
            package: section(schema.package.map(|package| PackageSection {
                name: PackageName(package.name.into()),
                workspace: text(package.workspace, WrittenPath),
                build: match package.build {
                    None | Some(BuildSchema::Enabled(true)) => Build::Found,
                    Some(BuildSchema::Enabled(false)) => Build::Disabled,
                    Some(BuildSchema::At(path)) => Build::At(WrittenPath(path.into())),
                },
                autolib: automatic(package.autolib),
                autobins: automatic(package.autobins),
                autoexamples: automatic(package.autoexamples),
                autotests: automatic(package.autotests),
                autobenches: automatic(package.autobenches),
            })),
            workspace: section(schema.workspace.map(workspace_section)),
            lib: section(schema.lib.map(|lib| LibrarySection {
                path: text(lib.path, WrittenPath),
                name: text(lib.name, TargetName),
            })),
            bins: schema.bin.into_iter().map(target).collect(),
            examples: schema.example.into_iter().map(target).collect(),
            tests: schema.test.into_iter().map(target).collect(),
            benches: schema.bench.into_iter().map(target).collect(),
            dependencies: found,
            patches: patches(schema.patch, schema.replace),
            inherits,
        })
    }
}

impl TryFrom<ConfigSchema> for Config {
    type Error = ManifestSchemaError;

    fn try_from(schema: ConfigSchema) -> Result<Self, Self::Error> {
        let target_dir = schema.build.and_then(|build| build.target_dir);
        if target_dir.as_ref().is_some_and(String::is_empty) {
            return Err(ManifestSchemaError::EmptyTargetDir);
        }
        Ok(Self {
            target_dir: text(target_dir, WrittenPath),
        })
    }
}

impl TryFrom<&str> for Manifest {
    type Error = ManifestError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let schema: ManifestSchema =
            toml::from_str(text).map_err(|error| ManifestError::Unreadable {
                reason: Reason(error.message().into()),
            })?;
        Self::try_from(schema).map_err(|error| ManifestError::Unreadable {
            reason: Reason(error.to_string().into()),
        })
    }
}

impl TryFrom<&str> for Config {
    type Error = ManifestError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let schema: ConfigSchema =
            toml::from_str(text).map_err(|error| ManifestError::Unreadable {
                reason: Reason(error.message().into()),
            })?;
        Self::try_from(schema).map_err(|error| ManifestError::Unreadable {
            reason: Reason(error.to_string().into()),
        })
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
