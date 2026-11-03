//! Cross-language source identities remain portable across native platforms.
use super::*;
#[test]
fn source_identity_uses_utf8_name_order_on_every_platform() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/reference-discrepancy-canonical.json"
    )))
    .expect("cross-language encoding fixture");
    let root = tempfile::tempdir().expect("source boundary");
    let mut files = Vec::new();
    for (name, source) in fixture["source_files"].as_object().expect("source map") {
        let file = std::path::PathBuf::from(name);
        let target = root.path().join(&file);
        std::fs::create_dir_all(target.parent().expect("source parent")).expect("source directory");
        std::fs::write(target, source.as_str().expect("source UTF8")).expect("source bytes");
        files.push(file);
    }
    files.reverse();
    verify(CaseArtifacts {
        profile: root.path(),
        root: root.path(),
        files: &files,
        source_sha256: fixture["source_sha256"]
            .as_str()
            .expect("Python source digest"),
        file_metadata_files: &[],
        file_metadata_sha256: None,
        evidence: &[],
    })
    .expect("identical UTF8 digest with BMP and supplementary-plane filenames");
}

#[cfg(unix)]
#[test]
fn artifact_binding_recomputes_actual_metadata_in_both_directions() {
    use std::os::unix::fs::PermissionsExt as _;
    for expected in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let files = vec![std::path::PathBuf::from("case.py")];
        let file = root.path().join(&files[0]);
        std::fs::write(&file, b"same bytes").unwrap();
        let mode = if expected { 0o744 } else { 0o644 };
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).unwrap();
        let source = source_digest(root.path(), &files).unwrap();
        let metadata = crate::outbound::suite::adapters::local::input_metadata::actual_digest(
            root.path(),
            &files,
        )
        .unwrap();
        let check = || {
            verify(CaseArtifacts {
                profile: root.path(),
                root: root.path(),
                files: &files,
                source_sha256: &source,
                file_metadata_files: &files,
                file_metadata_sha256: Some(&metadata),
                evidence: &[],
            })
        };
        check().unwrap();
        std::fs::set_permissions(
            &file,
            std::fs::Permissions::from_mode(if expected { 0o644 } else { 0o744 }),
        )
        .unwrap();
        assert!(check().is_err());
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).unwrap();
        check().unwrap();
    }
}

#[cfg(unix)]
#[test]
fn optional_partial_observation_scope_tracks_only_declared_files_without_losing_source_identity() {
    use std::os::unix::fs::PermissionsExt as _;
    for before in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let files = vec![
            std::path::PathBuf::from("observed.py"),
            std::path::PathBuf::from("unobserved.py"),
        ];
        for file in &files {
            std::fs::write(root.path().join(file), b"same bytes").unwrap();
        }
        let mode = if before { 0o744 } else { 0o644 };
        std::fs::set_permissions(
            root.path().join(&files[0]),
            std::fs::Permissions::from_mode(mode),
        )
        .unwrap();
        let scope = vec![files[0].clone()];
        let source = source_digest(root.path(), &files).unwrap();
        let metadata = crate::outbound::suite::adapters::local::input_metadata::actual_digest(
            root.path(),
            &scope,
        )
        .unwrap();
        let check = |scope: &[std::path::PathBuf]| {
            verify(CaseArtifacts {
                profile: root.path(),
                root: root.path(),
                files: &files,
                source_sha256: &source,
                file_metadata_files: scope,
                file_metadata_sha256: Some(&metadata),
                evidence: &[],
            })
        };
        check(&scope).unwrap();
        for other in [0o644, 0o744] {
            std::fs::set_permissions(
                root.path().join(&files[1]),
                std::fs::Permissions::from_mode(other),
            )
            .unwrap();
            check(&scope).unwrap();
        }
        std::fs::set_permissions(
            root.path().join(&files[0]),
            std::fs::Permissions::from_mode(if before { 0o644 } else { 0o744 }),
        )
        .unwrap();
        assert!(check(&scope).is_err());
        std::fs::set_permissions(
            root.path().join(&files[0]),
            std::fs::Permissions::from_mode(mode),
        )
        .unwrap();
        check(&scope).unwrap();
        assert!(check(&[]).is_err(), "the declared subset cannot disappear");
        assert!(check(&[files[0].clone(), files[0].clone()]).is_err());
        assert!(check(&[std::path::PathBuf::from("undeclared.py")]).is_err());
        std::fs::write(root.path().join(&files[1]), b"changed bytes").unwrap();
        assert!(
            check(&scope).is_err(),
            "source identity still covers unobserved file bytes"
        );
    }
}
