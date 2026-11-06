//! Selected consumers and the I001 namespace observation across cold and warm replay.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::python_310;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn selected_consumer_keeps_unselected_producer_without_its_diagnostics()
-> Result<(), Error> {
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

pub(super) fn actual_i001_mixed_namespace_observation_stays_unknown_on_cold_and_warm_replay() {
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
        source_language: ryo::linter::SourceLanguage::Recognized,
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
