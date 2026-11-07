//! Declared configuration, hosted inputs and authored suppression cohorts through the public SDK contracts.
use super::super::prepared_engine_test::PreparedEngine as _;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn declared_configuration_uses_real_sdk_scope_and_hosted_inputs_on_unchanged_source()
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
        selected: ryo_benchmark_contracts::domain::context::types::TableName("config.selected".into()),
        disabled: ryo_benchmark_contracts::domain::context::types::TableName("config.disabled".into()),
        external: ryo_benchmark_contracts::domain::context::types::TableName("config.external".into()),
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
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(selected.clone()),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Prepared(PreparedRuleContext {
                tables: tables.clone(),
                configuration: RuleConfiguration {
                    external_prefixes: vec![ryo_benchmark_contracts::domain::context::types::ExternalPrefix(prefix.into())],
                    disabled_rules,
                },
            }),
            project_context_mode: ProjectContextMode::Isolated,
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            root: corpus.clone(),
            rules: rules.clone(),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::new(),
            parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName("python-target=3.10".into())),
            source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
            files: files.clone(),
            state: WorkerState::Retained(root.path().join("cache")),
        };
        let report = factory::local(super::super::prepared_engine_test::python_310(), None)
            .run_prepared(request)?;
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
            match report.coverage {
                ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                    coverage,
                ) => Some(coverage.selected.0),
                ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted => None,
            },
            Some(if exclude { 3 } else { 4 })
        );
    }
    assert_eq!(
        std::fs::metadata(corpus.join("input.py"))?.modified()?,
        modified
    );
    Ok(())
}

pub(super) fn authored_suppression_cohort_queries_validate_through_public_sdk_contracts() {
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
        let answer =
            ryo::query::validate(&super::super::prepared_engine_test::python_310(), request);
        if let Err(error) = answer {
            failures.push(format!("{code}: {error:?}"));
        }
        if let Some(request) = undeclared {
            if let Ok(accepted) =
                ryo::query::validate(&super::super::prepared_engine_test::python_310(), request)
            {
                drop(accepted);
                failures.push("RUF100 undeclared root input was accepted".into());
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
