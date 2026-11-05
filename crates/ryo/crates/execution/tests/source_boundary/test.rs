//! Only probe insertion differs from production; no authority implementation is copied.
use cargo_metadata::{DependencyKind, Message, MetadataCommand, diagnostic::DiagnosticLevel};
use std::{fs, io, path::Path, process::Command};

fn copy_sources(source: &Path, target: &Path) -> io::Result<()> {
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
fn manifest(source: &Path) -> toml::Table {
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
    result.insert("workspace".into(), toml::Table::new().into());
    result.insert("lib".into(), original["lib"].clone());
    result.insert("features".into(), original["features"].clone());
    result.insert("lints".into(), workspace["workspace"]["lints"].clone());
    let mut dependencies = toml::Table::new();
    for dependency in package
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == DependencyKind::Normal)
    {
        let mut entry = toml::Table::new();
        entry.insert("version".into(), dependency.req.to_string().into());
        entry.insert("package".into(), dependency.name.clone().into());
        entry.insert("optional".into(), dependency.optional.into());
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
        dependencies.insert(
            dependency
                .rename
                .as_ref()
                .unwrap_or(&dependency.name)
                .clone(),
            entry.into(),
        );
    }
    result.insert("dependencies".into(), dependencies.into());
    result
}

const PROGRAM: &str = "src/application/file_dag/program.rs";
const RUNTIME: &str = "src/application/file_dag/program/runtime.rs";
const AUTHORIZED: &str = r#"
mod boundary_probe {
    use super::*;
    fn authorized<'a>(execution: &'a ProgramExecution, region: RegionId, inputs: &'a [AdmittedInput], left: &'a AdmittedInput, right: &'a AdmittedInput) {
        let authority = || KernelAdmission { execution, region };
        let _ = super::super::admission::KernelPermit::admitted(authority(), inputs);
        let _ = super::super::admission::KernelPermit::ordered(authority(), left);
        let _ = super::super::admission::KernelPermit::column(authority(), left, right);
    }
    const _: for<'a> fn(&'a ProgramExecution, RegionId, &'a [AdmittedInput], &'a AdmittedInput, &'a AdmittedInput) = authorized;
}
"#;
const SIBLING: &str = r#"
mod boundary_probe {
    use super::{admission::{AdmittedInput, KernelPermit}, runtime::{KernelAdmission, ProgramExecution}};
    use crate::domain::executable_program::types::RegionId;
"#;

#[test]
fn actual_runtime_authority_is_private_to_its_owner() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let overlay = tempfile::tempdir().unwrap();
    copy_sources(&source.join("src"), &overlay.path().join("src")).unwrap();
    fs::copy(source.join("README.md"), overlay.path().join("README.md")).unwrap();
    let manifest = manifest(source);
    let target = overlay.path().join("target");
    fs::write(
        overlay.path().join("Cargo.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    // Keep production's resolved versions while allowing Cargo to trim unused
    // workspace members from this temporary standalone manifest offline.
    fs::copy(
        source.join("../../Cargo.lock"),
        overlay.path().join("Cargo.lock"),
    )
    .unwrap();
    let program = fs::read_to_string(source.join(PROGRAM)).unwrap();
    let runtime = fs::read_to_string(source.join(RUNTIME)).unwrap();
    let probes = [
        (RUNTIME, AUTHORIZED.to_owned(), None),
        (
            PROGRAM,
            format!(
                "{SIBLING} fn forge(execution: &ProgramExecution, region: RegionId) {{ let _ = KernelAdmission {{ execution, region }}; }} const _: fn(&ProgramExecution, RegionId) = forge; }}"
            ),
            Some(("E0451", "execution", "region")),
        ),
        (
            PROGRAM,
            format!(
                "{SIBLING} fn admitted<'a>(execution: &'a ProgramExecution, inputs: &'a [AdmittedInput]) {{ let _ = KernelPermit::admitted(execution, inputs); }} const _: for<'a> fn(&'a ProgramExecution, &'a [AdmittedInput]) = admitted; }}"
            ),
            Some(("E0308", "KernelAdmission", "ProgramExecution")),
        ),
        (
            PROGRAM,
            format!(
                "{SIBLING} fn ordered<'a>(execution: &'a ProgramExecution, input: &'a AdmittedInput) {{ let _ = KernelPermit::ordered(execution, input); }} const _: for<'a> fn(&'a ProgramExecution, &'a AdmittedInput) = ordered; }}"
            ),
            Some(("E0308", "KernelAdmission", "ProgramExecution")),
        ),
        (
            PROGRAM,
            format!(
                "{SIBLING} fn column<'a>(execution: &'a ProgramExecution, input: &'a AdmittedInput) {{ let _ = KernelPermit::column(execution, input, input); }} const _: for<'a> fn(&'a ProgramExecution, &'a AdmittedInput) = column; }}"
            ),
            Some(("E0308", "KernelAdmission", "ProgramExecution")),
        ),
        (
            PROGRAM,
            format!(
                "{SIBLING} fn retarget(permit: KernelPermit<'_>, region: RegionId) {{ let _ = KernelPermit {{ region, ..permit }}; }} const _: for<'a> fn(KernelPermit<'a>, RegionId) = retarget; }}"
            ),
            Some(("E0451", "region", "private")),
        ),
    ];
    for (owner, probe, refusal) in probes {
        fs::write(overlay.path().join(PROGRAM), &program).unwrap();
        fs::write(overlay.path().join(RUNTIME), &runtime).unwrap();
        let original = if owner == PROGRAM { &program } else { &runtime };
        fs::write(overlay.path().join(owner), format!("{original}\n{probe}")).unwrap();
        let output = Command::new(env!("CARGO"))
            .args([
                "check",
                "--offline",
                "--lib",
                "--jobs",
                "2",
                "--message-format=json",
            ])
            .arg("--manifest-path")
            .arg(overlay.path().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .output()
            .unwrap();
        let errors = Message::parse_stream(output.stdout.as_slice())
            .map(Result::unwrap)
            .filter_map(|message| match message {
                Message::CompilerMessage(message)
                    if message.message.level == DiagnosticLevel::Error =>
                {
                    Some(message.message)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        match refusal {
            None => {
                assert!(
                    output.status.success(),
                    "authorized production control failed; parsed errors: {errors:#?}; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(errors.is_empty(), "{errors:#?}");
            }
            Some((code, first, second)) => {
                assert!(
                    !output.status.success(),
                    "forbidden sibling probe compiled: {probe}"
                );
                assert!(
                    !errors.is_empty(),
                    "no Rust diagnostic: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                for error in errors {
                    assert_eq!(
                        error.code.as_ref().map(|code| code.code.as_str()),
                        Some(code),
                        "{error:#?}"
                    );
                    let rendered = error.rendered.as_deref().unwrap_or(&error.message);
                    assert!(
                        rendered.contains(first) && rendered.contains(second),
                        "{rendered}"
                    );
                    assert!(
                        error.spans.iter().any(|span| span.is_primary
                            && span.file_name.ends_with(owner)
                            && span.line_start > original.lines().count()),
                        "{error:#?}"
                    );
                }
            }
        }
    }
}
