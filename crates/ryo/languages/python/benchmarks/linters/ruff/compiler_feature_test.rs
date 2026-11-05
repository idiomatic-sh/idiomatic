//! Actual local-adapter checks of canonical module features and annotation evaluation positions.
use super::PreparedEngine as _;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// One independently expected context; pinned competitor diagnostics are not rewritten.
#[derive(serde::Deserialize)]
struct ContextCase {
    /// Stable identity for failure messages and isolated source directories.
    name: String,
    /// Complete valid source to analyze.
    source: String,
    /// Exact existing syntax occurrence selected by real byte locations.
    needle: String,
    /// Expected semantic node kind for that occurrence.
    kind: String,
    /// Explicit parser target.
    target: String,
    /// Source filename; a stub suffix must not invent a future feature.
    file: String,
    /// Whether the validated module future flag is enabled.
    feature: bool,
    /// Direct annotation evaluation context, or empty for a different context.
    evaluation: String,
}

/// Read the actual source assets, not a second implementation or a historical helper snapshot.
fn stage_modules(root: &Path) -> Result<BTreeMap<String, PathBuf>, Error> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff/fragments");
    std::fs::create_dir_all(root.join("fragments"))?;
    let mut modules = BTreeMap::new();
    for name in ["future-annotation-context", "annotation-evaluation-context"] {
        let relative = PathBuf::from(format!("fragments/{name}.ryoql"));
        let _copied = std::fs::copy(source.join(format!("{name}.ryoql")), root.join(&relative))?;
        let _previous = modules.insert(format!("python/ruff/{name}"), relative);
    }
    Ok(modules)
}

/// Select one real occurrence, with an independent INPUT finding preventing empty-input passes.
fn write_queries(root: &Path, case: &ContextCase, start: usize, end: usize) -> Result<(), Error> {
    let input = format!(
        "let nodes = select id, kind, file, span from \"semantic-syntax\" where kind = {:?};\nlet locations = select id, \"start-byte\" as start, \"end-byte\" as end, file from \"source-location\";\nlet selected = select l.id as id, l.file as file, l.span as span from nodes as l join locations as r on l.id = r.id and l.file = r.file where r.start = {start} and r.end = {end};\nlet inputs = select id, file from selected;\n",
        case.kind
    );
    for (rule, evaluation) in [
        ("INPUT", ""),
        ("FLAG", ""),
        ("EAGER", "eager"),
        ("POSTPONED", "postponed"),
        ("DEFERRED", "deferred"),
        ("UNEVALUATED", "unevaluated"),
    ] {
        let directory = root.join("rules").join(rule);
        std::fs::create_dir_all(&directory)?;
        let query = if rule == "INPUT" {
            format!("{input}emit selected subject file key(span);\n")
        } else {
            let (import, matches) = if rule == "FLAG" {
                (
                    "import mode from \"python/ruff/future-annotation-context\";",
                    "let matched = mode(expressions=inputs);".to_owned(),
                )
            } else {
                (
                    "import mode from \"python/ruff/annotation-evaluation-context\";",
                    format!(
                        "let contexts = mode(expressions=inputs);\nlet matched = select id, file from contexts where evaluation = {evaluation:?};"
                    ),
                )
            };
            format!(
                "{import}\n{input}{matches}\nlet rows = select l.id as id, l.file as file, l.span as span from selected as l join matched as r on l.id = r.id and l.file = r.file;\nemit rows subject file key(span);\n"
            )
        };
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    Ok(())
}

/// The expected compiler mode and context are asserted through the actual SDK execution path.
fn check_case(
    root: &Path,
    modules: &BTreeMap<String, PathBuf>,
    case: &ContextCase,
) -> Result<(), Error> {
    use ryo_python::settings::{PythonParserSettings, PythonTargetVersion};
    let target = match case.target.as_str() {
        "py313" => PythonTargetVersion::Py313,
        "py314" => PythonTargetVersion::Py314,
        "py315" => PythonTargetVersion::Py315,
        _ => return Err(Error::Invalid("unknown context-test parser target".into())),
    };
    let start = case
        .source
        .rfind(&case.needle)
        .ok_or_else(|| Error::Invalid("the test occurrence must exist".into()))?;
    let end = start + case.needle.len();
    write_queries(root, case, start, end)?;
    let source = root.join(&case.name);
    std::fs::create_dir_all(&source)?;
    std::fs::write(source.join(&case.file), &case.source)?;
    let report = factory::local(
        ryo::linter::Languages(vec![ryo_python::linked_with_settings(
            PythonParserSettings::new(target),
        )]),
        None,
    )
    .run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(super::fixture_cohort(&[
            "INPUT",
            "FLAG",
            "EAGER",
            "POSTPONED",
            "DEFERRED",
            "UNEVALUATED",
        ])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode: ProjectContextMode::Isolated,
        context_manifest: None,
        root: source,
        rules: root.join("rules"),
        query_modules: modules.clone(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath(case.file.clone().into())],
        state: WorkerState::Fresh,
    })?;
    let coverage = report
        .coverage
        .ok_or_else(|| Error::Invalid("context helper coverage is required".into()))?;
    assert_eq!(
        (
            coverage.selected,
            coverage.evaluated,
            coverage.skipped,
            coverage.unknown_or_unread
        ),
        (6, 6, 0, 0),
        "{}",
        case.name
    );
    assert!(report.skipped.is_empty());
    let mut actual = report
        .diagnostics
        .iter()
        .map(|finding| {
            (
                finding.rule.as_ref().to_owned(),
                finding.start.0,
                finding.end.0,
            )
        })
        .collect::<Vec<_>>();
    let mut expected = vec![("INPUT".to_owned(), start, end)];
    if case.feature {
        expected.push(("FLAG".to_owned(), start, end));
    }
    if !case.evaluation.is_empty() {
        expected.push((case.evaluation.to_uppercase(), start, end));
    }
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected, "{}: {}", case.name, case.source);
    Ok(())
}

#[test]
fn compiler_features_and_annotation_contexts_follow_the_language_contract() -> Result<(), Error> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/compiler-features/annotation-contexts.json");
    let cases: Vec<ContextCase> = serde_json::from_slice(&std::fs::read(source)?)
        .map_err(|error| Error::Json(Box::new(error)))?;
    assert!(
        cases.len() >= 40,
        "positive and negative contexts must remain nonvacuous"
    );
    let temporary = tempfile::tempdir()?;
    let modules = stage_modules(temporary.path())?;
    for case in &cases {
        check_case(temporary.path(), &modules, case)?;
    }
    Ok(())
}

#[test]
fn declared_configuration_uses_real_sdk_scope_and_hosted_inputs_on_unchanged_source()
-> Result<(), Error> {
    use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
    use ryo_benchmark_contracts::domain::context::types::RuleConfiguration;
    use ryo_benchmark_contracts::domain::context::types::RuleContextTables;
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    let corpus = root.path().join("corpus");
    std::fs::create_dir_all(corpus.join("pkg"))?;
    for file in ["input.py", "pkg/input.py"] {
        std::fs::write(corpus.join(file), "def f():\n    pass\n")?;
    }
    let modified = std::fs::metadata(corpus.join("input.py"))?.modified()?;
    for (id, query) in [
        (
            "A001",
            "let functions = select id, file, span from \"function\"; emit functions subject file key(span);",
        ),
        (
            "B002",
            "let functions = select id, file, span from \"function\"; let configured = select code from \"config.selected\" where code = \"B002\"; let selected = join(left=functions, right=configured, left-on=[], right-on=[], output={id:[left,id],file:[left,file],span:[left,span],code:[right,code]}); let disabled = select file, code from \"config.disabled\"; let enabled = antijoin(left=selected, right=disabled, left-on=[file,code], right-on=[file,code]); let external = select prefix from \"config.external\" where prefix = \"V\"; let found = join(left=enabled, right=external, left-on=[], right-on=[], output={id:[left,id],file:[left,file],span:[left,span]}); emit found subject file key(span);",
        ),
    ] {
        std::fs::create_dir_all(rules.join(id))?;
        std::fs::write(rules.join(id).join("query.ryoql"), query)?;
    }
    let selected = ["A001", "B002"]
        .into_iter()
        .map(ryo::linter::RuleId::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let files = ["input.py", "pkg/input.py"]
        .into_iter()
        .map(|file| ryo::linter::RelativePath(file.into()))
        .collect::<Vec<_>>();
    let tables = RuleContextTables {
        selected: "config.selected".into(),
        disabled: "config.disabled".into(),
        external: "config.external".into(),
    };
    for (prefix, exclude, expected) in [
        ("V", false, 2),
        ("V", true, 1),
        ("V", true, 1),
        ("W", false, 0),
        ("V", false, 2),
    ] {
        let disabled_rules = if exclude {
            BTreeMap::from([(files[0].clone(), vec![selected[1].clone()])])
        } else {
            BTreeMap::new()
        };
        let request = WorkerRequest {
            performance: Default::default(),
            project_search_roots: None,
            given_project_inventory: Default::default(),
            selected_rules: Some(selected.clone()),
            rule_observations: None,
            rule_context: Some(PreparedRuleContext {
                tables: tables.clone(),
                configuration: RuleConfiguration {
                    external_prefixes: vec![prefix.into()],
                    disabled_rules,
                },
            }),
            project_context_mode: ProjectContextMode::Isolated,
            context_manifest: None,
            root: corpus.clone(),
            rules: rules.clone(),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::new(),
            parser_configuration: Some("python-target=3.10".into()),
            source_language: None,
            files: files.clone(),
            state: WorkerState::Retained(root.path().join("cache")),
        };
        let report = factory::local(super::python_310(), None).run_prepared(request)?;
        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|finding| finding.rule == selected[0])
                .count(),
            2
        );
        let configured: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|finding| finding.rule == selected[1])
            .collect();
        assert_eq!(
            configured.len(),
            expected,
            "prefix={prefix}, exclude={exclude}"
        );
        if exclude {
            assert_eq!(configured[0].file.0.as_ref(), "pkg/input.py");
        }
        assert_eq!(
            report.coverage.as_ref().unwrap().selected,
            if exclude { 3 } else { 4 }
        );
    }
    assert_eq!(
        std::fs::metadata(corpus.join("input.py"))?.modified()?,
        modified
    );
    Ok(())
}

#[test]
fn authored_suppression_cohort_queries_validate_through_public_sdk_contracts() {
    use ryo::query::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationScope, QueryModuleSource, RyoQlQuery, ValidateRequest,
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
    let profile: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.join("benchmark.toml")).unwrap()).unwrap();
    let modules = profile["query-modules"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, path)| {
            QueryModuleSource::try_new(
                name,
                &std::fs::read_to_string(root.join(path.as_str().unwrap())).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let modules = ryo::query::QueryModuleCatalog::try_new(modules).unwrap();
    let mut failures = Vec::new();
    for code in profile["defaults"]
        .as_array()
        .unwrap()
        .iter()
        .chain(profile["supporting-rules"].as_array().unwrap().iter())
        .map(|value| value.as_str().unwrap())
    {
        let text = std::fs::read_to_string(root.join(format!("rules/{code}/query.ryoql"))).unwrap();
        let query = RyoQlQuery::with_modules(&format!("test/{code}"), &text, &modules).unwrap();
        let request = ValidateRequest {
            query,
            hosted: if code == "RUF100" {
                vec![
                    ryo::linter::HostDeclaration {
                        name: ryo::linter::HostRelationName("config.external".into()),
                        columns: vec![ryo::linter::HostColumn {
                            name: ryo::linter::HostColumnName("prefix".into()),
                            kind: ryo::linter::HostKind::Text,
                        }],
                    },
                    ryo::linter::HostDeclaration {
                        name: ryo::linter::HostRelationName("config.selected".into()),
                        columns: vec![ryo::linter::HostColumn {
                            name: ryo::linter::HostColumnName("code".into()),
                            kind: ryo::linter::HostKind::Text,
                        }],
                    },
                    ryo::linter::HostDeclaration {
                        name: ryo::linter::HostRelationName("config.disabled".into()),
                        columns: vec![
                            ryo::linter::HostColumn {
                                name: ryo::linter::HostColumnName("file".into()),
                                kind: ryo::linter::HostKind::File,
                            },
                            ryo::linter::HostColumn {
                                name: ryo::linter::HostColumnName("code".into()),
                                kind: ryo::linter::HostKind::Text,
                            },
                        ],
                    },
                ]
            } else {
                Vec::new()
            },
            observations: if code == "RUF100" {
                vec![BoundObservationInput {
                    name: ObservationBindingName("suppression_matches".into()),
                    columns: vec![
                        ObservationColumn {
                            name: ObservationColumnName("directive".into()),
                            kind: ObservationKind::Node,
                        },
                        ObservationColumn {
                            name: ObservationColumnName("file".into()),
                            kind: ObservationKind::File,
                        },
                        ObservationColumn {
                            name: ObservationColumnName("code".into()),
                            kind: ObservationKind::Symbol,
                        },
                    ],
                    scope: ObservationScope::CrossFile,
                }]
            } else {
                Vec::new()
            },
        };
        let undeclared = (code == "RUF100").then(|| ValidateRequest {
            query: request.query.clone(),
            hosted: request.hosted.clone(),
            observations: Vec::new(),
        });
        let answer = ryo::query::validate(&super::python_310(), request);
        if let Err(error) = answer {
            failures.push(format!("{code}: {error:?}"));
        }
        if let Some(request) = undeclared {
            if ryo::query::validate(&super::python_310(), request).is_ok() {
                failures.push("RUF100 undeclared root input was accepted".into());
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
