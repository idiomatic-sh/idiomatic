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

/// A syntax-rejected file beside a valid one, as Django ships one: the pytest decorator rules PT020, PT025 and PT026 skip the
/// rejected file and answer the valid one with Ruff 0.16.5's findings. While their joins on a byte offset were not keyed by file,
/// their plans ran across files and the one rejected file left each of them Unknown at every file.
pub(super) fn a_rejected_file_leaves_pt020_pt025_and_pt026_answering_the_valid_file()
-> Result<(), Error> {
    use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, SourceByteOffset};
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    super::rule_fixtures_test::write_suppress_module(root.path())?;
    for (rule, query) in [
        ("PT020", include_str!("../rules/PT020/query.ryoql")),
        ("PT025", include_str!("../rules/PT025/query.ryoql")),
        ("PT026", include_str!("../rules/PT026/query.ryoql")),
    ] {
        let directory = rules.join(rule);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    std::fs::write(
        root.path().join("case.py"),
        "import pytest\n@pytest.yield_fixture\ndef resource():\n    yield 1\n@pytest.mark.usefixtures(\"resource\")\n@pytest.fixture\ndef combined():\n    return 1\n@pytest.mark.usefixtures()\ndef test_case():\n    pass\n",
    )?;
    std::fs::write(root.path().join("broken.py"), "def broken(:\n    pass\n")?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["PT020", "PT025", "PT026"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: super::prepared_engine_test::suppress_modules(),
        rule_applicability: BTreeMap::from([
            ("PT020".to_owned(), "accepted-syntax".to_owned()),
            ("PT025".to_owned(), "accepted-syntax".to_owned()),
            ("PT026".to_owned(), "accepted-syntax".to_owned()),
        ]),
        parser_configuration: Some("python-target=3.10".to_owned()),
        source_language: None,
        files: vec![
            ryo::linter::RelativePath("broken.py".into()),
            ryo::linter::RelativePath("case.py".into()),
        ],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let report =
        factory::local_with_frontend_diagnostic(python_310(), None, default_frontend_projection()?)
            .run_prepared(request)?;
    let finding =
        |rule: &str, file: &str, start: usize, end: usize| -> Result<BenchmarkFinding, Error> {
            Ok(BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from(rule)
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                file: ryo::linter::RelativePath(file.into()),
                start: SourceByteOffset(start),
                end: SourceByteOffset(end),
            })
        };
    let mut expected = vec![
        finding("invalid-syntax", "broken.py", 11, 12)?,
        finding("invalid-syntax", "broken.py", 12, 13)?,
        finding("PT020", "case.py", 14, 35)?,
        finding("PT025", "case.py", 64, 100)?,
        finding("PT026", "case.py", 146, 172)?,
    ];
    expected.sort();
    let mut actual = report.diagnostics;
    actual.sort();
    assert_eq!(
        actual, expected,
        "each rule answers the valid file as Ruff 0.16.5 does"
    );
    assert_eq!(
        report.skipped.len(),
        3,
        "the rejected file is skipped by each rule, not Unknown"
    );
    Ok(())
}

/// The rules whose plans cross files by design (I001 reads `project-namespace-membership`, PLW0406 `module-identity`) and RUF100,
/// which consumes their `noqa_matches`, beside a syntax-rejected file: the lineage of their cross-file steps keeps the rejected
/// file's missing input in that file, so the valid file answers Ruff 0.16.5's findings (I001 and PLW0406 on the import of the
/// module itself, RUF100 on the unused `noqa`), and all three skip the rejected file and are undecided nowhere. Before the
/// lineage, the one rejected file left all three Unknown at every file.
pub(super) fn a_rejected_file_leaves_i001_plw0406_and_ruf100_answering_the_valid_file() {
    use super::constants::{
        BENCHMARK_TOML, BENCHMARKS_LINTERS_RUFF, CODE, CONFIG_DISABLED, CONFIG_EXTERNAL,
        CONFIG_SELECTED, DIRECTIVE, EMPTY, FILE, NOQA_MATCHES, PREFIX, QUERY_MODULES,
        RULES_I001_QUERY_RYOQL, RULES_RUF100_QUERY_RYOQL, SUPPRESSION_MATCHES,
    };
    use ryo::linter::{
        Cache, CaseSource, Cell, Corpus, ExecutablePermission, GivenProjectInventory, HostColumn,
        HostColumnName, HostKind, HostRelation, HostRelationName, HostRow, HostText, LintRequest,
        Linter, MemoryBudget, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationKind, ObservationName, ObservationProducer,
        ProjectEntryObservation, ProjectEntryState, ProjectInventoryCompleteness,
        ProjectSearchScope, RelativePath, Rule, RuleApplicability, RuleId, SourceText, WorkerPool,
        Workers, lint_with,
    };
    const CASE: &str = "pkg/case.py";
    const PACKAGE: &str = "pkg/__init__.py";
    const BROKEN: &str = "broken.py";
    let root = tempfile::tempdir().unwrap();
    let authored = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BENCHMARKS_LINTERS_RUFF);
    let profile: toml::Value =
        toml::from_str(&std::fs::read_to_string(authored.join(BENCHMARK_TOML)).unwrap()).unwrap();
    let modules = profile[QUERY_MODULES]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, path)| {
            ryo::query::QueryModuleSource::try_new(
                name,
                &std::fs::read_to_string(authored.join(path.as_str().unwrap())).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let modules = ryo::query::QueryModuleCatalog::try_new(modules).unwrap();
    let i001 = RuleId::try_from("I001").unwrap();
    let plw0406 = RuleId::try_from("PLW0406").unwrap();
    let ruf100 = RuleId::try_from("RUF100").unwrap();
    let rule = |name: &str, query: &str, applicability, observations| Rule {
        query: ryo::query::RyoQlQuery::with_modules(name, query, &modules).unwrap(),
        observations,
        applicability,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let read = |path: &str| std::fs::read_to_string(authored.join(path)).unwrap();
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName(DIRECTIVE.into()),
            kind: ObservationKind::Node,
        },
        ObservationColumn {
            name: ObservationColumnName(FILE.into()),
            kind: ObservationKind::File,
        },
        ObservationColumn {
            name: ObservationColumnName(CODE.into()),
            kind: ObservationKind::Symbol,
        },
    ];
    let consumed = vec![ObservationBinding {
        name: ObservationBindingName(SUPPRESSION_MATCHES.into()),
        columns,
        producers: [&i001, &plw0406]
            .into_iter()
            .map(|producer| ObservationProducer::Output {
                rule: producer.clone(),
                output: ObservationName(NOQA_MATCHES.into()),
            })
            .collect(),
    }];
    let text = |code: &str| HostRow(vec![Cell::Text(HostText(code.into()))]);
    let hosted = vec![
        HostRelation {
            name: HostRelationName(CONFIG_SELECTED.into()),
            columns: vec![HostColumn {
                name: HostColumnName(CODE.into()),
                kind: HostKind::Text,
            }],
            rows: ["I001", "PLW0406", "RUF100"]
                .into_iter()
                .map(text)
                .collect(),
        },
        HostRelation {
            name: HostRelationName(CONFIG_DISABLED.into()),
            columns: vec![
                HostColumn {
                    name: HostColumnName(FILE.into()),
                    kind: HostKind::File,
                },
                HostColumn {
                    name: HostColumnName(CODE.into()),
                    kind: HostKind::Text,
                },
            ],
            rows: Vec::new(),
        },
        HostRelation {
            name: HostRelationName(CONFIG_EXTERNAL.into()),
            columns: vec![HostColumn {
                name: HostColumnName(PREFIX.into()),
                kind: HostKind::Text,
            }],
            rows: Vec::new(),
        },
    ];
    let source = |path: &str, text: &str| CaseSource {
        path: RelativePath(path.into()),
        text: SourceText(text.into()),
        executable: ExecutablePermission::Unknown,
    };
    let request = LintRequest {
        source_language: ryo::linter::SourceLanguage::Recognized,
        root: root.path().into(),
        project_search_scope: ProjectSearchScope::Declared(vec![RelativePath(EMPTY.into())]),
        given_project_inventory: GivenProjectInventory {
            entries: [CASE, PACKAGE, BROKEN]
                .into_iter()
                .map(|path| ProjectEntryObservation {
                    path: RelativePath(path.into()),
                    state: ProjectEntryState::File,
                })
                .collect(),
            completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
        },
        corpus: Corpus::Given(vec![
            source(BROKEN, "def broken(:\n    pass\n"),
            source(PACKAGE, ""),
            source(
                CASE,
                "import sys\nimport os\nimport pkg.case\n\nvalue = 1  # noqa: E501\n",
            ),
        ]),
        rules: BTreeMap::from([
            (
                i001.clone(),
                rule(
                    "test/I001",
                    &read(RULES_I001_QUERY_RYOQL),
                    RuleApplicability::AcceptedSyntax,
                    Vec::new(),
                ),
            ),
            (
                plw0406.clone(),
                rule(
                    "test/PLW0406",
                    include_str!("../rules/PLW0406/query.ryoql"),
                    RuleApplicability::AcceptedSyntax,
                    Vec::new(),
                ),
            ),
            (
                ruf100.clone(),
                rule(
                    "test/RUF100",
                    &read(RULES_RUF100_QUERY_RYOQL),
                    // RUF100 takes its producers' pin: it cannot answer where they skip.
                    RuleApplicability::AcceptedSyntax,
                    consumed,
                ),
            ),
        ]),
        files: Vec::new(),
        beside: Vec::new(),
        hosted,
    };
    let linter = Linter::new(
        python_310(),
        WorkerPool::from(Workers(std::num::NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::None,
    );
    let report = lint_with(&linter, request).unwrap();
    let at_case = |rule: &RuleId| {
        report
            .findings
            .iter()
            .filter(|finding| {
                &finding.rule == rule
                    && finding.subject
                        == ryo::linter::Slot::Is(Cell::File(RelativePath(CASE.into())))
            })
            .count()
    };
    // Ruff 0.16.5 on the same files: I001 at 1:1-3:16, PLW0406 at 3:8-3:16, RUF100 at 5:12-5:24 of pkg/case.py.
    for rule in [&i001, &plw0406, &ruf100] {
        assert_eq!(
            at_case(rule),
            1,
            "{rule:?} answers the valid file as Ruff does"
        );
    }
    assert!(
        report.unknowns.is_empty(),
        "the rejected file leaves no rule undecided anywhere: {:?}",
        report.unknowns
    );
    for rule in [&i001, &plw0406, &ruf100] {
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| &skip.rule == rule && skip.path == RelativePath(BROKEN.into())),
            "{rule:?} skips the rejected file"
        );
    }
}
