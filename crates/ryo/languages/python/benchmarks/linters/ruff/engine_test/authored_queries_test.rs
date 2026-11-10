//! The authored Ruff queries, the suppression cohort among them, validate through the public SDK with the host relations and
//! observations the benchmark declares for them.
use std::path::Path;

pub(super) fn authored_suppression_cohort_queries_validate_through_public_sdk_contracts() {
    use ryo::query::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationScope, QueryModuleSource, RyoQlQuery, ValidateRequest,
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
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
        let answer = ryo::query::validate(&super::prepared_engine_test::python_310(), request);
        if let Err(error) = answer {
            failures.push(format!("{code}: {error:?}"));
        }
        if let Some(request) = undeclared {
            if let Ok(accepted) =
                ryo::query::validate(&super::prepared_engine_test::python_310(), request)
            {
                drop(accepted);
                failures.push("RUF100 undeclared root input was accepted".into());
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
