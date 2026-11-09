//! Real local-adapter project context checks. The retained-cache lifecycle of project evidence is proved by core over owned fixtures.
use super::boundary_failure_test::{BoundaryFailure, failure};
use super::prepared_engine_test::PreparedEngine as _;
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::inbound::use_cases::prepare_cases::interface::PrepareCasesUseCase as _;
use ryo_benchmark::outbound::engine::factory;
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
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
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

#[derive(Clone, Copy)]
enum ProjectContextScenario {
    ExactRootAndPackageControlsReachTheRealLocalAdapter,
    ChangedUndeclaredEscapingAndUnreadableContextNeverCertify,
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
        name: CaseName("changed_undeclared_escaping_and_unreadable_context_never_certify"),
        word: Word::Succeeds,
        input: || ProjectContextScenario::ChangedUndeclaredEscapingAndUnreadableContextNeverCertify,
        expect: Expect::Value(true),
    },
];

fn exercise(input: Input) -> Result<Value, Error> {
    match input {
        ProjectContextScenario::ExactRootAndPackageControlsReachTheRealLocalAdapter => {
            exact_root_and_package_controls_reach_the_real_local_adapter().map(|()| true)
        }
        ProjectContextScenario::ChangedUndeclaredEscapingAndUnreadableContextNeverCertify => {
            changed_undeclared_escaping_and_unreadable_context_never_certify().map(|()| true)
        }
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
