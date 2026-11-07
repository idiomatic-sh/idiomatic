//! A production-source insertion and its precise Rust diagnostic contract.
use cargo_metadata::DependencyKind;
use cargo_metadata::MetadataCommand;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub(super) enum Compilation {
    Library,
    LibraryTests,
    WantedMutation,
}
pub(super) struct Probe {
    pub(super) compilation: Compilation,
    pub(super) owner: &'static str,
    pub(super) source: String,
    pub(super) expected: Expectation,
}
pub(super) enum Expectation {
    Accept,
    Refuse {
        code: &'static str,
        first: &'static str,
        second: &'static str,
    },
}
pub(super) fn copy_sources(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_sources(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
/// Cargo's resolved dependency descriptions preserve workspace inheritance,
/// renames and feature selections without copying the workspace or token types.
pub(super) fn manifest(source: &Path) -> toml::Table {
    let metadata = MetadataCommand::new()
        .manifest_path(source.join("Cargo.toml"))
        .no_deps()
        .other_options(vec!["--offline".into(), "--locked".into()])
        .exec()
        .unwrap();
    let package = metadata
        .packages
        .iter()
        .find(|package| package.manifest_path.as_std_path() == source.join("Cargo.toml"))
        .unwrap();
    let original: toml::Table = fs::read_to_string(source.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let workspace: toml::Table = fs::read_to_string(metadata.workspace_root.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let mut result = toml::Table::new();
    let mut identity = toml::Table::new();
    identity.insert("name".into(), package.name.to_string().into());
    identity.insert("version".into(), package.version.to_string().into());
    identity.insert("edition".into(), package.edition.to_string().into());
    identity.insert(
        "description".into(),
        package.description.clone().unwrap_or_default().into(),
    );
    identity.insert("autotests".into(), false.into());
    result.insert("package".into(), identity.into());
    let mut resolution = toml::Table::new();
    resolution.insert(
        "resolver".into(),
        workspace["workspace"]["resolver"].clone(),
    );
    result.insert("workspace".into(), resolution.into());
    result.insert("lib".into(), original["lib"].clone());
    result.insert("features".into(), original["features"].clone());
    result.insert("lints".into(), workspace["workspace"]["lints"].clone());
    let mut dependencies = toml::Table::new();
    let mut development = toml::Table::new();
    for dependency in &package.dependencies {
        let destination = match dependency.kind {
            DependencyKind::Normal => &mut dependencies,
            // The crate's own dev-dependency (its tests' `fixtures` feature) would make the overlay
            // depend on the production copy of itself.
            DependencyKind::Development if dependency.name == package.name.to_string() => continue,
            DependencyKind::Development => &mut development,
            _ => continue,
        };
        let mut entry = toml::Table::new();
        entry.insert("version".into(), dependency.req.to_string().into());
        entry.insert("package".into(), dependency.name.clone().into());
        if dependency.kind == DependencyKind::Normal {
            entry.insert("optional".into(), dependency.optional.into());
        }
        entry.insert(
            "default-features".into(),
            dependency.uses_default_features.into(),
        );
        entry.insert(
            "features".into(),
            toml::Value::Array(
                dependency
                    .features
                    .iter()
                    .cloned()
                    .map(Into::into)
                    .collect(),
            ),
        );
        if let Some(path) = &dependency.path {
            entry.insert("path".into(), path.to_string().into());
        }
        destination.insert(
            dependency
                .rename
                .as_ref()
                .unwrap_or(&dependency.name)
                .clone(),
            entry.into(),
        );
    }
    result.insert("dependencies".into(), dependencies.into());
    result.insert("dev-dependencies".into(), development.into());
    result
}
pub(super) const ADMISSION: &str = "src/application/file_dag/program/admission.rs";
pub(super) const PROGRAM: &str = "src/application/file_dag/program.rs";
pub(super) const RUNTIME_MODULE: &str = "src/application/file_dag/program/runtime/mod.rs";
pub(super) const RUNTIME_TYPES: &str = "src/application/file_dag/program/runtime/types.rs";
pub(super) const RELATIONS: &str = "src/application/file_dag/relations.rs";
pub(super) const REQUEST: &str = "src/inbound/use_cases/execution/use_case.rs";
pub(super) const RELATION_ISSUER: &str = "src/application/file_dag/relations/admission/service.rs";
pub(super) const RUNTIME: &str = "src/application/file_dag/program/runtime/service.rs";
pub(super) const AUTHORIZED: &str = r#"
mod boundary_probe {
    use super::*;
    fn authorized<'a>(execution: &'a ProgramExecution, region: RegionId, inputs: &'a [AdmittedInput], left: &'a AdmittedInput) {
        let authority = || KernelAdmission { execution, region };
        let _ = crate::application::file_dag::program::runtime::service::KernelPermit::admitted(authority(), inputs);
        let _ = crate::application::file_dag::program::runtime::service::KernelPermit::ordered(authority(), left);
    }
    const _: for<'a> fn(&'a ProgramExecution, RegionId, &'a [AdmittedInput], &'a AdmittedInput) = authorized;
    fn assert_owned_result<T: Send + Sync + 'static>() {}
    fn owned_walk_result() { assert_owned_result::<crate::application::file_dag::program::native_walk::WalkResult>(); }
    const _: fn() = owned_walk_result;
}
"#;
pub(super) const SIBLING: &str = r#"
mod boundary_probe {
    use super::admission::AdmittedInput;
    use crate::application::file_dag::program::runtime::service::KernelPermit;
    use crate::application::file_dag::program::runtime::service::KernelAdmission;
    use crate::application::file_dag::program::runtime::service::ProgramExecution;
    use crate::domain::executable_program::types::RegionId;
"#;
