//! Real local-adapter project context and retained-cache lifecycle checks.
use super::boundary_failure_test::{BoundaryFailure, failure};
use super::prepared_engine_test::PreparedEngine as _;
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::inbound::use_cases::prepare_cases::interface::PrepareCasesUseCase as _;
use ryo_benchmark::outbound::{
    engine::factory,
    suite::{interface::SuitePort as _, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
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
        "suppress-matches",
        "text-units",
        "noqa-line-map",
        "noqa-locations",
        "noqa-additional-locations",
        "noqa-heads",
        "noqa-codes",
        "noqa-blankets",
        "noqa-ranges",
        "noqa-suppressions",
        "noqa-suppressions-with-additional-lines",
    ] {
        let _copied_fragment = std::fs::copy(
            source.join(format!("fragments/{name}.ryoql")),
            profile.join(format!("fragments/{name}.ryoql")),
        )?;
    }
    let cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &ryo_benchmark::outbound::suite::factory::local(),
            &ryo_benchmark::outbound::context::factory::local(),
        )
        .prepare_cases(&profile.join("rules"), &root.join("cases"))?;
    Ok((profile, cases))
}
fn request(profile: &Path, case: &PreparedCase, cache: &Path) -> WorkerRequest {
    WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            vec![case.rule.clone()],
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: case.request.project_search_roots.clone(),
        given_project_inventory: case.request.given_project_inventory.clone(),
        project_context_mode: ProjectContextMode::ListedDisk,
        context_manifest: case.request.context_manifest.clone(),
        root: case.request.root.clone(),
        rules: profile.join("rules"),
        query_modules: {
            let mut modules = super::prepared_engine_test::suppress_modules();
            let _previous = modules.insert(
                "python/ruff/text-units".into(),
                "fragments/text-units.ryoql".into(),
            );
            super::prepared_engine_test::module_names(modules)
        },
        rule_applicability: BTreeMap::new(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(
                ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(
                    "python-target=3.10".into(),
                ),
            ),
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: case.request.files.clone(),
        state: WorkerState::Retained(cache.to_owned()),
    }
}
fn sealed(
    seal: &ryo_benchmark_contracts::domain::requests::types::ContextSeal,
) -> &ContextManifest {
    match seal {
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest) => manifest,
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed => panic!("seal"),
    }
}
fn sealed_mut(
    seal: &mut ryo_benchmark_contracts::domain::requests::types::ContextSeal,
) -> &mut ContextManifest {
    match seal {
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest) => manifest,
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed => panic!("seal"),
    }
}
fn content_hash(
    bytes: &[u8],
) -> ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest {
    use sha2::Digest as _;
    ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(format!(
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
fn exact_root_and_package_controls_reach_the_real_local_adapter() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    assert_eq!(cases.len(), 21);
    let engine = factory::local(super::prepared_engine_test::python_310(), None);
    for case in &cases {
        let result = engine.run_prepared(request(
            &profile,
            case,
            &scratch.path().join(format!("cache-{}", case.name.0)),
        ))?;
        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(&result)
            .map_err(|message| Error::Invalid(format!("{}:{message}", case.name.0)))?;
        // Authored expectations declare diagnostics, not observed execution metadata.
        ryo_benchmark::domain::diagnostics::service::compare_diagnostics(&case.expected, &result)
            .map_err(|difference| Error::Invalid(format!("{}:{difference:?}", case.name.0)))?;
        let ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(receipt) =
            result.project_context
        else {
            panic!("ListedDisk has an active receipt");
        };
        assert_eq!(receipt.mode, ProjectContextMode::ListedDisk);
        assert_eq!(receipt.actual_root, case.request.root);
        assert!(!receipt.dependencies.is_empty());
    }
    Ok(())
}
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
    let engine = factory::local(super::prepared_engine_test::python_310(), None);
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
    isolated.context_manifest =
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed;
    let result = engine.run_prepared(isolated.clone())?;
    assert!(
        result.diagnostics.is_empty(),
        "Given mode cannot reuse Disk package rows"
    );
    assert_eq!(
        result.project_context,
        ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested
    );
    isolated
        .files
        .push(ryo::linter::RelativePath("__init__.py".into()));
    assert!(
        matches!(engine.run_prepared(isolated), Err(Error::Invalid(_))),
        "Given root package is unknown rather than an empty success"
    );
    let mut edited = request(&profile, pkg, &cache);
    let bytes = b"import other.mod\n";
    std::fs::write(edited.root.join("mod.py"), bytes)?;
    changed_manifest(
        sealed_mut(&mut edited.context_manifest),
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
fn changed_undeclared_escaping_and_unreadable_context_never_certify() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    let case = cases
        .iter()
        .find(|case| case.name.0 == "root-package-name-matches")
        .expect("case");
    let engine = factory::local(super::prepared_engine_test::python_310(), None);
    let cache = scratch.path().join("cache");
    let mut undeclared = request(&profile, case, &cache);
    sealed_mut(&mut undeclared.context_manifest).missing.clear();
    assert!(sealed(&undeclared.context_manifest).missing.is_empty());
    let discovered = engine.run_prepared(undeclared)?;
    assert_eq!(
        match discovered.project_context {
            ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(receipt) =>
                receipt,
            ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested =>
                panic!("observed context"),
        }
        .dependencies
        .get(std::path::Path::new("../__init__.py")),
        Some(&None),
        "the candidate must report the actual absent ancestor despite an empty base missing list"
    );
    let mut escape = request(&profile, case, &cache);
    sealed_mut(&mut escape.context_manifest)
        .missing
        .push("../outside".into());
    let _verified_1 = engine.run_prepared(escape).expect_err("expected refusal");
    std::fs::write(case.request.root.join("__init__.py"), "changed")?;
    assert!(
        matches!(
            engine.run_prepared(request(&profile, case, &cache)),
            Err(Error::Invalid(_))
        ),
        "auxiliary bytes cannot change after sealing"
    );
    std::fs::write(case.request.root.join("__init__.py"), "")?;
    let manifest = sealed(&case.request.context_manifest);
    std::fs::write(manifest.boundary.join("__init__.py"), "")?;
    assert!(
        matches!(
            engine.run_prepared(request(&profile, case, &cache)),
            Err(Error::Invalid(_))
        ),
        "a sealed missing marker cannot appear"
    );
    std::fs::remove_file(manifest.boundary.join("__init__.py"))?;
    std::fs::remove_file(case.request.root.join("__init__.py"))?;
    std::fs::create_dir_all(case.request.root.join("__init__.py"))?;
    assert!(
        matches!(
            engine.run_prepared(request(&profile, case, &cache)),
            Err(Error::Io(_))
        ),
        "unreadable file content is never a missing dependency"
    );
    Ok(())
}

fn ancestor_marker_appearance_removal_and_edits_invalidate_warm_project_rows() -> Result<(), Error>
{
    let scratch = tempfile::tempdir()?;
    let (profile, cases) = staged(scratch.path())?;
    let case = cases
        .iter()
        .find(|case| case.name.0 == "ancestor-package-absent")
        .expect("case");
    let engine = factory::local(super::prepared_engine_test::python_310(), None);
    let cache = scratch.path().join("cache");
    let mut asked = request(&profile, case, &cache);
    let marker = PathBuf::from("pkg2/__init__.py");
    sealed_mut(&mut asked.context_manifest)
        .missing
        .retain(|path| path != &marker);
    let absent = engine.run_prepared(asked.clone())?;
    assert!(absent.diagnostics.is_empty());
    assert_eq!(absent, engine.run_prepared(asked.clone())?);
    let added_seal = sealed_mut(&mut asked.context_manifest);
    std::fs::write(added_seal.boundary.join(&marker), b"")?;
    added_seal.missing.retain(|path| path != &marker);
    let _previous = added_seal
        .auxiliary_files
        .insert(marker.clone(), content_hash(b""));
    let added = engine.run_prepared(asked.clone())?;
    assert_eq!(added.diagnostics.len(), 1);
    assert_eq!(added, engine.run_prepared(asked.clone())?);
    let edited_seal = sealed_mut(&mut asked.context_manifest);
    std::fs::write(edited_seal.boundary.join(&marker), b"# edited\n")?;
    changed_manifest(edited_seal, &marker, b"# edited\n");
    let edited = engine.run_prepared(asked.clone())?;
    assert_eq!(edited.diagnostics, added.diagnostics);
    assert_ne!(
        edited.project_context, added.project_context,
        "kept dependency content identity must refresh"
    );
    let removed_seal = sealed_mut(&mut asked.context_manifest);
    std::fs::remove_file(removed_seal.boundary.join(&marker))?;
    let _removed = removed_seal.auxiliary_files.remove(&marker);
    let removed = engine.run_prepared(asked.clone())?;
    assert!(removed.diagnostics.is_empty());
    assert_eq!(removed.project_context, absent.project_context);
    assert_eq!(removed, engine.run_prepared(asked)?);
    Ok(())
}

#[derive(Clone, Copy)]
enum ProjectContextScenario {
    ExactRootAndPackageControlsReachTheRealLocalAdapter,
    SharedCacheRootModeAndSourceEditsPreserveProjectEvidence,
    ChangedUndeclaredEscapingAndUnreadableContextNeverCertify,
    AncestorMarkerAppearanceRemovalAndEditsInvalidateWarmProjectRows,
}
type Input = ProjectContextScenario;
type Value = bool;
type Failure = BoundaryFailure;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("exact_root_and_package_controls_reach_the_real_local_adapter"),
        word: Word::Succeeds,
        input: || ProjectContextScenario::ExactRootAndPackageControlsReachTheRealLocalAdapter,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("shared_cache_root_mode_and_source_edits_preserve_project_evidence"),
        word: Word::Succeeds,
        input: || ProjectContextScenario::SharedCacheRootModeAndSourceEditsPreserveProjectEvidence,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("changed_undeclared_escaping_and_unreadable_context_never_certify"),
        word: Word::Succeeds,
        input: || ProjectContextScenario::ChangedUndeclaredEscapingAndUnreadableContextNeverCertify,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("ancestor_marker_appearance_removal_and_edits_invalidate_warm_project_rows"),
        word: Word::Succeeds,
        input: || {
            ProjectContextScenario::AncestorMarkerAppearanceRemovalAndEditsInvalidateWarmProjectRows
        },
        expect: Expect::Value(true),
    },
];

fn exercise(input: Input) -> Result<Value, Error> {
    match input {
        ProjectContextScenario::ExactRootAndPackageControlsReachTheRealLocalAdapter => exact_root_and_package_controls_reach_the_real_local_adapter().map(|()| true),
        ProjectContextScenario::SharedCacheRootModeAndSourceEditsPreserveProjectEvidence => shared_cache_root_mode_and_source_edits_preserve_project_evidence().map(|()| true),
        ProjectContextScenario::ChangedUndeclaredEscapingAndUnreadableContextNeverCertify => changed_undeclared_escaping_and_unreadable_context_never_certify().map(|()| true),
        ProjectContextScenario::AncestorMarkerAppearanceRemovalAndEditsInvalidateWarmProjectRows => ancestor_marker_appearance_removal_and_edits_invalidate_warm_project_rows().map(|()| true),
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
