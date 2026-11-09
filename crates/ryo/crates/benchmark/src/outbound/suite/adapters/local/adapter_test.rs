//! Actual physical suite observations, exact digest bytes and profile decoding.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::fixtures::contract::check;
use crate::outbound::suite::{
    factory::local,
    interface::SuitePort as _,
    types::{ArtifactMetadata, ArtifactPath, ArtifactSet, CaseArtifacts},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;

/// Whether the artifacts were refused as not matching their declared identity.
fn invalid(result: Result<(), SuiteError>) -> bool {
    matches!(result, Err(SuiteError::Invalid(_)))
}
fn physical_contract() {
    check(&local()).unwrap();
}
fn exact_executable_digest_bytes() {
    let directory = tempfile::tempdir().expect("source artifacts");
    let bytes = directory.path().join("executable");
    std::fs::write(&bytes, b"abc").expect("digest fixture");
    assert_eq!(
        local().digest(ArtifactPath(&bytes)).expect("digest").0,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(
        matches!(local().digest(ArtifactPath(directory.path())), Err(ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message)) if message == "executable digest requires a regular file")
    );
}
fn optional_source_language_profile_preserves_default_and_validates_exact_names() {
    let directory = tempfile::tempdir().expect("profile");
    let base = "tool = 'fixture'\nversion = '1'\nsha256 = ['pin']\ndefaults = ['F401']\n";
    for (suffix, expected) in [("", None), ("source-language = 'python'\n", Some("python"))] {
        std::fs::write(
            directory.path().join("benchmark.toml"),
            format!("{base}{suffix}"),
        )
        .expect("profile bytes");
        let profile = local()
            .profile(ArtifactPath(directory.path()))
            .expect("profile");
        assert_eq!(
            match &profile.source_language {
                ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(name) => Some(name.0.as_str()),
                ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified => None,
            },
            expected
        );
    }
    for bad in ["", " python", "python "] {
        std::fs::write(
            directory.path().join("benchmark.toml"),
            format!("{base}source-language = '{bad}'\n"),
        )
        .expect("profile bytes");
        assert!(matches!(
            local().profile(ArtifactPath(directory.path())),
            Err(_refused_profile)
        ));
    }
}

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
    local()
        .verify_discrepancy_artifacts(CaseArtifacts {
            profile: root.path(),
            root: root.path(),
            files: &files,
            source_sha256: &ArtifactDigest(
                fixture["source_sha256"]
                    .as_str()
                    .expect("Python source digest")
                    .to_owned(),
            ),
            file_metadata_files: &[],
            file_metadata: ArtifactMetadata::Undeclared,
            evidence: &[],
        })
        .expect("identical UTF8 digest with BMP and supplementary-plane filenames");
}
#[cfg(unix)]
fn artifact_binding_recomputes_actual_metadata_in_both_directions() {
    for expected in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let files = vec![std::path::PathBuf::from("case.py")];
        let file = root.path().join(&files[0]);
        std::fs::write(&file, b"same bytes").unwrap();
        let mode = if expected { 0o744 } else { 0o644 };
        set_mode(&file, mode);
        let source = ArtifactDigest(
            local()
                .case_source_digest(ArtifactPath(root.path()), ArtifactSet(&files))
                .unwrap()
                .0,
        );
        let metadata = ArtifactDigest(
            crate::outbound::suite::adapters::local::file_metadata::schemas::actual_digest(
                root.path(),
                &files,
            )
            .unwrap(),
        );
        let check = || {
            local().verify_discrepancy_artifacts(CaseArtifacts {
                profile: root.path(),
                root: root.path(),
                files: &files,
                source_sha256: &source,
                file_metadata_files: &files,
                file_metadata: ArtifactMetadata::Declared(&metadata),
                evidence: &[],
            })
        };
        check().unwrap();
        set_mode(&file, if expected { 0o644 } else { 0o744 });
        assert!(invalid(check()));
        set_mode(&file, mode);
        check().unwrap();
    }
}
#[cfg(unix)]
fn optional_partial_observation_scope_tracks_only_declared_files_without_losing_source_identity() {
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
        set_mode(&root.path().join(&files[0]), mode);
        let scope = vec![files[0].clone()];
        let source = ArtifactDigest(
            local()
                .case_source_digest(ArtifactPath(root.path()), ArtifactSet(&files))
                .unwrap()
                .0,
        );
        let metadata = ArtifactDigest(
            crate::outbound::suite::adapters::local::file_metadata::schemas::actual_digest(
                root.path(),
                &scope,
            )
            .unwrap(),
        );
        let check = |scope: &[std::path::PathBuf]| {
            local().verify_discrepancy_artifacts(CaseArtifacts {
                profile: root.path(),
                root: root.path(),
                files: &files,
                source_sha256: &source,
                file_metadata_files: scope,
                file_metadata: ArtifactMetadata::Declared(&metadata),
                evidence: &[],
            })
        };
        check(&scope).unwrap();
        for other in [0o644, 0o744] {
            set_mode(&root.path().join(&files[1]), other);
            check(&scope).unwrap();
        }
        set_mode(
            &root.path().join(&files[0]),
            if before { 0o644 } else { 0o744 },
        );
        assert!(invalid(check(&scope)));
        set_mode(&root.path().join(&files[0]), mode);
        check(&scope).unwrap();
        assert!(invalid(check(&[])), "the declared subset cannot disappear");
        assert!(invalid(check(&[files[0].clone(), files[0].clone()])));
        assert!(invalid(check(&[std::path::PathBuf::from("undeclared.py")])));
        std::fs::write(root.path().join(&files[1]), b"changed bytes").unwrap();
        assert!(
            invalid(check(&scope)),
            "source identity still covers unobserved file bytes"
        );
    }
}
/// Set the permission bits of a file.
#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
/// Platforms without permission bits keep the file as written.
#[cfg(not(unix))]
fn set_mode(_path: &std::path::Path, _mode: u32) {}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("physical_contract"),
        word: Word::Succeeds,
        input: || physical_contract,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("exact_executable_digest_bytes"),
        word: Word::Succeeds,
        input: || exact_executable_digest_bytes,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "optional_source_language_profile_preserves_default_and_validates_exact_names",
        ),
        word: Word::Succeeds,
        input: || optional_source_language_profile_preserves_default_and_validates_exact_names,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_identity_uses_utf8_name_order_on_every_platform"),
        word: Word::Succeeds,
        input: || source_identity_uses_utf8_name_order_on_every_platform,
        expect: Expect::Value(()),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("artifact_binding_recomputes_actual_metadata_in_both_directions"),
        word: Word::Succeeds,
        input: || artifact_binding_recomputes_actual_metadata_in_both_directions,
        expect: Expect::Value(()),
    },
    #[cfg(unix)]
    Case {
        name: CaseName(
            "optional_partial_observation_scope_tracks_only_declared_files_without_losing_source_identity",
        ),
        word: Word::Succeeds,
        input: || {
            optional_partial_observation_scope_tracks_only_declared_files_without_losing_source_identity
        },
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
