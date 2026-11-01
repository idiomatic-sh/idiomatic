//! Real SDK evidence for missing files, unread or partial inputs and typed findings.
use super::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::BenchmarkFinding},
    suite::types::WorkerRequest,
};
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort as _};
use ryo_benchmark::outbound::suite::interface::SuitePort as _;
use std::collections::BTreeMap;

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
    UnknownFutureWithoutConsumer,
    FrontendRuleCollision,
    ProjectionCacheQueryChange,
    QueryModuleInlineEquivalence,
    TargetConfigurationCacheChange,
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
        name: CaseName("stub_semantic_parse_diagnostic_keeps_f821_and_frontend_rows"),
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
];
fn exercise(scenario: Input) -> Result<Value, Error> {
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
    write_rule_fixtures(&rules, scenario)?;
    let file = input_file(scenario);
    if scenario != EngineScenario::Missing {
        std::fs::write(root.path().join(file), source_for(scenario))?;
    }
    let request = WorkerRequest {
        root: root.path().to_owned(),
        rules,
        query_modules: suppress_modules(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    };
    let result = run_with_projection(scenario, request);
    check_expected_failure(scenario, &result);
    let report = result?;
    let expected = expected_diagnostics(scenario)?;
    Ok(report.files == [ryo::linter::RelativePath(file.into())] && report.diagnostics == expected)
}

#[test]
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
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::from([
            ("ASYNC100".to_owned(), "always".to_owned()),
            ("UP025".to_owned(), "accepted-syntax".to_owned()),
        ]),
        parser_configuration: Some("python-target=3.10".to_owned()),
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    };
    let result = factory::local_with_frontend_diagnostic(
        python_310(),
        None,
        frontend_projection(EngineScenario::LexicallyCompleteSyntaxError)?,
    )
    .run(request);
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
        Ok(_) => {
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
        | EngineScenario::UnknownFutureWithoutConsumer
        | EngineScenario::FrontendRuleCollision
        | EngineScenario::ProjectionCacheQueryChange
        | EngineScenario::QueryModuleInlineEquivalence
        | EngineScenario::TargetConfigurationCacheChange => "input.py",
        EngineScenario::PreservedStubSemanticSyntaxError => "input.pyi",
    }
}

fn source_for(scenario: EngineScenario) -> &'static str {
    match scenario {
        EngineScenario::Partial => "try:\n    pass\nexcept:\n    pass\n\ndef broken(\n",
        EngineScenario::InvalidSyntax
        | EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::CustomFrontendProjection
        | EngineScenario::FrontendRuleCollision => "global x\nnonlocal x\n",
        EngineScenario::LexicallyCompleteSyntaxError => "value = ur\"hello\"\n",
        EngineScenario::PreservedSemanticSyntaxError => {
            "from __future__ import annotations\ndef f(x: (a := b)): ...\n"
        }
        EngineScenario::PreservedStubSemanticSyntaxError
        | EngineScenario::OrdinaryAnnotationNamedExpression => "def f(x: (a := b)): ...\n",
        EngineScenario::FutureFeatureProjection | EngineScenario::UnknownFutureWithoutConsumer => {
            "from __future__ import no_such_feature\n"
        }
        EngineScenario::ProjectionCacheQueryChange => "nonlocal x\n",
        EngineScenario::Finding
        | EngineScenario::Missing
        | EngineScenario::Unread
        | EngineScenario::QueryModuleInlineEquivalence
        | EngineScenario::TargetConfigurationCacheChange => "try:\n    pass\nexcept:\n    pass\n",
    }
}

fn write_rule_fixtures(rules: &std::path::Path, scenario: EngineScenario) -> Result<(), Error> {
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
        let f821 = rules.join("F821");
        std::fs::create_dir_all(&f821)?;
        std::fs::write(
            f821.join("query.ryoql"),
            include_str!("rules/F821/candidate.ryoql"),
        )?;
    }
    if scenario == EngineScenario::FutureFeatureProjection {
        let f407 = rules.join("F407");
        std::fs::create_dir_all(&f407)?;
        std::fs::write(
            f407.join("query.ryoql"),
            include_str!("rules/F407/candidate.ryoql"),
        )?;
    }
    if scenario == EngineScenario::FrontendRuleCollision {
        let invalid_syntax = rules.join("invalid-syntax");
        std::fs::create_dir_all(&invalid_syntax)?;
        std::fs::write(
            invalid_syntax.join("query.ryoql"),
            include_str!("rules/E722/query.ryoql"),
        )?;
    }
    Ok(())
}

fn suppress_modules() -> BTreeMap<String, std::path::PathBuf> {
    BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        std::path::PathBuf::from("fragments/suppress.ryoql"),
    )])
}

fn write_suppress_module(root: &std::path::Path) -> Result<(), Error> {
    let path = root.join("fragments/suppress.ryoql");
    let parent = path
        .parent()
        .ok_or_else(|| Error::Invalid("suppress module has no parent".into()))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, include_str!("fragments/suppress.ryoql"))?;
    Ok(())
}

fn python_310() -> ryo::linter::Languages {
    ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )])
}

fn run_with_projection(
    scenario: EngineScenario,
    request: WorkerRequest,
) -> Result<ryo_benchmark::domain::diagnostics::types::RunReport, Error> {
    if scenario == EngineScenario::InvalidSyntaxProjectionDisabled {
        return factory::local(python_310(), None).run(request);
    }
    let projection = frontend_projection(scenario)?;
    factory::local_with_frontend_diagnostic(python_310(), None, projection).run(request)
}

fn frontend_projection(
    scenario: EngineScenario,
) -> Result<factory::FrontendDiagnosticProjection, Error> {
    let (rule, query) = if scenario == EngineScenario::CustomFrontendProjection {
        (
            "parser-check",
            r#"let diagnostics = select id, file, span from "syntax-error" where message = "nonlocal declaration not allowed at module level";
emit diagnostics subject file key(id);
"#,
        )
    } else {
        (
            "invalid-syntax",
            crate::constants::FRONTEND_DIAGNOSTIC_QUERY,
        )
    };
    Ok(factory::FrontendDiagnosticProjection {
        rule: ryo::linter::RuleId::try_from(rule)
            .map_err(|error| Error::Invalid(error.to_string()))?,
        query: ryo::query::RyoQlQuery::try_from(query)
            .map_err(|error| Error::Invalid(error.to_string()))?,
    })
}

fn check_expected_failure(
    scenario: EngineScenario,
    result: &Result<ryo_benchmark::domain::diagnostics::types::RunReport, Error>,
) {
    if scenario == EngineScenario::Unread {
        assert!(
            matches!(result, Err(Error::Invalid(message)) if message.starts_with("incomplete candidate:") && message.contains("input.unknown")),
            "unread input must be reported as incomplete candidate coverage"
        );
    }
    if scenario == EngineScenario::Partial {
        assert!(
            matches!(result, Err(Error::Invalid(message))
                if message.starts_with("incomplete candidate:")
                    && !message.contains("unknowns=[]")
                    && message.contains("unread=[]")),
            "partial relation availability must refuse despite complete file coverage"
        );
    }
}

fn expected_diagnostics(scenario: EngineScenario) -> Result<Vec<BenchmarkFinding>, Error> {
    use ryo_benchmark::domain::diagnostics::types::SourceByteOffset as At;
    if matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
    ) {
        let (file, syntax_start, syntax_end, name_start, name_end) =
            if scenario == EngineScenario::PreservedSemanticSyntaxError {
                ("input.py", 45, 51, 50, 51)
            } else {
                ("input.pyi", 10, 16, 15, 16)
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
        EngineScenario::Finding
        | EngineScenario::Missing
        | EngineScenario::Unread
        | EngineScenario::Partial
        | EngineScenario::FrontendRuleCollision
        | EngineScenario::ProjectionCacheQueryChange
        | EngineScenario::QueryModuleInlineEquivalence => ("E722", 14, 20),
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
    write_rule_fixtures(&rules, EngineScenario::ProjectionCacheQueryChange)?;
    let file = input_file(EngineScenario::ProjectionCacheQueryChange);
    std::fs::write(
        root.join(file),
        source_for(EngineScenario::ProjectionCacheQueryChange),
    )?;
    let request = WorkerRequest {
        root: root.to_owned(),
        rules,
        query_modules: suppress_modules(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
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
    let first = run("nonlocal-at-module")?.run(WorkerRequest {
        state: ryo_benchmark::domain::suite::types::WorkerState::Retained(
            root.join("retained-parse-cache"),
        ),
        ..request.clone()
    })?;
    let second = run("no-such-parser-code")?.run(WorkerRequest {
        state: ryo_benchmark::domain::suite::types::WorkerState::Retained(
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
    let plain = factory::local(languages.clone(), None).run(WorkerRequest {
        root: root.into(),
        rules: rules.clone(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
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
    let mut modules = BTreeMap::new();
    let _previous = modules.insert(
        "test/identity".to_owned(),
        std::path::PathBuf::from("fragments/identity.ryoql"),
    );
    let imported = factory::local(languages, None).run(WorkerRequest {
        root: root.into(),
        rules,
        query_modules: modules,
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    })?;
    Ok(plain == imported)
}

fn stage_py310_negative_files(
    root: &std::path::Path,
) -> Result<Vec<ryo::linter::RelativePath>, Error> {
    #[derive(serde::Deserialize)]
    struct Cases {
        cases: Vec<Case>,
    }

    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        files: BTreeMap<String, String>,
        expected: Vec<serde::de::IgnoredAny>,
    }

    let cases: Cases = toml::from_str(include_str!("rules/UP017/cases.toml"))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let mut negative_files = Vec::new();
    for case in cases.cases {
        if !case.expected.is_empty() {
            return Err(Error::Invalid(format!(
                "UP017 PY310 guard case {} is not an expected negative",
                case.name
            )));
        }
        for (relative, text) in case.files {
            let path = format!("up017-py310/{}/{}", case.name, relative);
            let destination = root.join(&path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(destination, text)?;
            negative_files.push(ryo::linter::RelativePath(path.into()));
        }
    }
    negative_files.sort();
    Ok(negative_files)
}

fn target_configuration_cache_change(root: &std::path::Path) -> Result<bool, Error> {
    use ryo_benchmark::domain::diagnostics::types::SourceByteOffset;
    use ryo_benchmark::domain::suite::types::WorkerState;
    let rules = root.join("rules");
    let rule_directory = rules.join("UP017");
    std::fs::create_dir_all(&rule_directory)?;
    std::fs::write(
        rule_directory.join("query.ryoql"),
        include_str!("rules/UP017/candidate.ryoql"),
    )?;
    let module_path = root.join("fragments/suppress.ryoql");
    std::fs::create_dir_all(
        module_path
            .parent()
            .ok_or_else(|| Error::Invalid("suppress module has no parent".into()))?,
    )?;
    std::fs::write(&module_path, include_str!("fragments/suppress.ryoql"))?;
    std::fs::write(
        root.join("input.py"),
        "import datetime\ndatetime.timezone.utc\n",
    )?;
    let files = vec![ryo::linter::RelativePath("input.py".into())];
    let cache = root.join("retained-target-cache");
    let query_modules = BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        std::path::PathBuf::from("fragments/suppress.ryoql"),
    )]);
    let negative_files = stage_py310_negative_files(root)?;
    let negative = factory::local(
        ryo::linter::Languages(vec![ryo_python::linked_with_settings(
            ryo_python::settings::PythonParserSettings::new(
                ryo_python::settings::PythonTargetVersion::Py310,
            ),
        )]),
        None,
    )
    .run(WorkerRequest {
        root: root.to_owned(),
        rules: rules.clone(),
        query_modules: query_modules.clone(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: Some("python-target=3.10".to_owned()),
        files: negative_files.clone(),
        state: WorkerState::Fresh,
    })?;
    if negative.files != negative_files || !negative.diagnostics.is_empty() {
        let message = format!(
            "UP017 PY310 target guard differs: requested={negative_files:?}, observed={:?}, diagnostics={:?}",
            negative.files, negative.diagnostics
        );
        return Err(Error::Invalid(message));
    }
    let run = |target, identity: &'static str| {
        let engine = factory::local(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(target),
            )]),
            None,
        );
        engine.run(WorkerRequest {
            root: root.to_owned(),
            rules: rules.clone(),
            query_modules: query_modules.clone(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: Some(identity.to_owned()),
            files: files.clone(),
            state: WorkerState::Retained(cache.clone()),
        })
    };
    let py310 = run(
        ryo_python::settings::PythonTargetVersion::Py310,
        "python-target=3.10",
    )?;
    let py311 = run(
        ryo_python::settings::PythonTargetVersion::Py311,
        "python-target=3.11",
    )?;
    let expected = vec![BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("UP017")
            .map_err(|error| Error::Invalid(error.to_string()))?,
        file: files
            .first()
            .cloned()
            .ok_or_else(|| Error::Invalid("missing target fixture file".into()))?,
        start: SourceByteOffset(16),
        end: SourceByteOffset(37),
    }];
    if !py310.diagnostics.is_empty() || py311.diagnostics != expected {
        let message = format!(
            "UP017 target cache transition differs: py310={:?}, py311={:?}",
            py310.diagnostics, py311.diagnostics
        );
        return Err(Error::Invalid(message));
    }
    Ok(true)
}

fn case_identities(
    cases: Vec<ryo_benchmark::domain::suite::types::PreparedCase>,
) -> BTreeMap<(String, String), ryo_benchmark::domain::suite::types::PreparedCase> {
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

#[test]
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
            include_str!("rules/F632/candidate.ryoql"),
            include_str!("rules/F632/cases.toml"),
        ),
    ];
    for (rule, baseline, imported, cases) in fixtures {
        write_rule_profile(&inline_rules, rule, baseline, cases)?;
        write_rule_profile(&imported_rules, rule, imported, cases)?;
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
    let imported_by_name = case_identities(imported_cases);
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )]);
    let inline_engine = factory::local(languages.clone(), None);
    let imported_engine = factory::local(languages, None);
    let registry = BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        std::path::PathBuf::from("fragments/suppress.ryoql"),
    )]);
    for case in inline_cases {
        let key = (case.rule.as_ref().to_owned(), case.name.0.clone());
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
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            files: case.request.files,
            state,
        })?;
        let fragmented = imported_engine.run(WorkerRequest {
            root: imported.request.root.clone(),
            rules: imported_rules.clone(),
            query_modules: registry.clone(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            files: imported.request.files.clone(),
            state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
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
    let module = profile_root.join("fragments/suppress.ryoql");
    let module_parent = module
        .parent()
        .ok_or_else(|| Error::Invalid("fragment path has no parent directory".to_owned()))?;
    std::fs::create_dir_all(module_parent)?;
    std::fs::write(&module, include_str!("fragments/suppress.ryoql"))?;
    for case in inline_cases {
        let key = (case.name.0.clone(), case.rule.as_ref().to_owned());
        let imported_case = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported case for {key:?}")))?;
        let inline = inline_engine.run(WorkerRequest {
            root: case.request.root,
            rules: inline_parent.to_path_buf(),
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
            files: case.request.files,
            state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
        })?;
        let fragmented = imported_engine.run(WorkerRequest {
            root: imported_case.request.root.clone(),
            rules: imported_parent.to_path_buf(),
            query_modules: BTreeMap::from([(
                "python/ruff/suppress".to_owned(),
                std::path::PathBuf::from("fragments/suppress.ryoql"),
            )]),
            rule_applicability: BTreeMap::default(),
            parser_configuration: None,
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
    let report = factory::local(python_310(), None).run(WorkerRequest {
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        files: ["true.py", "false.py"]
            .into_iter()
            .map(|path| ryo::linter::RelativePath(path.into()))
            .collect(),
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
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
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
