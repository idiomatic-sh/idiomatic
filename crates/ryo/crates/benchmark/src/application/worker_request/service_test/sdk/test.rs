//! A scenario of the `sdk` group: two cases of `worker_request` run through the real SDK: inputs declared missing are prepared once, and a prepared rule context
//! keeps its sibling rule's findings.

use super::super::super::service::{configure, prepare};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeMap;

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread` scenario.
    ActualSdkExecutesMissingInputFactsButRefusesUnmodeledProviderUnread,
    /// The `prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule` scenario.
    PreparedContextCountsDisabledPairsAndKeepsTheSiblingRule,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `; `.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread",
        ),
        word: Word::Succeeds,
        input: || Given::ActualSdkExecutesMissingInputFactsButRefusesUnmodeledProviderUnread,
        expect: Expect::Value(
            "{RelativePath(\"missing.py\")}; 1; RelativePath(\"missing.py\"); 0; 0; true; true",
        ),
    },
    Case {
        name: CaseName("prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule"),
        word: Word::Succeeds,
        input: || Given::PreparedContextCountsDisabledPairsAndKeepsTheSiblingRule,
        expect: Expect::Value(
            "2; 2; 4 pairs; 2; 1; true; 3 pairs; 2; 1; true; 3 pairs; 2; 0; 4 pairs; 2; 2; 4 pairs",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ActualSdkExecutesMissingInputFactsButRefusesUnmodeledProviderUnread => {
            actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread()
        }
        Given::PreparedContextCountsDisabledPairsAndKeepsTheSiblingRule => {
            prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread() -> String {
    let mut observed: Vec<String> = Vec::new();
    use crate::outbound::engine::interface::EnginePort as _;
    use ryo_benchmark_contracts::domain::context::types::CaseContext;
    let scratch = tempfile::tempdir().unwrap();
    let rules = scratch.path().join("rules");
    std::fs::create_dir_all(rules.join("R1")).unwrap();
    std::fs::write(rules.join("R1/query.ryoql"), "let failures = select path as file, span from \"filesystem.read-failure\"; emit failures subject file key(file);").unwrap();
    let boundary = scratch.path().join("snapshot");
    let manifest = crate::outbound::context::interface::ContextPort::materialize(
        &crate::outbound::context::factory::local(),
        &boundary,
        &CaseContext {
            logical_root: "project".into(),
            auxiliary_files: Default::default(),
            missing: vec!["project/missing.py".into()],
        },
        &Default::default(),
    )
    .unwrap();
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            vec![ryo::linter::RuleId::try_from("R1").unwrap()],
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: ProjectContextMode::ListedDisk,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(
            manifest,
        ),
        root: boundary.join("project"),
        rules,
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath("missing.py".into())],
        state: WorkerState::Fresh,
    };
    let engine = crate::outbound::engine::factory::local_with_frontend_diagnostic(
        ryo::linter::Languages::default(), None,
        crate::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from("P001").unwrap(),
            query: ryo::query::RyoQlQuery::try_from("let diagnostics = select id, file, span from \"syntax-error\"; emit diagnostics subject file key(id);").unwrap(),
        },
    );
    let prepared = prepare(request.clone()).unwrap();
    observed.push(format!("{:?}", prepared.missing.selected_missing));
    let session = engine.load(&prepared.request).unwrap();
    let configured = configure(&prepared.request, session).unwrap();
    let report = configured.execute().unwrap().report;
    observed.push(format!("{:?}", report.diagnostics.len()));
    observed.push(format!("{:?}", report.diagnostics[0].file));
    observed.push(format!("{:?}", report.diagnostics[0].start.0));
    observed.push(format!("{:?}", report.diagnostics[0].end.0));
    let mut readable = request;
    let source = [(
        ryo::linter::RelativePath("input.txt".into()),
        ryo::linter::SourceText("value = 1\n".into()),
    )]
    .into();
    readable.context_manifest =
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(
            crate::outbound::context::interface::ContextPort::materialize(
                &crate::outbound::context::factory::local(),
                &boundary,
                &CaseContext {
                    logical_root: "project".into(),
                    ..Default::default()
                },
                &source,
            )
            .unwrap(),
        );
    readable.files = vec![ryo::linter::RelativePath("input.txt".into())];
    let prepared = prepare(readable).unwrap();
    observed.push(format!("{}", prepared.missing.selected_missing.is_empty()));
    let session = engine.load(&prepared.request).unwrap();
    let configured = configure(&prepared.request, session).unwrap();
    observed.push(format!(
        "{}",
        configured
            .execute()
            .unwrap_err()
            .to_string()
            .contains("incomplete candidate")
    ));
    observed.join("; ")
}

fn prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule() -> String {
    let mut observed: Vec<String> = Vec::new();
    use crate::outbound::engine::interface::EnginePort as _;
    use ryo_benchmark_contracts::domain::context::types::{
        CaseContext, ExternalPrefix, PreparedRuleContext, RuleConfiguration, RuleContextTables,
        TableName,
    };
    use ryo_benchmark_contracts::domain::reports::types::CoverageAccounting;
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, ContextSeal, InputDeclaration, ParserSelection, ProviderSelection,
        RuleContextPreparation, SearchRoots,
    };
    let scratch = tempfile::tempdir().unwrap();
    let rules = scratch.path().join("rules");
    for (id, query) in [
        (
            "A001",
            "let failures = select path as file, span from \"filesystem.read-failure\"; emit failures subject file key(file);",
        ),
        (
            "B002",
            "let failures = select path as file, span from \"filesystem.read-failure\"; let configured = select code from \"config.selected\" where code = \"B002\"; let selected = join(left=failures, right=configured, left-on=[], right-on=[], output={file:[left,file],span:[left,span],code:[right,code]}); let disabled = select file, code from \"config.disabled\"; let enabled = antijoin(left=selected, right=disabled, left-on=[file,code], right-on=[file,code]); let external = select prefix from \"config.external\" where prefix = \"V\"; let found = join(left=enabled, right=external, left-on=[], right-on=[], output={file:[left,file],span:[left,span]}); emit found subject file key(file);",
        ),
    ] {
        std::fs::create_dir_all(rules.join(id)).unwrap();
        std::fs::write(rules.join(id).join("query.ryoql"), query).unwrap();
    }
    let boundary = scratch.path().join("snapshot");
    let manifest = crate::outbound::context::interface::ContextPort::materialize(
        &crate::outbound::context::factory::local(),
        &boundary,
        &CaseContext {
            logical_root: "project".into(),
            auxiliary_files: Default::default(),
            missing: vec!["project/missing.py".into(), "project/pkg/missing.py".into()],
        },
        &Default::default(),
    )
    .unwrap();
    let selected: Vec<_> = ["A001", "B002"]
        .into_iter()
        .map(|id| ryo::linter::RuleId::try_from(id).unwrap())
        .collect();
    let files = vec![
        ryo::linter::RelativePath("missing.py".into()),
        ryo::linter::RelativePath("pkg/missing.py".into()),
    ];
    let engine = crate::outbound::engine::factory::local(ryo::linter::Languages::default(), None);
    let store = scratch.path().join("store");
    for (prefix, disabled) in [
        ("V", false),
        ("V", true),
        ("V", true),
        ("W", false),
        ("V", false),
    ] {
        let request = WorkerRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Explicit(selected.clone()),
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Prepared(PreparedRuleContext {
                tables: RuleContextTables {
                    selected: TableName("config.selected".into()),
                    disabled: TableName("config.disabled".into()),
                    external: TableName("config.external".into()),
                },
                configuration: RuleConfiguration {
                    external_prefixes: vec![ExternalPrefix(prefix.into())],
                    disabled_rules: if disabled {
                        BTreeMap::from([(files[0].clone(), vec![selected[1].clone()])])
                    } else {
                        BTreeMap::new()
                    },
                },
            }),
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: ProjectContextMode::ListedDisk,
            context_manifest: ContextSeal::Sealed(manifest.clone()),
            root: boundary.join("project"),
            rules: rules.clone(),
            query_modules: Default::default(),
            rule_applicability: Default::default(),
            parser_configuration: ParserSelection::Unspecified,
            source_language: ProviderSelection::Unspecified,
            files: files.clone(),
            state: WorkerState::Retained(store.clone()),
        };
        let prepared = prepare(request).unwrap();
        let session = engine.load(&prepared.request).unwrap();
        let report = configure(&prepared.request, session)
            .unwrap()
            .execute()
            .unwrap()
            .report;
        let found = |rule: &ryo::linter::RuleId| {
            report
                .diagnostics
                .iter()
                .filter(|finding| &finding.rule == rule)
                .count()
        };
        observed.push(format!("{:?}", found(&selected[0])));
        observed.push(format!("{:?}", found(&selected[1])));
        if disabled {
            observed.push(format!(
                "{}",
                report
                    .diagnostics
                    .iter()
                    .all(|finding| finding.rule != selected[1] || finding.file == files[1])
            ));
        }
        observed.push(match &report.coverage {
            CoverageAccounting::Accounted(coverage) => format!("{} pairs", coverage.selected.0),
            CoverageAccounting::Unaccounted => "unaccounted".to_owned(),
        });
    }
    observed.join("; ")
}
