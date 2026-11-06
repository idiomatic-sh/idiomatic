//! Real SDK evidence for missing files, unread or partial inputs and typed findings.
use boundary_failure_test::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(crate) mod boundary_failure_test;
pub(crate) mod constants;
mod prepared_engine_test;
use prepared_engine_test::{
    PreparedEngine as _, custom_frontend_projection, default_frontend_projection, python_310,
};

mod scenario_test;
use scenario_test::EngineScenario;
mod cache_change_test;
use cache_change_test::{
    projection_cache_query_change, query_module_inline_equivalence,
    target_configuration_cache_change,
};
mod suppression_fragment_test;
use suppression_fragment_test::shared_suppression_fragment_matches_the_historical_inline_queries;
mod literal_properties_test;
use literal_properties_test::{
    decoded_numeric_payloads_and_strict_greater_composition_reach_engine_queries,
    literal_properties_reach_engine_queries_for_true_and_false_boolean_nodes,
};
mod consumer_selection_test;
use consumer_selection_test::{
    actual_i001_mixed_namespace_observation_stays_unknown_on_cold_and_warm_replay,
    selected_consumer_keeps_unselected_producer_without_its_diagnostics,
};
mod rule_fixtures_test;
use rule_fixtures_test::{modules_for, write_rule_fixtures};
mod rejected_syntax_test;
use rejected_syntax_test::rejected_syntax_with_provider_partials_skips_only_accepted_syntax_rules;
mod scenario_expectations_test;
use scenario_expectations_test::{expected_diagnostics, input_file, source_for};

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

fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
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

pub mod worker_request_input_test;
