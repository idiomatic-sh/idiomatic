//! Two cases of `worker_request` run through the real SDK: inputs declared missing are prepared once, and a prepared rule context
//! keeps its sibling rule's findings.
use super::super::service::{configure, prepare};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeMap;

pub(in super::super) fn actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread()
 {
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
    assert_eq!(
        prepared.missing.selected_missing,
        request.files.iter().cloned().collect()
    );
    let session = engine.load(&prepared.request).unwrap();
    let configured = configure(&prepared.request, session).unwrap();
    let report = configured.execute().unwrap().report;
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].file, request.files[0]);
    assert_eq!(report.diagnostics[0].start.0, 0);
    assert_eq!(report.diagnostics[0].end.0, 0);
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
    assert!(prepared.missing.selected_missing.is_empty());
    let session = engine.load(&prepared.request).unwrap();
    let configured = configure(&prepared.request, session).unwrap();
    assert!(
        configured
            .execute()
            .unwrap_err()
            .to_string()
            .contains("incomplete candidate"),
        "a readable source refused by all providers is not a declared input absence"
    );
}

/// Two rules over two declared-missing files with a prepared rule context: the hosted tables and a disabled pair reach the SDK, the
/// selected pairs the engine is given are the rules times the files less the disabled pairs, and the rule nothing disables keeps both
/// findings on a retained store while the other rule's exclusion and hosted rows change.
pub(in super::super) fn prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule() {
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
    for (prefix, disabled, sibling, configured, pairs) in [
        ("V", false, 2, 2, 4),
        ("V", true, 2, 1, 3),
        ("V", true, 2, 1, 3),
        ("W", false, 2, 0, 4),
        ("V", false, 2, 2, 4),
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
        let case = format!("prefix={prefix}, disabled={disabled}");
        assert_eq!(found(&selected[0]), sibling, "{case}");
        assert_eq!(found(&selected[1]), configured, "{case}");
        if disabled {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .all(|finding| finding.rule != selected[1] || finding.file == files[1]),
                "{case}"
            );
        }
        assert!(
            matches!(&report.coverage, CoverageAccounting::Accounted(coverage) if coverage.selected.0 == pairs),
            "{case}: {:?}",
            report.coverage
        );
    }
}
