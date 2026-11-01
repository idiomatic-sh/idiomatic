//! Real SDK evidence for missing files, unread or partial inputs and typed findings.
use super::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::BenchmarkFinding},
    suite::types::WorkerRequest,
};
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort as _};
use ryo_benchmark::outbound::suite::interface::SuitePort as _;

#[derive(Clone, Copy)]
enum EngineScenario {
    Finding,
    Missing,
    Unread,
    Partial,
    InvalidSyntax,
    InvalidSyntaxProjectionDisabled,
    CustomFrontendProjection,
    FutureFeatureProjection,
    UnknownFutureWithoutConsumer,
    FrontendRuleCollision,
    ProjectionCacheQueryChange,
    QueryModuleInlineEquivalence,
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
];
#[expect(
    clippy::expect_used,
    reason = "the fixed fixture rule identifier must be valid for engine assertions"
)]
fn exercise(scenario: Input) -> Result<Value, Error> {
    let root = tempfile::tempdir()?;
    if matches!(scenario, EngineScenario::QueryModuleInlineEquivalence) {
        return query_module_inline_equivalence(root.path());
    }
    let rules = root.path().join("rules");
    std::fs::create_dir_all(rules.join("E722"))?;
    std::fs::write(
        rules.join("E722/query.ryoql"),
        include_str!("rules/E722/query.ryoql"),
    )?;
    if matches!(scenario, EngineScenario::FutureFeatureProjection) {
        std::fs::create_dir_all(rules.join("F407"))?;
        std::fs::write(
            rules.join("F407/query.ryoql"),
            include_str!("rules/F407/candidate.ryoql"),
        )?;
    }
    if matches!(scenario, EngineScenario::FrontendRuleCollision) {
        std::fs::create_dir_all(rules.join("invalid-syntax"))?;
        std::fs::write(
            rules.join("invalid-syntax/query.ryoql"),
            include_str!("rules/E722/query.ryoql"),
        )?;
    }
    let file = match scenario {
        EngineScenario::Finding
        | EngineScenario::Partial
        | EngineScenario::InvalidSyntax
        | EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::CustomFrontendProjection
        | EngineScenario::FutureFeatureProjection
        | EngineScenario::UnknownFutureWithoutConsumer
        | EngineScenario::FrontendRuleCollision
        | EngineScenario::ProjectionCacheQueryChange
        | EngineScenario::QueryModuleInlineEquivalence => "input.py",
        EngineScenario::Missing => "missing.py",
        EngineScenario::Unread => "input.unknown",
    };
    if !matches!(scenario, EngineScenario::Missing) {
        std::fs::write(
            root.path().join(file),
            if matches!(scenario, EngineScenario::Partial) {
                "try:\n    pass\nexcept:\n    pass\n\ndef broken(\n"
            } else if matches!(
                scenario,
                EngineScenario::InvalidSyntax
                    | EngineScenario::InvalidSyntaxProjectionDisabled
                    | EngineScenario::CustomFrontendProjection
                    | EngineScenario::FrontendRuleCollision
            ) {
                "global x\nnonlocal x\n"
            } else if matches!(scenario, EngineScenario::FutureFeatureProjection) {
                "from __future__ import no_such_feature\n"
            } else if matches!(scenario, EngineScenario::UnknownFutureWithoutConsumer) {
                "from __future__ import no_such_feature\n"
            } else if matches!(scenario, EngineScenario::ProjectionCacheQueryChange) {
                "nonlocal x\n"
            } else {
                "try:\n    pass\nexcept:\n    pass\n"
            },
        )?;
    }
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules,
        query_modules: std::collections::BTreeMap::default(),
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    };
    if matches!(scenario, EngineScenario::ProjectionCacheQueryChange) {
        let rule = ryo::linter::RuleId::try_from("cache-probe").expect("cache probe identity");
        let first = factory::local_with_frontend_diagnostic(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(ryo_python::settings::PythonTargetVersion::Py310),
            )]),
            None,
            factory::FrontendDiagnosticProjection {
                rule: rule.clone(),
                query: ryo::query::RyoQlQuery::try_from("let diagnostics = select id, file, span from \"syntax-error\" where code = \"nonlocal-at-module\";\nemit diagnostics subject file key(id);\n")
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            },
        )
        .run(WorkerRequest {
            state: ryo_benchmark::domain::suite::types::WorkerState::Retained(
                root.path().join("retained-parse-cache"),
            ),
            ..request.clone()
        })?;
        let second = factory::local_with_frontend_diagnostic(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(ryo_python::settings::PythonTargetVersion::Py310),
            )]),
            None,
            factory::FrontendDiagnosticProjection {
                rule,
                query: ryo::query::RyoQlQuery::try_from("let diagnostics = select id, file, span from \"syntax-error\" where code = \"no-such-parser-code\";\nemit diagnostics subject file key(id);\n")
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            },
        )
        .run(WorkerRequest {
            state: ryo_benchmark::domain::suite::types::WorkerState::Retained(
                root.path().join("retained-parse-cache"),
            ),
            ..request
        })?;
        return Ok(first.diagnostics.len() == 1
            && first.diagnostics[0].rule.as_ref() == "cache-probe"
            && second.diagnostics.is_empty());
    }
    let result = if matches!(scenario, EngineScenario::InvalidSyntaxProjectionDisabled) {
        factory::local(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(
                    ryo_python::settings::PythonTargetVersion::Py310,
                ),
            )]),
            None,
        )
        .run(request)
    } else {
        let projection = if matches!(scenario, EngineScenario::CustomFrontendProjection) {
            factory::FrontendDiagnosticProjection {
                rule: ryo::linter::RuleId::try_from("parser-check")
                    .expect("custom diagnostic identity"),
                query: ryo::query::RyoQlQuery::try_from("let diagnostics = select id, file, span from \"syntax-error\" where message = \"nonlocal declaration not allowed at module level\";\nemit diagnostics subject file key(id);\n")
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            }
        } else {
            factory::FrontendDiagnosticProjection {
                rule: ryo::linter::RuleId::try_from("invalid-syntax").expect("frontend identity"),
                query: ryo::query::RyoQlQuery::try_from(
                    crate::constants::FRONTEND_DIAGNOSTIC_QUERY,
                )
                .map_err(|error| Error::Invalid(error.to_string()))?,
            }
        };
        factory::local_with_frontend_diagnostic(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(
                    ryo_python::settings::PythonTargetVersion::Py310,
                ),
            )]),
            None,
            projection,
        )
        .run(request)
    };
    if matches!(scenario, EngineScenario::Unread) {
        assert!(
            matches!(&result, Err(Error::Invalid(message)) if message.starts_with("incomplete candidate:") && message.contains("input.unknown")),
            "unread input must be reported as incomplete candidate coverage"
        );
    }
    if matches!(scenario, EngineScenario::Partial) {
        assert!(
            matches!(&result, Err(Error::Invalid(message))
                if message.starts_with("incomplete candidate:")
                    && !message.contains("unknowns=[]")
                    && message.contains("unread=[]")),
            "partial relation availability must refuse despite complete file coverage"
        );
    }
    let report = result?;
    let expected_diagnostics = if matches!(scenario, EngineScenario::InvalidSyntax) {
        vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("invalid-syntax").expect("syntax rule"),
            file: ryo::linter::RelativePath("input.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(9),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(19),
        }]
    } else if matches!(scenario, EngineScenario::InvalidSyntaxProjectionDisabled) {
        vec![]
    } else if matches!(scenario, EngineScenario::CustomFrontendProjection) {
        vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("parser-check").expect("custom rule"),
            file: ryo::linter::RelativePath("input.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(9),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(19),
        }]
    } else if matches!(scenario, EngineScenario::FutureFeatureProjection) {
        vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("F407").expect("future feature rule"),
            file: ryo::linter::RelativePath("input.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(23),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(38),
        }]
    } else if matches!(scenario, EngineScenario::UnknownFutureWithoutConsumer) {
        vec![]
    } else {
        vec![BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from("E722").expect("fixture rule"),
            file: ryo::linter::RelativePath("input.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(14),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(20),
        }]
    };
    Ok(
        report.files == [ryo::linter::RelativePath("input.py".into())]
            && report.diagnostics == expected_diagnostics,
    )
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
    let plain = factory::local(languages.clone(), None).run(WorkerRequest {
        root: root.into(),
        rules: rules.clone(),
        query_modules: std::collections::BTreeMap::default(),
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
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
    let mut modules = std::collections::BTreeMap::new();
    let _previous = modules.insert(
        "test/identity".to_owned(),
        std::path::PathBuf::from("fragments/identity.ryoql"),
    );
    let imported = factory::local(languages, None).run(WorkerRequest {
        root: root.into(),
        rules,
        query_modules: modules,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    })?;
    Ok(plain == imported)
}

#[test]
fn shared_suppression_fragment_matches_the_historical_inline_queries() -> Result<(), Error> {
    // These inline baselines are exact query bytes from pre-fragment head b9223b8613:
    // E722 cdcca980d4a6d8e771e40b46306e5afcb9b12ad3bbb69af38d3114c8b1737a9a;
    // F632 08832f8d91a994220563a2f480cb52c81936358516cb794f65c865b619bcda51.
    let temporary = tempfile::tempdir()?;
    let inline_profile = temporary.path().join("inline");
    let imported_profile = temporary.path().join("imported");
    let inline_rules = inline_profile.join("rules");
    let imported_rules = imported_profile.join("rules");
    for (rule, baseline, imported, cases) in [
        (
            "E722",
            include_str!("../../../tests/fixtures/ryoql-fragments/E722/query.ryoql"),
            include_str!("rules/E722/query.ryoql"),
            include_str!("rules/E722/cases.toml"),
        ),
        (
            "F632",
            include_str!("../../../tests/fixtures/ryoql-fragments/F632/query.ryoql"),
            include_str!("rules/F632/candidate.ryoql"),
            include_str!("rules/F632/cases.toml"),
        ),
    ] {
        for (root, query) in [(&inline_rules, baseline), (&imported_rules, imported)] {
            let rule_root = root.join(rule);
            std::fs::create_dir_all(&rule_root)?;
            std::fs::write(rule_root.join("query.ryoql"), query)?;
            std::fs::write(rule_root.join("cases.toml"), cases)?;
        }
    }
    let fragment_root = imported_profile.join("fragments");
    std::fs::create_dir_all(&fragment_root)?;
    std::fs::write(
        fragment_root.join("suppress.ryoql"),
        include_str!("fragments/suppress.ryoql"),
    )?;

    let suite = ryo_benchmark::outbound::suite::factory::local();
    let inline_cases = suite.cases(
        ryo_benchmark::outbound::suite::types::ArtifactPath(&inline_rules),
        ryo_benchmark::outbound::suite::types::ArtifactPath(&temporary.path().join("inline-cases")),
    )?;
    let imported_cases = suite.cases(
        ryo_benchmark::outbound::suite::types::ArtifactPath(&imported_rules),
        ryo_benchmark::outbound::suite::types::ArtifactPath(
            &temporary.path().join("imported-cases"),
        ),
    )?;
    let imported_by_name: std::collections::BTreeMap<_, _> = imported_cases
        .into_iter()
        .map(|case| ((case.rule.to_string(), case.name.0.to_string()), case))
        .collect();
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )]);
    let inline_engine = factory::local(languages.clone(), None);
    let imported_engine = factory::local(languages, None);
    let registry = std::collections::BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        std::path::PathBuf::from("fragments/suppress.ryoql"),
    )]);
    for case in inline_cases {
        let key = (case.rule.to_string(), case.name.0.to_string());
        let imported = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported counterpart for {key:?}")))?;
        let module = imported.request.root.join("fragments/suppress.ryoql");
        let module_parent = module
            .parent()
            .ok_or_else(|| Error::Invalid("staged fragment has no parent directory".to_owned()))?;
        std::fs::create_dir_all(module_parent)?;
        std::fs::write(&module, include_str!("fragments/suppress.ryoql"))?;
        let state = ryo_benchmark::domain::suite::types::WorkerState::Fresh;
        let inline = inline_engine.run(WorkerRequest {
            root: case.request.root,
            rules: inline_rules.clone(),
            query_modules: std::collections::BTreeMap::default(),
            files: case.request.files,
            state,
        })?;
        let fragmented = imported_engine.run(WorkerRequest {
            root: imported.request.root.clone(),
            rules: imported_rules.clone(),
            query_modules: registry.clone(),
            files: imported.request.files.clone(),
            state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
        })?;
        assert_eq!(
            inline, fragmented,
            "inline and fragment reports differ for {}/{}",
            key.0, key.1
        );
    }
    for (rule, baseline, imported, cases) in [
        (
            "E722",
            include_str!("../../../tests/fixtures/ryoql-fragments/E722/query.ryoql"),
            include_str!("rules/E722/query.ryoql"),
            include_str!("rules/E722/cases.toml"),
        ),
        (
            "F632",
            include_str!("../../../tests/fixtures/ryoql-fragments/F632/query.ryoql"),
            include_str!("rules/F632/candidate.ryoql"),
            include_str!("rules/F632/cases.toml"),
        ),
    ] {
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
    let inline_cases = suite.cases(
        ryo_benchmark::outbound::suite::types::ArtifactPath(inline_parent),
        ryo_benchmark::outbound::suite::types::ArtifactPath(
            &root.join(format!("single-inline-cases-{rule}")),
        ),
    )?;
    let imported_cases = suite.cases(
        ryo_benchmark::outbound::suite::types::ArtifactPath(imported_parent),
        ryo_benchmark::outbound::suite::types::ArtifactPath(
            &root.join(format!("single-imported-cases-{rule}")),
        ),
    )?;
    let imported_by_name: std::collections::BTreeMap<_, _> = imported_cases
        .into_iter()
        .map(|case| ((case.name.0.to_string(), case.rule.to_string()), case))
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
    let module = profile_root.join("fragments/suppress.ryoql");
    let module_parent = module
        .parent()
        .ok_or_else(|| Error::Invalid("fragment path has no parent directory".to_owned()))?;
    std::fs::create_dir_all(module_parent)?;
    std::fs::write(&module, include_str!("fragments/suppress.ryoql"))?;
    for case in inline_cases {
        let key = (case.name.0.to_string(), case.rule.to_string());
        let imported_case = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported case for {key:?}")))?;
        let inline = inline_engine.run(WorkerRequest {
            root: case.request.root,
            rules: inline_parent.to_path_buf(),
            query_modules: std::collections::BTreeMap::default(),
            files: case.request.files,
            state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
        })?;
        let fragmented = imported_engine.run(WorkerRequest {
            root: imported_case.request.root.clone(),
            rules: imported_parent.to_path_buf(),
            query_modules: std::collections::BTreeMap::from([(
                "python/ruff/suppress".to_owned(),
                std::path::PathBuf::from("fragments/suppress.ryoql"),
            )]),
            files: imported_case.request.files.clone(),
            state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
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
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
