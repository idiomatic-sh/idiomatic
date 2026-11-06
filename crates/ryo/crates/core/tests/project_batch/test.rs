//! Real core sources are compiled intact; only an appended sibling probe varies.
use super::fixtures::{OWNER, PROBES};
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

#[test]
fn actual_project_batch_sealing_and_required_api() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let overlay = tempfile::tempdir().unwrap();
    copy_sources(&source.join("src"), &overlay.path().join("src")).unwrap();
    fs::copy(source.join("README.md"), overlay.path().join("README.md")).unwrap();
    fs::write(
        overlay.path().join("Cargo.toml"),
        toml::to_string(&manifest(source)).unwrap(),
    )
    .unwrap();
    fs::copy(
        source.join("../../Cargo.lock"),
        overlay.path().join("Cargo.lock"),
    )
    .unwrap();
    let original = fs::read_to_string(source.join(OWNER)).unwrap();
    let target = overlay.path().join("target");
    for probe in PROBES {
        fs::write(
            overlay.path().join(OWNER),
            format!("{original}\n{}", probe.client),
        )
        .unwrap();
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
        match probe.expected {
            None => {
                assert!(
                    output.status.success(),
                    "authorized {} failed: {errors:#?}; {}",
                    probe.name,
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(errors.is_empty());
            }
            Some((code, phrase)) => {
                assert!(
                    !output.status.success(),
                    "forbidden {} compiled",
                    probe.name
                );
                assert!(
                    !errors.is_empty(),
                    "{} yielded no Rust diagnostic: {}",
                    probe.name,
                    String::from_utf8_lossy(&output.stderr)
                );
                for error in errors {
                    assert_eq!(
                        error.code.as_ref().map(|code| code.code.as_str()),
                        Some(code),
                        "{}: {error:#?}",
                        probe.name
                    );
                    let rendered = error.rendered.as_deref().unwrap_or(&error.message);
                    assert!(rendered.contains(phrase), "{}: {rendered}", probe.name);
                    assert!(
                        error.spans.iter().any(|span| span.is_primary
                            && span.file_name.ends_with(OWNER)
                            && span.line_start > original.lines().count()),
                        "{} failed outside its actual-source client: {error:#?}",
                        probe.name
                    );
                }
            }
        }
    }
}
