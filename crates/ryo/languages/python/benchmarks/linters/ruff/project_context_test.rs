//! Real local-adapter project context and retained-cache lifecycle checks.
use super::PreparedEngine as _;
use ryo_benchmark::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::types::{ContextManifest, PreparedCase, ProjectContextMode, WorkerRequest, WorkerState},
};
use ryo_benchmark::outbound::{
    engine::factory,
    suite::{interface::SuitePort as _, types::ArtifactPath},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn staged(root: &Path) -> Result<(PathBuf, Vec<PreparedCase>), Error> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
    let profile = root.join("profile");
    std::fs::create_dir_all(profile.join("rules/PLW0406"))?;
    std::fs::create_dir_all(profile.join("fragments"))?;
    let _copied_profile = std::fs::copy(
        source.join("profiles/PLW0406/benchmark.toml"),
        profile.join("benchmark.toml"),
    )?;
    let _copied_query = std::fs::copy(
        source.join("rules/PLW0406/query.ryoql"),
        profile.join("rules/PLW0406/query.ryoql"),
    )?;
    let _copied_cases = std::fs::copy(
        source.join("rules/PLW0406/cases.toml"),
        profile.join("rules/PLW0406/cases.toml"),
    )?;
    for name in [
        "suppress",
        "suppress-directives",
        "text-units",
        "noqa-line-map",
        "noqa-locations",
        "noqa-additional-locations",
        "suppression-edges",
    ] {
        let _copied_fragment = std::fs::copy(
            source.join(format!("fragments/{name}.ryoql")),
            profile.join(format!("fragments/{name}.ryoql")),
        )?;
    }
    let cases = ryo_benchmark::outbound::suite::factory::local().cases(
        ArtifactPath(&profile.join("rules")),
        ArtifactPath(&root.join("cases")),
    )?;
    Ok((profile, cases))
}
fn request(profile: &Path, case: &PreparedCase, cache: &Path) -> WorkerRequest {
    WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(vec![case.rule.clone()]),
        rule_observations: None,
        rule_context: None,
        project_search_roots: case.request.project_search_roots.clone(),
        given_project_inventory: case.request.given_project_inventory.clone(),
        project_context_mode: ProjectContextMode::ListedDisk,
        context_manifest: case.request.context_manifest.clone(),
        root: case.request.root.clone(),
        rules: profile.join("rules"),
        query_modules: {
            let mut modules = super::suppress_modules();
            let _previous = modules.insert(
                "python/ruff/text-units".into(),
                "fragments/text-units.ryoql".into(),
            );
            modules
        },
        rule_applicability: BTreeMap::new(),
        parser_configuration: Some("python-target=3.10".into()),
        source_language: None,
        files: case.request.files.clone(),
        state: WorkerState::Retained(cache.to_owned()),
    }
}
fn content_hash(bytes: &[u8]) -> ryo_benchmark::domain::suite::types::ArtifactDigest {
    use sha2::Digest as _;
    ryo_benchmark::domain::suite::types::ArtifactDigest(format!(
        "{:x}",
        sha2::Sha256::digest(bytes)
    ))
}
fn changed_manifest(manifest: &mut ContextManifest, path: &Path, bytes: &[u8]) {
    let digest = content_hash(bytes);
    if let Some(previous) = manifest.auxiliary_files.get_mut(path) {
        *previous = digest;
    } else {
        let _previous = manifest.source_files.insert(path.to_owned(), digest);
    }
}
#[test]
fn exact_root_and_package_controls_reach_the_real_local_adapter() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    assert_eq!(cases.len(), 21);
    let engine = factory::local(super::python_310(), None);
    for case in &cases {
        let result = engine.run_prepared(request(
            &profile,
            case,
            &scratch.path().join(format!("cache-{}", case.name.0)),
        ))?;
        ryo_benchmark::domain::diagnostics::service::require_complete_execution(&result)
            .map_err(|message| Error::Invalid(format!("{}:{message}", case.name.0)))?;
        // Authored expectations declare diagnostics, not observed execution metadata.
        ryo_benchmark::domain::diagnostics::service::compare_diagnostics(&case.expected, &result)
            .map_err(|difference| Error::Invalid(format!("{}:{difference:?}", case.name.0)))?;
        let receipt = result
            .project_context
            .expect("ListedDisk has an active receipt");
        assert_eq!(receipt.mode, ProjectContextMode::ListedDisk);
        assert_eq!(receipt.actual_root, case.request.root);
        assert!(!receipt.dependencies.is_empty());
    }
    Ok(())
}
#[test]
fn shared_cache_root_mode_and_source_edits_preserve_project_evidence() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    let find = |name: &str| {
        cases
            .iter()
            .find(|case| case.name.0 == name)
            .expect("retained case")
    };
    let pkg = find("root-package-name-matches");
    let other = find("root-package-name-differs");
    let cache = scratch.path().join("shared-cache");
    let engine = factory::local(super::python_310(), None);
    let cold = engine.run_prepared(request(&profile, pkg, &cache))?;
    let warm = engine.run_prepared(request(&profile, pkg, &cache))?;
    assert_eq!(
        cold, warm,
        "warm reuse retains the complete dependency receipt"
    );
    assert_eq!(cold.diagnostics.len(), 1);
    assert!(
        engine
            .run_prepared(request(&profile, other, &cache))?
            .diagnostics
            .is_empty(),
        "identical files under another root have another module identity"
    );
    let mut isolated = request(&profile, pkg, &cache);
    isolated.project_context_mode = ProjectContextMode::Isolated;
    isolated.context_manifest = None;
    let result = engine.run_prepared(isolated.clone())?;
    assert!(
        result.diagnostics.is_empty(),
        "Given mode cannot reuse Disk package rows"
    );
    assert!(result.project_context.is_none());
    isolated
        .files
        .push(ryo::linter::RelativePath("__init__.py".into()));
    assert!(
        engine.run_prepared(isolated).is_err(),
        "Given root package is unknown rather than an empty success"
    );
    let mut edited = request(&profile, pkg, &cache);
    let bytes = b"import other.mod\n";
    std::fs::write(edited.root.join("mod.py"), bytes)?;
    changed_manifest(
        edited.context_manifest.as_mut().expect("seal"),
        Path::new("pkg/mod.py"),
        bytes,
    );
    assert!(engine.run_prepared(edited.clone())?.diagnostics.is_empty());
    assert_eq!(
        engine.run_prepared(edited.clone())?,
        engine.run_prepared(edited)?
    );
    Ok(())
}
#[test]
fn changed_undeclared_escaping_and_unreadable_context_never_certify() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    let case = cases
        .iter()
        .find(|case| case.name.0 == "root-package-name-matches")
        .expect("case");
    let engine = factory::local(super::python_310(), None);
    let cache = scratch.path().join("cache");
    let mut undeclared = request(&profile, case, &cache);
    undeclared
        .context_manifest
        .as_mut()
        .expect("seal")
        .missing
        .clear();
    assert!(
        engine.run_prepared(undeclared).is_err(),
        "the first missing ancestor must have been declared"
    );
    let mut escape = request(&profile, case, &cache);
    escape
        .context_manifest
        .as_mut()
        .expect("seal")
        .missing
        .push("../outside".into());
    let _verified_1 = engine.run_prepared(escape).expect_err("expected refusal");
    std::fs::write(case.request.root.join("__init__.py"), "changed")?;
    assert!(
        engine
            .run_prepared(request(&profile, case, &cache))
            .is_err(),
        "auxiliary bytes cannot change after sealing"
    );
    std::fs::write(case.request.root.join("__init__.py"), "")?;
    let manifest = case.request.context_manifest.as_ref().expect("seal");
    std::fs::write(manifest.boundary.join("__init__.py"), "")?;
    assert!(
        engine
            .run_prepared(request(&profile, case, &cache))
            .is_err(),
        "a sealed missing marker cannot appear"
    );
    std::fs::remove_file(manifest.boundary.join("__init__.py"))?;
    std::fs::remove_file(case.request.root.join("__init__.py"))?;
    std::fs::create_dir_all(case.request.root.join("__init__.py"))?;
    assert!(
        engine
            .run_prepared(request(&profile, case, &cache))
            .is_err(),
        "unreadable file content is never a missing dependency"
    );
    Ok(())
}

#[test]
fn ancestor_marker_appearance_removal_and_edits_invalidate_warm_project_rows() -> Result<(), Error>
{
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    let case = cases
        .iter()
        .find(|case| case.name.0 == "ancestor-package-absent")
        .expect("case");
    let engine = factory::local(super::python_310(), None);
    let cache = scratch.path().join("cache");
    let mut asked = request(&profile, case, &cache);
    let absent = engine.run_prepared(asked.clone())?;
    assert!(absent.diagnostics.is_empty());
    let marker = PathBuf::from("pkg2/__init__.py");
    let added_seal = asked.context_manifest.as_mut().expect("seal");
    std::fs::write(added_seal.boundary.join(&marker), b"")?;
    added_seal.missing.retain(|path| path != &marker);
    let _previous = added_seal
        .auxiliary_files
        .insert(marker.clone(), content_hash(b""));
    let added = engine.run_prepared(asked.clone())?;
    assert_eq!(added.diagnostics.len(), 1);
    assert_eq!(added, engine.run_prepared(asked.clone())?);
    let edited_seal = asked.context_manifest.as_mut().expect("seal");
    std::fs::write(edited_seal.boundary.join(&marker), b"# edited\n")?;
    changed_manifest(edited_seal, &marker, b"# edited\n");
    let edited = engine.run_prepared(asked.clone())?;
    assert_eq!(edited.diagnostics, added.diagnostics);
    assert_ne!(
        edited.project_context, added.project_context,
        "kept dependency content identity must refresh"
    );
    let removed_seal = asked.context_manifest.as_mut().expect("seal");
    std::fs::remove_file(removed_seal.boundary.join(&marker))?;
    let _removed = removed_seal.auxiliary_files.remove(&marker);
    removed_seal.missing.push(marker);
    let removed = engine.run_prepared(asked.clone())?;
    assert!(removed.diagnostics.is_empty());
    assert_eq!(removed, engine.run_prepared(asked)?);
    Ok(())
}
