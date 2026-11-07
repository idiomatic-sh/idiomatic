//! Rejected syntax with provider partials skips only accepted-syntax rules.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::default_frontend_projection;
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::python_310;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn rejected_syntax_with_provider_partials_skips_only_accepted_syntax_rules()
-> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    let query =
        "let names = select id, file from \"qualified-name\";\nemit names subject file key(id);\n";
    for rule in ["ASYNC100", "UP025"] {
        let directory = rules.join(rule);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    std::fs::write(root.path().join("input.py"), "value = ur\"hello\"\n")?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&["ASYNC100", "UP025"])),
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::from([
            (ryo_benchmark_contracts::domain::requests::types::RuleLabel("ASYNC100".to_owned()), ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy("always".to_owned())),
            (ryo_benchmark_contracts::domain::requests::types::RuleLabel("UP025".to_owned()), ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy("accepted-syntax".to_owned())),
        ]),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName("python-target=3.10".to_owned())),
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let result =
        factory::local_with_frontend_diagnostic(python_310(), None, default_frontend_projection()?)
            .run_prepared(request);
    match result {
        Err(Error::Invalid(message)) => {
            assert!(
                message.contains("ASYNC100"),
                "always-applicable partial must remain unknown: {message}"
            );
            assert!(
                !message.contains("UP025"),
                "intentionally skipped rule became unknown: {message}"
            );
        }
        Err(error) => return Err(error),
        Ok(unexpected) => {
            drop(unexpected);
            return Err(Error::Invalid(
                "provider partial was not retained for the always rule".into(),
            ));
        }
    }
    Ok(())
}
