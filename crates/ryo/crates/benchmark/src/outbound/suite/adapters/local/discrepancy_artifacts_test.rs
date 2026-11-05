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

fn toy_prior_control() -> (serde_json::Value, EvidenceRequirement) {
    let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let worker = "a".repeat(64);
    let query = "select id from toy";
    let profile = "tool = 'toy'\n";
    let source = "x\n";
    let source_sha = hash(
        [b"module.py\0".as_slice(), source.as_bytes()]
            .concat()
            .as_slice(),
    );
    let finding = serde_json::json!({"rule":"toy", "file":"module.py", "start":0, "end":1});
    let receipt = serde_json::json!({
        "side":"candidate", "name":"positive", "complete":true, "error":null,
        "case_passed":true, "source_sha256":source_sha, "skipped":[],
        "diagnostics":[finding.clone()], "expected":[finding],
        "coverage":{"selected_rule_file_pairs":1,"evaluated_rule_file_pairs":1,
            "unknown_or_unread_rule_file_pairs":0,"skipped_rule_file_pairs":0}
    })
    .to_string()
        + "\n";
    let query_sha = hash(query.as_bytes());
    let profile_sha = hash(profile.as_bytes());
    let receipt_sha = hash(receipt.as_bytes());
    let execution = serde_json::json!({
        "schema":"prior-worker-execution/v1", "command":[format!("/sealed/{worker}"),
            "verify-cases", "toy", "/stage"], "stage_root":"/stage", "status":1,
        "worker_sha256":worker, "control_source_sha256":source_sha,
        "control_query_sha256":query_sha, "control_profile_sha256":profile_sha,
        "raw_receipt_sha256":receipt_sha, "witness_case":"positive"
    })
    .to_string();
    let evidence = serde_json::json!({
        "schema":"prior-worker-control/v1", "worker_sha256":worker,
        "control_source_sha256":source_sha, "control_query_sha256":query_sha,
        "control_profile_sha256":profile_sha, "witness_case":"positive",
        "control_sources":{"module.py":source}, "control_query":query,
        "control_profile":profile, "raw_receipt":receipt,
        "raw_receipt_sha256":receipt_sha, "execution_manifest":execution,
        "execution_manifest_sha256":hash(execution.as_bytes())
    });
    let requirement = EvidenceRequirement::PriorWorkerRun {
        worker_sha256: worker,
        control_source_sha256: source_sha,
        control_query_sha256: query_sha,
        control_profile_sha256: profile_sha,
        witness_case: "positive".into(),
    };
    (evidence, requirement)
}

#[test]
fn prior_control_requires_exact_execution_identity_and_completed_positive_witness() {
    let (evidence, requirement) = toy_prior_control();
    let check = |evidence: &serde_json::Value, requirement: &EvidenceRequirement| {
        verify_requirement(&serde_json::to_vec(evidence).unwrap(), requirement)
    };
    check(&evidence, &requirement).expect("neutral complete control");
    for changed in [
        "worker_sha256",
        "control_query_sha256",
        "control_profile_sha256",
    ] {
        let mut requirement = requirement.clone();
        if let EvidenceRequirement::PriorWorkerRun {
            worker_sha256,
            control_query_sha256,
            control_profile_sha256,
            ..
        } = &mut requirement
        {
            *match changed {
                "worker_sha256" => worker_sha256,
                "control_query_sha256" => control_query_sha256,
                _ => control_profile_sha256,
            } = "b".repeat(64);
        }
        assert!(
            check(&evidence, &requirement).is_err(),
            "metadata-only {changed} mutation"
        );
    }
    let mut changed = evidence.clone();
    let mut execution: serde_json::Value =
        serde_json::from_str(changed["execution_manifest"].as_str().unwrap()).unwrap();
    execution["status"] = serde_json::json!(2);
    let execution = execution.to_string();
    changed["execution_manifest"] = serde_json::json!(execution);
    changed["execution_manifest_sha256"] =
        serde_json::json!(format!("{:x}", Sha256::digest(execution.as_bytes())));
    assert!(
        check(&changed, &requirement).is_err(),
        "failed prior execution"
    );

    let mut changed = evidence;
    let raw = "";
    let raw_sha = format!("{:x}", Sha256::digest(raw.as_bytes()));
    changed["raw_receipt"] = serde_json::json!(raw);
    changed["raw_receipt_sha256"] = serde_json::json!(raw_sha);
    let mut execution: serde_json::Value =
        serde_json::from_str(changed["execution_manifest"].as_str().unwrap()).unwrap();
    execution["raw_receipt_sha256"] = serde_json::json!(raw_sha);
    let execution = execution.to_string();
    changed["execution_manifest"] = serde_json::json!(execution);
    changed["execution_manifest_sha256"] =
        serde_json::json!(format!("{:x}", Sha256::digest(execution.as_bytes())));
    assert!(
        check(&changed, &requirement).is_err(),
        "one-sided control lacks candidate witness"
    );
}

#[test]
fn public_contract_requires_pinned_source_and_declared_assertion() {
    let source = "/// Explicit provider selects requested source.\n";
    let source_sha = format!("{:x}", Sha256::digest(source.as_bytes()));
    let evidence = serde_json::json!({
        "schema":"public-tool-contract/v1", "source_revision":"c".repeat(40),
        "source_path":"crates/tool/src/api.rs", "source_sha256":source_sha,
        "contract_assertion":"Explicit provider selects requested source.",
        "source_text":source
    });
    let requirement = EvidenceRequirement::PublicToolContract {
        source_revision: "c".repeat(40),
        source_path: "crates/tool/src/api.rs".into(),
    };
    let check = |value: &serde_json::Value| {
        verify_requirement(&serde_json::to_vec(value).unwrap(), &requirement)
    };
    check(&evidence).expect("neutral pinned source");
    let mut changed = evidence.clone();
    changed["source_path"] = serde_json::json!("crates/other.rs");
    assert!(check(&changed).is_err());
    let mut changed = evidence;
    changed["contract_assertion"] = serde_json::json!("unpublished policy");
    assert!(check(&changed).is_err());
}
