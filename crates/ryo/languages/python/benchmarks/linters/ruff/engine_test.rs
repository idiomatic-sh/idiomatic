//! Real SDK evidence for missing files, unread or partial inputs and typed findings.
use boundary_failure_test::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::inbound::use_cases::prepare_cases::interface::PrepareCasesUseCase as _;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark::outbound::suite::interface::SuitePort as _;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(crate) mod boundary_failure_test;
pub(crate) mod constants;
mod prepared_engine_test;
use prepared_engine_test::{
    PreparedEngine as _, custom_frontend_projection, default_frontend_projection, fixture_cohort,
    python_310, suppress_modules,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EngineScenario {
    Finding,
    Missing,
    Unread,
    Partial,
    InvalidSyntax,
    LexicallyCompleteSyntaxError,
    PreservedSemanticSyntaxError,
    PreservedStubSemanticSyntaxError,
    OrdinaryAnnotationNamedExpression,
    InvalidSyntaxProjectionDisabled,
    CustomFrontendProjection,
    FutureFeatureProjection,
    LateFutureDirectiveProjection,
    UnknownFutureWithoutConsumer,
    FrontendRuleCollision,
    ProjectionCacheQueryChange,
    QueryModuleInlineEquivalence,
    TargetConfigurationCacheChange,
    RejectedSyntaxWithProviderPartialsSkipsOnlyAcceptedSyntaxRules,
    SharedSuppressionFragmentMatchesTheHistoricalInlineQueries,
    LiteralPropertiesReachEngineQueriesForTrueAndFalseBooleanNodes,
    DecodedNumericPayloadsAndStrictGreaterCompositionReachEngineQueries,
    SelectedConsumerKeepsUnselectedProducerWithoutItsDiagnostics,
    ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay,
    LazyFutureStaysVisibleWithoutADedicatedSelectedRule,
    ListedProjectInputsAndInvalidSourcesKeepDistinctReaders,
    I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory,
}
type Input = EngineScenario;
type Value = bool;
type Failure = BoundaryFailure;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sdk_preserves_checked_file_rule_and_byte_span"),
        word: Word::Succeeds,
        input: || EngineScenario::Finding,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("sdk_projects_module_nonlocal_as_invalid_syntax"),
        word: Word::Succeeds,
        input: || EngineScenario::InvalidSyntax,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontend_projection_keeps_parse_diagnostics_with_complete_tokens"),
        word: Word::Succeeds,
        input: || EngineScenario::LexicallyCompleteSyntaxError,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontend_projection_keeps_semantic_parse_diagnostics_and_ast_facts"),
        word: Word::Succeeds,
        input: || EngineScenario::PreservedSemanticSyntaxError,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("stub_future_annotation_error_keeps_f821_and_frontend_rows"),
        word: Word::Succeeds,
        input: || EngineScenario::PreservedStubSemanticSyntaxError,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("ordinary_annotation_keeps_f821_without_syntax_row"),
        word: Word::Succeeds,
        input: || EngineScenario::OrdinaryAnnotationNamedExpression,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontend_projection_is_disabled_without_an_identity"),
        word: Word::Succeeds,
        input: || EngineScenario::InvalidSyntaxProjectionDisabled,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontend_projection_uses_its_supplied_rule_and_query"),
        word: Word::Succeeds,
        input: || EngineScenario::CustomFrontendProjection,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("future-feature_validation_stays_a_lint_not_invalid_syntax"),
        word: Word::Succeeds,
        input: || EngineScenario::FutureFeatureProjection,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("late_future_directive_keeps_f404_without_a_grammar_error"),
        word: Word::Succeeds,
        input: || EngineScenario::LateFutureDirectiveProjection,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unconsumed_future-feature_diagnostic_does_not_fall_back_to_invalid_syntax"),
        word: Word::Succeeds,
        input: || EngineScenario::UnknownFutureWithoutConsumer,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontend_projection_cannot_replace_an_authored_rule"),
        word: Word::Refuses,
        input: || EngineScenario::FrontendRuleCollision,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("retained_parse_cache_respects_a_changed_projection_query"),
        word: Word::Succeeds,
        input: || EngineScenario::ProjectionCacheQueryChange,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("staged_query_module_matches_its_inline_query"),
        word: Word::Succeeds,
        input: || EngineScenario::QueryModuleInlineEquivalence,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("retained_cache_recomputes_target_gated_query_after_configuration_change"),
        word: Word::Succeeds,
        input: || EngineScenario::TargetConfigurationCacheChange,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unread_input_is_not_checked"),
        word: Word::Refuses,
        input: || EngineScenario::Unread,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("partial_relations_cannot_be_reported_as_checked"),
        word: Word::Refuses,
        input: || EngineScenario::Partial,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("missing_listed_file_is_failure"),
        word: Word::Fails,
        input: || EngineScenario::Missing,
        expect: Expect::Error(BoundaryFailure::Io(std::io::ErrorKind::NotFound)),
    },
    Case {
        name: CaseName("rejected_syntax_with_provider_partials_skips_only_accepted_syntax_rules"),
        word: Word::Succeeds,
        input: || EngineScenario::RejectedSyntaxWithProviderPartialsSkipsOnlyAcceptedSyntaxRules,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("shared_suppression_fragment_matches_the_historical_inline_queries"),
        word: Word::Succeeds,
        input: || EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("literal_properties_reach_engine_queries_for_true_and_false_boolean_nodes"),
        word: Word::Succeeds,
        input: || EngineScenario::LiteralPropertiesReachEngineQueriesForTrueAndFalseBooleanNodes,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "decoded_numeric_payloads_and_strict_greater_composition_reach_engine_queries",
        ),
        word: Word::Succeeds,
        input: || {
            EngineScenario::DecodedNumericPayloadsAndStrictGreaterCompositionReachEngineQueries
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("selected_consumer_keeps_unselected_producer_without_its_diagnostics"),
        word: Word::Succeeds,
        input: || EngineScenario::SelectedConsumerKeepsUnselectedProducerWithoutItsDiagnostics,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "actual_i001_mixed_namespace_observation_stays_unknown_on_cold_and_warm_replay",
        ),
        word: Word::Succeeds,
        input: || {
            EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("lazy_future_stays_visible_without_a_dedicated_selected_rule"),
        word: Word::Succeeds,
        input: || EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("listed_project_inputs_and_invalid_sources_keep_distinct_readers"),
        word: Word::Succeeds,
        input: || EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "i001_hidden_match_and_ruf100_output_follow_first_import_and_unknown_inventory",
        ),
        word: Word::Succeeds,
        input: || {
            EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory
        },
        expect: Expect::Value(true),
    },
];
fn exercise(scenario: Input) -> Result<Value, Error> {
    if scenario == EngineScenario::RejectedSyntaxWithProviderPartialsSkipsOnlyAcceptedSyntaxRules {
        return rejected_syntax_with_provider_partials_skips_only_accepted_syntax_rules()
            .map(|()| true);
    }
    if scenario == EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries {
        return shared_suppression_fragment_matches_the_historical_inline_queries().map(|()| true);
    }
    if scenario == EngineScenario::LiteralPropertiesReachEngineQueriesForTrueAndFalseBooleanNodes {
        return literal_properties_reach_engine_queries_for_true_and_false_boolean_nodes()
            .map(|()| true);
    }
    if scenario
        == EngineScenario::DecodedNumericPayloadsAndStrictGreaterCompositionReachEngineQueries
    {
        return decoded_numeric_payloads_and_strict_greater_composition_reach_engine_queries()
            .map(|()| true);
    }
    if scenario == EngineScenario::SelectedConsumerKeepsUnselectedProducerWithoutItsDiagnostics {
        return selected_consumer_keeps_unselected_producer_without_its_diagnostics()
            .map(|()| true);
    }
    if scenario
        == EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
    {
        actual_i001_mixed_namespace_observation_stays_unknown_on_cold_and_warm_replay();
        return Ok(true);
    }
    if scenario == EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule {
        return future_directives_test::lazy_future_stays_visible_without_a_dedicated_selected_rule().map(|()| true);
    }
    if scenario == EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders {
        return project_inputs_test::listed_project_inputs_and_invalid_sources_keep_distinct_readers().map(|()| true);
    }
    if scenario
        == EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory
    {
        i001_observation_test::i001_hidden_match_and_ruf100_output_follow_first_import_and_unknown_inventory();
        return Ok(true);
    }
    let root = tempfile::tempdir()?;
    if scenario == EngineScenario::QueryModuleInlineEquivalence {
        return query_module_inline_equivalence(root.path());
    }
    if scenario == EngineScenario::TargetConfigurationCacheChange {
        return target_configuration_cache_change(root.path());
    }
    if scenario == EngineScenario::ProjectionCacheQueryChange {
        return projection_cache_query_change(root.path());
    }
    let rules = root.path().join("rules");
    let selected_rules = write_rule_fixtures(&rules, scenario)?;
    let file = input_file(scenario);
    if scenario != EngineScenario::Missing {
        std::fs::write(root.path().join(file), source_for(scenario))?;
    }
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(selected_rules),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: modules_for(scenario)?,
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let result = run_with_projection(scenario, request);
    check_expected_failure(scenario, &result);
    let report = result?;
    let expected = expected_diagnostics(scenario)?;
    Ok(report.files == [ryo::linter::RelativePath(file.into())] && report.diagnostics == expected)
}

fn rejected_syntax_with_provider_partials_skips_only_accepted_syntax_rules() -> Result<(), Error> {
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
        selected_rules: Some(fixture_cohort(&["ASYNC100", "UP025"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::from([
            ("ASYNC100".to_owned(), "always".to_owned()),
            ("UP025".to_owned(), "accepted-syntax".to_owned()),
        ]),
        parser_configuration: Some("python-target=3.10".to_owned()),
        source_language: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let result = factory::local_with_frontend_diagnostic(
        python_310(),
        None,
        default_frontend_projection()?,
    )
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

fn input_file(scenario: EngineScenario) -> &'static str {
    match scenario {
        EngineScenario::Missing => "missing.py",
        EngineScenario::Unread => "input.unknown",
        EngineScenario::Finding
        | EngineScenario::Partial
        | EngineScenario::InvalidSyntax
        | EngineScenario::LexicallyCompleteSyntaxError
        | EngineScenario::PreservedSemanticSyntaxError
        | EngineScenario::OrdinaryAnnotationNamedExpression
        | EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::CustomFrontendProjection
        | EngineScenario::FutureFeatureProjection
        | EngineScenario::LateFutureDirectiveProjection
        | EngineScenario::UnknownFutureWithoutConsumer
        | EngineScenario::FrontendRuleCollision
        | EngineScenario::ProjectionCacheQueryChange
        | EngineScenario::QueryModuleInlineEquivalence
        | EngineScenario::TargetConfigurationCacheChange
        | EngineScenario::RejectedSyntaxWithProviderPartialsSkipsOnlyAcceptedSyntaxRules
        | EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries
        | EngineScenario::LiteralPropertiesReachEngineQueriesForTrueAndFalseBooleanNodes
        | EngineScenario::DecodedNumericPayloadsAndStrictGreaterCompositionReachEngineQueries
        | EngineScenario::SelectedConsumerKeepsUnselectedProducerWithoutItsDiagnostics
        | EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        | EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule
        | EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders
        | EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory => "input.py",
        EngineScenario::PreservedStubSemanticSyntaxError => "input.pyi",
    }
}

fn source_for(scenario: EngineScenario) -> &'static str {
    match scenario {
        // An unmatched delimiter can now certify complete lexical facts. An
        // unterminated string cannot: suppression must not trust comment absence.
        EngineScenario::Partial => "try:\n    pass\nexcept:\n    pass\n\n\"unterminated\n",
        EngineScenario::InvalidSyntax
        | EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::CustomFrontendProjection
        | EngineScenario::FrontendRuleCollision => "global x\nnonlocal x\n",
        EngineScenario::LexicallyCompleteSyntaxError => "value = ur\"hello\"\n",
        EngineScenario::PreservedSemanticSyntaxError
        | EngineScenario::PreservedStubSemanticSyntaxError => {
            "from __future__ import annotations\ndef f(x: (a := b)): ...\n"
        }
        EngineScenario::OrdinaryAnnotationNamedExpression => "def f(x: (a := b)): ...\n",
        EngineScenario::FutureFeatureProjection | EngineScenario::UnknownFutureWithoutConsumer => {
            "from __future__ import no_such_feature\n"
        }
        EngineScenario::LateFutureDirectiveProjection => {
            "x = 1\nfrom __future__ import annotations\n"
        }
        EngineScenario::ProjectionCacheQueryChange => "nonlocal x\n",
        EngineScenario::Finding
        | EngineScenario::Missing
        | EngineScenario::Unread
        | EngineScenario::QueryModuleInlineEquivalence
        | EngineScenario::TargetConfigurationCacheChange
        | EngineScenario::RejectedSyntaxWithProviderPartialsSkipsOnlyAcceptedSyntaxRules
        | EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries
        | EngineScenario::LiteralPropertiesReachEngineQueriesForTrueAndFalseBooleanNodes
        | EngineScenario::DecodedNumericPayloadsAndStrictGreaterCompositionReachEngineQueries
        | EngineScenario::SelectedConsumerKeepsUnselectedProducerWithoutItsDiagnostics
        | EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        | EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule
        | EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders
        | EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory => "try:\n    pass\nexcept:\n    pass\n",
    }
}

fn write_rule_fixtures(
    rules: &std::path::Path,
    scenario: EngineScenario,
) -> Result<Vec<ryo::linter::RuleId>, Error> {
    let mut selected = fixture_cohort(&["E722"]);
    let root = rules
        .parent()
        .ok_or_else(|| Error::Invalid("rules directory has no root".into()))?;
    write_suppress_module(root)?;
    let e722 = rules.join("E722");
    std::fs::create_dir_all(&e722)?;
    std::fs::write(
        e722.join("query.ryoql"),
        include_str!("rules/E722/query.ryoql"),
    )?;
    if matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
            | EngineScenario::OrdinaryAnnotationNamedExpression
    ) {
        let source =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
        for path in modules_for(scenario)?.values() {
            let destination = root.join(path);
            let parent = destination
                .parent()
                .ok_or_else(|| Error::Invalid("module has no parent".into()))?;
            std::fs::create_dir_all(parent)?;
            let _copied = std::fs::copy(source.join(path), destination)?;
        }
        selected.extend(fixture_cohort(&["F821"]));
        let f821 = rules.join("F821");
        std::fs::create_dir_all(&f821)?;
        std::fs::write(
            f821.join("query.ryoql"),
            include_str!("rules/F821/query.ryoql"),
        )?;
    }
    if scenario == EngineScenario::FutureFeatureProjection {
        selected.extend(fixture_cohort(&["F407"]));
        let f407 = rules.join("F407");
        std::fs::create_dir_all(&f407)?;
        std::fs::write(
            f407.join("query.ryoql"),
            include_str!("rules/F407/query.ryoql"),
        )?;
    }
    if scenario == EngineScenario::LateFutureDirectiveProjection {
        selected.extend(fixture_cohort(&["F404"]));
        let f404 = rules.join("F404");
        std::fs::create_dir_all(&f404)?;
        std::fs::write(
            f404.join("query.ryoql"),
            include_str!("rules/F404/query.ryoql"),
        )?;
    }
    if scenario == EngineScenario::FrontendRuleCollision {
        selected.extend(fixture_cohort(&["invalid-syntax"]));
        let invalid_syntax = rules.join("invalid-syntax");
        std::fs::create_dir_all(&invalid_syntax)?;
        std::fs::write(
            invalid_syntax.join("query.ryoql"),
            include_str!("rules/E722/query.ryoql"),
        )?;
    }
    Ok(selected)
}

/// Use the query's declared dependency registry for the preserved-facts integration cases.
fn modules_for(scenario: EngineScenario) -> Result<BTreeMap<String, std::path::PathBuf>, Error> {
    if !matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
            | EngineScenario::OrdinaryAnnotationNamedExpression
    ) {
        return Ok(suppress_modules());
    }
    let profile: toml::Value = toml::from_str(include_str!("profiles/F821/benchmark.toml"))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let modules = profile
        .get("query-modules")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::Invalid("F821 profile has no declared modules".into()))?;
    let mut resolved: BTreeMap<String, std::path::PathBuf> = modules
        .iter()
        .map(|(name, path)| {
            path.as_str()
                .map(|path| (name.clone(), std::path::PathBuf::from(path)))
                .ok_or_else(|| Error::Invalid("F821 module path is not text".into()))
        })
        .collect::<Result<_, Error>>()?;
    resolved.extend(suppress_modules());
    Ok(resolved)
}

fn write_suppress_module(root: &std::path::Path) -> Result<(), Error> {
    let directory = root.join("fragments");
    std::fs::create_dir_all(&directory)?;
    for (name, source) in [
        (
            "suppress-matches",
            include_str!("fragments/suppress-matches.ryoql"),
        ),
        (
            "noqa-line-map",
            include_str!("fragments/noqa-line-map.ryoql"),
        ),
        (
            "noqa-locations",
            include_str!("fragments/noqa-locations.ryoql"),
        ),
        (
            "noqa-additional-locations",
            include_str!("fragments/noqa-additional-locations.ryoql"),
        ),
        (
            "suppression-edges",
            include_str!("fragments/suppression-edges.ryoql"),
        ),
        ("suppress", include_str!("fragments/suppress.ryoql")),
        (
            "suppress-directives",
            include_str!("fragments/suppress-directives.ryoql"),
        ),
    ] {
        std::fs::write(directory.join(format!("{name}.ryoql")), source)?;
    }
    Ok(())
}

fn run_with_projection(
    scenario: EngineScenario,
    request: WorkerRequest,
) -> Result<ryo_benchmark_contracts::domain::reports::types::RunReport, Error> {
    if scenario == EngineScenario::InvalidSyntaxProjectionDisabled {
        return factory::local(python_310(), None).run_prepared(request);
    }
    let projection = if scenario == EngineScenario::CustomFrontendProjection {
        custom_frontend_projection()?
    } else {
        default_frontend_projection()?
    };
    factory::local_with_frontend_diagnostic(python_310(), None, projection).run_prepared(request)
}

fn check_expected_failure(
    scenario: EngineScenario,
    result: &Result<ryo_benchmark_contracts::domain::reports::types::RunReport, Error>,
) {
    if scenario == EngineScenario::Unread {
        assert!(
            matches!(result, Err(Error::Invalid(message)) if message.starts_with("incomplete candidate:") && message.contains("input.unknown")),
            "unread input must be reported as incomplete candidate coverage: {result:?}"
        );
    }
    if scenario == EngineScenario::Partial {
        assert!(
            matches!(result, Err(Error::Invalid(message))
                if message.starts_with("incomplete candidate:")
                    && !message.contains("unknowns=[]")
                    && message.contains("unread=[]")),
            "partial relation availability must refuse despite complete file coverage: {result:?}"
        );
    }
}

fn expected_diagnostics(scenario: EngineScenario) -> Result<Vec<BenchmarkFinding>, Error> {
    use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset as At;
    if matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
    ) {
        let (file, syntax_start, syntax_end, name_start, name_end) =
            if scenario == EngineScenario::PreservedSemanticSyntaxError {
                ("input.py", 45, 51, 50, 51)
            } else {
                ("input.pyi", 45, 51, 50, 51)
            };
        return Ok(vec![
            BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from("F821")
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                file: ryo::linter::RelativePath(file.into()),
                start: At(name_start),
                end: At(name_end),
            },
            BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from("invalid-syntax")
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                file: ryo::linter::RelativePath(file.into()),
                start: At(syntax_start),
                end: At(syntax_end),
            },
        ]);
    }
    if scenario == EngineScenario::OrdinaryAnnotationNamedExpression {
        return Ok(vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("F821")
                .map_err(|error| Error::Invalid(error.to_string()))?,
            file: ryo::linter::RelativePath("input.py".into()),
            start: At(15),
            end: At(16),
        }]);
    }
    let (rule, start, end) = match scenario {
        EngineScenario::InvalidSyntax => ("invalid-syntax", 9, 19),
        EngineScenario::LexicallyCompleteSyntaxError => ("invalid-syntax", 10, 17),
        EngineScenario::PreservedSemanticSyntaxError
        | EngineScenario::PreservedStubSemanticSyntaxError
        | EngineScenario::OrdinaryAnnotationNamedExpression => {
            return Err(Error::Invalid(
                "semantic syntax scenario bypassed its dedicated expectation".to_owned(),
            ));
        }
        EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::UnknownFutureWithoutConsumer => {
            return Ok(Vec::new());
        }
        EngineScenario::CustomFrontendProjection => ("parser-check", 9, 19),
        EngineScenario::FutureFeatureProjection => ("F407", 23, 38),
        EngineScenario::LateFutureDirectiveProjection => ("F404", 6, 40),
        EngineScenario::Finding
        | EngineScenario::Missing
        | EngineScenario::Unread
        | EngineScenario::Partial
        | EngineScenario::FrontendRuleCollision
        | EngineScenario::ProjectionCacheQueryChange
        | EngineScenario::QueryModuleInlineEquivalence
        | EngineScenario::RejectedSyntaxWithProviderPartialsSkipsOnlyAcceptedSyntaxRules
        | EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries
        | EngineScenario::LiteralPropertiesReachEngineQueriesForTrueAndFalseBooleanNodes
        | EngineScenario::DecodedNumericPayloadsAndStrictGreaterCompositionReachEngineQueries
        | EngineScenario::SelectedConsumerKeepsUnselectedProducerWithoutItsDiagnostics
        | EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        | EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule
        | EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders
        | EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory => ("E722", 14, 20),
        EngineScenario::TargetConfigurationCacheChange => ("UP017", 16, 37),
    };
    Ok(vec![BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from(rule)
            .map_err(|error| Error::Invalid(error.to_string()))?,
        file: ryo::linter::RelativePath("input.py".into()),
        start: At(start),
        end: At(end),
    }])
}

fn projection_cache_query_change(root: &std::path::Path) -> Result<Value, Error> {
    let rules = root.join("rules");
    let selected_rules = write_rule_fixtures(&rules, EngineScenario::ProjectionCacheQueryChange)?;
    let file = input_file(EngineScenario::ProjectionCacheQueryChange);
    std::fs::write(
        root.join(file),
        source_for(EngineScenario::ProjectionCacheQueryChange),
    )?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(selected_rules),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.to_owned(),
        rules,
        query_modules: suppress_modules(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let rule = ryo::linter::RuleId::try_from("cache-probe")
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let run = |code| {
        let query = format!(
            "let diagnostics = select id, file, span from \"syntax-error\" where code = \"{code}\";\nemit diagnostics subject file key(id);\n"
        );
        let projection = factory::FrontendDiagnosticProjection {
            rule: rule.clone(),
            query: ryo::query::RyoQlQuery::try_from(query.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))?,
        };
        Ok::<_, Error>(factory::local_with_frontend_diagnostic(
            python_310(),
            None,
            projection,
        ))
    };
    let first = run("nonlocal-at-module")?.run_prepared(WorkerRequest {
        project_search_roots: None,
        given_project_inventory: Default::default(),
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
            root.join("retained-parse-cache"),
        ),
        ..request.clone()
    })?;
    let second = run("no-such-parser-code")?.run_prepared(WorkerRequest {
        project_search_roots: None,
        given_project_inventory: Default::default(),
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
            root.join("retained-parse-cache"),
        ),
        ..request
    })?;
    Ok(first.diagnostics.len() == 1
        && first
            .diagnostics
            .first()
            .is_some_and(|finding| finding.rule.as_ref() == "cache-probe")
        && second.diagnostics.is_empty())
}

fn query_module_inline_equivalence(root: &std::path::Path) -> Result<bool, Error> {
    let rules = root.join("rules");
    let rule_dir = rules.join("fixture");
    std::fs::create_dir_all(&rule_dir)?;
    std::fs::write(root.join("input.py"), "value = 1\n")?;
    let input = "let nodes = select id, file, span from \"syntax-tree\";\n";
    let inline = format!("{input}emit nodes subject file key(span);\n");
    std::fs::write(rule_dir.join("query.ryoql"), inline)?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked()]);
    let plain = factory::local(languages.clone(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["fixture"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.into(),
        rules: rules.clone(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;

    let fragments = root.join("fragments");
    std::fs::create_dir_all(&fragments)?;
    std::fs::write(
        fragments.join("identity.ryoql"),
        "input source: relation<id: node, file: file, span: span>;\noutput relation<id: node, file: file, span: span>;\nlet rows = select id, file, span from source;\nreturn rows;\n",
    )?;
    std::fs::write(
        rule_dir.join("query.ryoql"),
        format!(
            "import identity from \"test/identity\";\n{input}let rows = identity(source=nodes);\nemit rows subject file key(span);\n"
        ),
    )?;
    let mut modules = BTreeMap::new();
    let _previous = modules.insert(
        "test/identity".to_owned(),
        std::path::PathBuf::from("fragments/identity.ryoql"),
    );
    let imported = factory::local(languages, None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["fixture"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.into(),
        rules,
        query_modules: modules,
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    Ok(plain == imported)
}

/// Stage every canonical source; its declared expectations are bound to Python3.11.
/// The same complete source set must be negative on3.10, before and after cache reuse.
fn stage_up017_target_files(
    root: &std::path::Path,
) -> Result<(Vec<ryo::linter::RelativePath>, Vec<BenchmarkFinding>), Error> {
    #[derive(serde::Deserialize)]
    struct Cases {
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        files: BTreeMap<String, String>,
        expected: Vec<Finding>,
    }
    #[derive(serde::Deserialize)]
    struct Finding {
        rule: String,
        file: String,
        start: usize,
        end: usize,
    }
    let cases: Cases = toml::from_str(include_str!("rules/UP017/cases.toml"))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if cases.cases.len() != 14 {
        return Err(Error::Invalid(
            "UP017 target control must retain all14 canonical cases".into(),
        ));
    }
    let (mut files, mut expected_py311) = (Vec::new(), Vec::new());
    for case in cases.cases {
        for finding in case.expected {
            if finding.rule != "UP017" || !case.files.contains_key(&finding.file) {
                return Err(Error::Invalid(format!(
                    "invalid UP017 py311 expectation in {}",
                    case.name
                )));
            }
            expected_py311.push(BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from(finding.rule.as_str())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                file: ryo::linter::RelativePath(
                    format!("up017-target/{}/{}", case.name, finding.file).into(),
                ),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                    finding.start,
                ),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(finding.end),
            });
        }
        for (relative, text) in case.files {
            let path = format!("up017-target/{}/{}", case.name, relative);
            let destination = root.join(&path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(destination, text)?;
            files.push(ryo::linter::RelativePath(path.into()));
        }
    }
    files.sort();
    expected_py311.sort();
    if files.len() != 14 || expected_py311.is_empty() {
        return Err(Error::Invalid(
            "UP017 target guard needs all14 sources and live py311 positives".into(),
        ));
    }
    Ok((files, expected_py311))
}

fn target_configuration_cache_change(root: &std::path::Path) -> Result<bool, Error> {
    use ryo_benchmark_contracts::domain::requests::types::WorkerState;
    let rules = root.join("rules");
    let rule_directory = rules.join("UP017");
    std::fs::create_dir_all(&rule_directory)?;
    std::fs::write(
        rule_directory.join("query.ryoql"),
        include_str!("rules/UP017/query.ryoql"),
    )?;
    write_suppress_module(root)?;
    let (files, expected_py311) = stage_up017_target_files(root)?;
    let cache = root.join("retained-target-cache");
    let query_modules = suppress_modules();
    let run = |target, identity: &'static str, state| {
        factory::local(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(target),
            )]),
            None,
        )
        .run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(fixture_cohort(&["UP017"])),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: None,
            root: root.to_owned(),
            rules: rules.clone(),
            query_modules: query_modules.clone(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: Some(identity.to_owned()),
            source_language: None,
            files: files.clone(),
            state,
        })
    };
    for (target, identity, state, expected) in [
        (
            ryo_python::settings::PythonTargetVersion::Py310,
            "python-target=3.10",
            WorkerState::Fresh,
            Vec::new(),
        ),
        (
            ryo_python::settings::PythonTargetVersion::Py310,
            "python-target=3.10",
            WorkerState::Retained(cache.clone()),
            Vec::new(),
        ),
        (
            ryo_python::settings::PythonTargetVersion::Py311,
            "python-target=3.11",
            WorkerState::Retained(cache.clone()),
            expected_py311,
        ),
        (
            ryo_python::settings::PythonTargetVersion::Py310,
            "python-target=3.10",
            WorkerState::Retained(cache.clone()),
            Vec::new(),
        ),
    ] {
        let report = run(target, identity, state)?;
        let mut observed_files = report.files;
        observed_files.sort();
        let mut observed = report.diagnostics;
        observed.sort(); // Canonical diagnostic multiset: preserve every duplicate.
        if observed_files != files || observed != expected {
            return Err(Error::Invalid(format!(
                "UP017 full14-source target/cache guard differs at {identity}: files={observed_files:?}; expected={expected:?}; observed={observed:?}"
            )));
        }
    }
    Ok(true)
}

fn case_identities(
    cases: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
) -> BTreeMap<(String, String), ryo_benchmark_contracts::domain::workloads::types::PreparedCase> {
    cases
        .into_iter()
        .map(|case| ((case.rule.as_ref().to_owned(), case.name.0.clone()), case))
        .collect()
}

fn write_rule_profile(
    rules: &std::path::Path,
    rule: &str,
    query: &str,
    cases: &str,
) -> Result<(), Error> {
    let directory = rules.join(rule);
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("query.ryoql"), query)?;
    std::fs::write(directory.join("cases.toml"), cases)?;
    Ok(())
}

fn shared_suppression_fragment_matches_the_historical_inline_queries() -> Result<(), Error> {
    // E722 retains its historical inline-query baseline. F632 independently exercises
    // inline suppression with current operator-ID semantics: distinct chained operators
    // remain distinct findings even when they share the same source span.
    let temporary = tempfile::tempdir()?;
    let inline_profile = temporary.path().join("inline");
    let imported_profile = temporary.path().join("imported");
    let inline_rules = inline_profile.join("rules");
    let imported_rules = imported_profile.join("rules");
    let fixtures = [
        (
            "E722",
            include_str!("../../../tests/fixtures/ryoql-fragments/E722/query.ryoql"),
            include_str!("rules/E722/query.ryoql"),
            include_str!("rules/E722/cases.toml"),
        ),
        (
            "F632",
            include_str!("../../../tests/fixtures/ryoql-fragments/F632/query.ryoql"),
            include_str!("rules/F632/query.ryoql"),
            include_str!("rules/F632/cases.toml"),
        ),
    ];
    for (rule, baseline, imported, cases) in fixtures {
        write_rule_profile(&inline_rules, rule, baseline, cases)?;
        write_rule_profile(&imported_rules, rule, imported, cases)?;
    }
    write_suppress_module(&imported_profile)?;

    let suite = ryo_benchmark::outbound::suite::factory::local();
    let inline_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
        )
        .prepare_cases(&inline_rules, &temporary.path().join("inline-cases"))?;
    let imported_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
        )
        .prepare_cases(&imported_rules, &temporary.path().join("imported-cases"))?;
    let imported_by_name = case_identities(imported_cases);
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )]);
    let inline_engine = factory::local(languages.clone(), None);
    let imported_engine = factory::local(languages, None);
    let registry = suppress_modules();
    for case in inline_cases {
        let key = (case.rule.as_ref().to_owned(), case.name.0.clone());
        let imported = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported counterpart for {key:?}")))?;
        write_suppress_module(&imported.request.root)?;
        let state = ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh;
        let inline = inline_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(fixture_cohort(&["E722", "F632"])),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: None,
            root: case.request.root,
            rules: inline_rules.clone(),
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            source_language: None,
            files: case.request.files,
            state,
        })?;
        let fragmented = imported_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(fixture_cohort(&["E722", "F632"])),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: None,
            root: imported.request.root.clone(),
            rules: imported_rules.clone(),
            query_modules: registry.clone(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            source_language: None,
            files: imported.request.files.clone(),
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
        })?;
        assert_eq!(
            inline, fragmented,
            "inline and fragment reports differ for {}/{}",
            key.0, key.1
        );
    }
    for (rule, baseline, imported, cases) in fixtures {
        assert_single_rule_fragment_equivalence(temporary.path(), rule, baseline, imported, cases)?;
    }
    Ok(())
}

fn assert_single_rule_fragment_equivalence(
    root: &std::path::Path,
    rule: &str,
    baseline: &str,
    imported: &str,
    cases: &str,
) -> Result<(), Error> {
    let inline_rules = root.join(format!("single-inline/{rule}"));
    let imported_rules = root.join(format!("single-imported/{rule}"));
    for (directory, query) in [(&inline_rules, baseline), (&imported_rules, imported)] {
        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
        std::fs::write(directory.join("cases.toml"), cases)?;
    }
    let suite = ryo_benchmark::outbound::suite::factory::local();
    let inline_parent = inline_rules
        .parent()
        .ok_or_else(|| Error::Invalid("inline rule directory has no parent".to_owned()))?;
    let imported_parent = imported_rules
        .parent()
        .ok_or_else(|| Error::Invalid("imported rule directory has no parent".to_owned()))?;
    let inline_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
        )
        .prepare_cases(
            inline_parent,
            &root.join(format!("single-inline-cases-{rule}")),
        )?;
    let imported_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
        )
        .prepare_cases(
            imported_parent,
            &root.join(format!("single-imported-cases-{rule}")),
        )?;
    let imported_by_name: BTreeMap<_, _> = imported_cases
        .into_iter()
        .map(|case| ((case.name.0.clone(), case.rule.as_ref().to_owned()), case))
        .collect();
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )]);
    let inline_engine = factory::local(languages.clone(), None);
    let imported_engine = factory::local(languages, None);
    let profile_root = imported_parent
        .parent()
        .ok_or_else(|| Error::Invalid("imported profile has no root".to_owned()))?;
    write_suppress_module(profile_root)?;
    for case in inline_cases {
        let key = (case.name.0.clone(), case.rule.as_ref().to_owned());
        let imported_case = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported case for {key:?}")))?;
        let inline = inline_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(fixture_cohort(&[rule])),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: None,
            root: case.request.root,
            rules: inline_parent.to_path_buf(),
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            source_language: None,
            files: case.request.files,
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
        })?;
        let fragmented = imported_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(fixture_cohort(&[rule])),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: None,
            root: imported_case.request.root.clone(),
            rules: imported_parent.to_path_buf(),
            query_modules: suppress_modules(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            source_language: None,
            files: imported_case.request.files.clone(),
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
        })?;
        assert_eq!(inline, fragmented, "single-rule mismatch for {key:?}");
    }
    Ok(())
}
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn literal_properties_reach_engine_queries_for_true_and_false_boolean_nodes() -> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    let rule_directory = rules.join("LIT");
    std::fs::create_dir_all(&rule_directory)?;
    std::fs::write(
        rule_directory.join("query.ryoql"),
        "let truth = select id, file, span from \"literal-properties\" where kind = \"boolean\" and boolean = \"true\";\nemit truth subject file key(span);",
    )?;
    std::fs::write(root.path().join("true.py"), "value = True\n")?;
    std::fs::write(root.path().join("false.py"), "value = False\n")?;
    let report = factory::local(python_310(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["LIT"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: ["true.py", "false.py"]
            .into_iter()
            .map(|path| ryo::linter::RelativePath(path.into()))
            .collect(),
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    assert_eq!(report.files.len(), 2, "both files must be completely read");
    assert!(
        report
            .files
            .contains(&ryo::linter::RelativePath("true.py".into()))
    );
    assert!(
        report
            .files
            .contains(&ryo::linter::RelativePath("false.py".into()))
    );
    assert_eq!(report.diagnostics.len(), 1);
    let finding = report
        .diagnostics
        .first()
        .ok_or_else(|| Error::Invalid("the true literal should produce a finding".to_owned()))?;
    assert_eq!(finding.rule.as_ref(), "LIT");
    assert_eq!(finding.file.0.as_ref(), "true.py");
    assert_eq!(finding.start.0, 8);
    assert_eq!(finding.end.0, 12);
    Ok(())
}

fn decoded_numeric_payloads_and_strict_greater_composition_reach_engine_queries()
-> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    for (rule, query) in [
        (
            "EXACT",
            "let exact = select id, file, span from \"numeric-literal\" where kind = \"integer\" and numeric_gte(value, \"9007199254740993\"); emit exact subject file key(id);",
        ),
        (
            "STRICT",
            "let above = select id, file, span from \"numeric-literal\" where numeric_gte(value, 86400); let below = select id, file from \"numeric-literal\" where numeric_gte(86400, value); let strict = antijoin(left=above, right=below, left-on=[\"id\", \"file\"], right-on=[\"id\", \"file\"]); emit strict subject file key(id);",
        ),
        (
            "SUCCESSOR",
            "let adjacent = select id, file, span from \"numeric-literal\" where integer_successor(value, 9007199254740993); emit adjacent subject file key(id);",
        ),
    ] {
        let directory = rules.join(rule);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    let source = "0x10\n1_000\n9007199254740992\n9007199254740993\n86400.0\n86400.25\n86401.0\n1e9999\nTrue\n1j\n";
    std::fs::write(root.path().join("numeric.py"), source)?;
    let report = factory::local(python_310(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["EXACT", "STRICT", "SUCCESSOR"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("numeric.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    assert_eq!(
        report.files,
        [ryo::linter::RelativePath("numeric.py".into())]
    );
    assert!(report.skipped.is_empty());
    let mut actual = report
        .diagnostics
        .iter()
        .map(|finding| (finding.rule.as_ref(), finding.start.0, finding.end.0))
        .collect::<Vec<_>>();
    let mut expected = Vec::new();
    for (rule, text) in [
        ("EXACT", "9007199254740993"),
        ("SUCCESSOR", "9007199254740992"),
        ("STRICT", "9007199254740992"),
        ("STRICT", "9007199254740993"),
        ("STRICT", "86400.25"),
        ("STRICT", "86401.0"),
        ("STRICT", "1e9999"),
    ] {
        let start = source
            .find(text)
            .expect("independent numeric fixture occurrence");
        expected.push((rule, start, start + text.len()));
    }
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    Ok(())
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

#[path = "project_context_test.rs"]
mod project_context;

#[path = "compiler_feature_test.rs"]
mod compiler_features;

mod future_directives_test;

mod project_inputs_test;

mod i001_observation_test;

mod worker_request_input_test;

fn selected_consumer_keeps_unselected_producer_without_its_diagnostics() -> Result<(), Error> {
    use ryo::linter::{
        ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationName, ObservationProducer, RuleId,
    };
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    for (id, query) in [
        (
            "CONSUMER",
            "let found = select path, span from published; emit found subject path key(path);",
        ),
        (
            "PRODUCER",
            "let files = select file as path, span from \"semantic-syntax\" where kind = \"module\"; observe files as used; emit files subject path key(path);",
        ),
    ] {
        std::fs::create_dir_all(rules.join(id))?;
        std::fs::write(rules.join(id).join("query.ryoql"), query)?;
    }
    std::fs::write(root.path().join("case.py"), "value = 1\n")?;
    let consumer = RuleId::try_from("CONSUMER").unwrap();
    let producer = RuleId::try_from("PRODUCER").unwrap();
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(vec![consumer.clone()]),
        rule_observations: Some(
            BTreeMap::from([(
                consumer.clone(),
                vec![ObservationBinding {
                    name: ObservationBindingName("published".into()),
                    columns: vec![
                        ObservationColumn {
                            name: ObservationColumnName("path".into()),
                            kind: ObservationKind::File,
                        },
                        ObservationColumn {
                            name: ObservationColumnName("span".into()),
                            kind: ObservationKind::Span,
                        },
                    ],
                    producers: vec![ObservationProducer::Output {
                        rule: producer,
                        output: ObservationName("used".into()),
                    }],
                }],
            )])
            .into(),
        ),
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode: Default::default(),
        context_manifest: None,
        root: root.path().into(),
        rules,
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: Some("python-target=3.10".into()),
        source_language: None,
        files: vec![ryo::linter::RelativePath("case.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
            root.path().join("cache"),
        ),
    };
    let engine = factory::local(python_310(), None);
    for _ in 0..2 {
        let report = engine.run_prepared(request.clone())?;
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].rule, consumer);
    }
    let mut missing = request;
    std::fs::remove_file(missing.rules.join("PRODUCER/query.ryoql"))?;
    missing.state = ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh;
    assert!(
        matches!(engine.run_prepared(missing), Err(Error::Io(_))),
        "missing bound producer cannot become empty"
    );
    Ok(())
}

fn actual_i001_mixed_namespace_observation_stays_unknown_on_cold_and_warm_replay() {
    use ryo::linter::{
        Cache, CaseSource, Corpus, ExecutablePermission, GivenProjectInventory, Glob, LintRequest,
        Linter, MemoryBudget, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationKind, ObservationName, ObservationProducer,
        ProjectEntryObservation, ProjectEntryState, ProjectInventoryCompleteness,
        ProjectSearchScope, RelativePath, Rule, RuleApplicability, RuleId, SourceText, WorkerPool,
        Workers, lint_with,
    };
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let authored = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("benchmarks/linters/ruff/rules/I001/query.ryoql"),
    )
    .unwrap();
    let emission = authored.rfind("emit ").expect("actual I001 emission");
    let producer_query = format!(
        "{}let known_imports = select r.id as id, r.file as file, r.span as span from first_party_aliases as l join import_nodes as r on l.file = r.file; observe import_nodes as independent_imports; observe known_imports as namespace_imports; {}",
        &authored[..emission],
        &authored[emission..]
    );
    let profile: toml::Value = toml::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("benchmarks/linters/ruff/benchmark.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    let modules = profile["query-modules"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, path)| {
            ryo::query::QueryModuleSource::try_new(
                name,
                &std::fs::read_to_string(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("benchmarks/linters/ruff")
                        .join(path.as_str().unwrap()),
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let producer = RuleId::try_from("I001").unwrap();
    let independent = RuleId::try_from("INDEPENDENT").unwrap();
    let dependent = RuleId::try_from("DEPENDENT").unwrap();
    let modules = ryo::query::QueryModuleCatalog::try_new(modules).unwrap();
    let rule = |query: &str, observations| Rule {
        query: ryo::query::RyoQlQuery::with_modules("test/readiness", query, &modules).unwrap(),
        observations,
        applicability: RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let consumer = |output: &str| {
        rule(
            "let found = select id, file, span from incoming; emit found subject file key(id);",
            vec![ObservationBinding {
                name: ObservationBindingName("incoming".into()),
                columns: vec![
                    ObservationColumn {
                        name: ObservationColumnName("id".into()),
                        kind: ObservationKind::Node,
                    },
                    ObservationColumn {
                        name: ObservationColumnName("file".into()),
                        kind: ObservationKind::File,
                    },
                    ObservationColumn {
                        name: ObservationColumnName("span".into()),
                        kind: ObservationKind::Span,
                    },
                ],
                producers: vec![ObservationProducer::Output {
                    rule: producer.clone(),
                    output: ObservationName(output.into()),
                }],
            }],
        )
    };
    let mut producing = rule(&producer_query, Vec::new());
    producing.exclude.push(Glob::try_from("**").unwrap());
    let mut request = LintRequest {
        source_language: None,
        root: root.path().into(),
        project_search_scope: ProjectSearchScope::Declared(vec![RelativePath("".into())]),
        given_project_inventory: GivenProjectInventory {
            entries: vec![ProjectEntryObservation {
                path: RelativePath("local.py".into()),
                state: ProjectEntryState::File,
            }],
            completeness: ProjectInventoryCompleteness::Partial,
        },
        corpus: Corpus::Given(vec![CaseSource {
            path: RelativePath("case.py".into()),
            text: SourceText("import local\nimport unresolved\n".into()),
            executable: ExecutablePermission::Unknown,
        }]),
        rules: BTreeMap::from([
            (independent.clone(), consumer("independent_imports")),
            (dependent.clone(), consumer("namespace_imports")),
            (producer.clone(), producing),
        ]),
        files: Vec::new(),
        beside: Vec::new(),
        hosted: Vec::new(),
    };
    let engine = |cache| {
        Linter::new(
            python_310(),
            WorkerPool::from(Workers(std::num::NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let cached = engine(Cache::At(store.path().into()));
    for complete in [true, true, false, false, true] {
        request.given_project_inventory.completeness = if complete {
            ProjectInventoryCompleteness::CompleteWithinDeclaredScope
        } else {
            ProjectInventoryCompleteness::Partial
        };
        let fresh = lint_with(&engine(Cache::None), request.clone()).unwrap();
        let actual = lint_with(&cached, request.clone()).unwrap();
        assert_eq!(
            actual, fresh,
            "persisted readiness must agree with a fresh evaluation"
        );
        assert_eq!(
            actual
                .findings
                .iter()
                .filter(|finding| finding.rule == independent)
                .count(),
            2,
            "independent import root remains known"
        );
        assert!(
            !actual
                .unknowns
                .iter()
                .any(|unknown| unknown.rule == independent)
        );
        assert_eq!(
            actual
                .findings
                .iter()
                .filter(|finding| finding.rule == dependent)
                .count(),
            if complete { 2 } else { 0 }
        );
        assert_eq!(
            actual
                .unknowns
                .iter()
                .any(|unknown| unknown.rule == dependent),
            !complete,
            "filtered positive namespace rows do not erase the missing physical population"
        );
        assert!(
            !actual
                .findings
                .iter()
                .any(|finding| finding.rule == producer),
            "excluded Emit does not suppress demanded roots"
        );
    }
}
