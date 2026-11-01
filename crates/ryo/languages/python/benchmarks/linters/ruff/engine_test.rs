//! Real SDK evidence for missing files, unread or partial inputs and typed findings.
use super::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::BenchmarkFinding},
    suite::types::WorkerRequest,
};
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort as _};

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
            "let feature = select id, file, span from \"syntax-error\" where code = \"future-feature-not-defined\";\nemit feature subject file key(id);\n",
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
        | EngineScenario::ProjectionCacheQueryChange => "input.py",
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
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
    };
    if matches!(scenario, EngineScenario::ProjectionCacheQueryChange) {
        let rule = ryo::linter::RuleId::try_from("cache-probe").expect("cache probe identity");
        let first = factory::local_with_frontend_diagnostic(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::PythonParserSettings::new(ryo_python::PythonTargetVersion::Py310),
            )]),
            None,
            ryo_benchmark::outbound::engine::factory::FrontendDiagnosticProjection {
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
                ryo_python::PythonParserSettings::new(ryo_python::PythonTargetVersion::Py310),
            )]),
            None,
            ryo_benchmark::outbound::engine::factory::FrontendDiagnosticProjection {
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
                ryo_python::PythonParserSettings::new(ryo_python::PythonTargetVersion::Py310),
            )]),
            None,
        )
        .run(request)
    } else {
        let projection = if matches!(scenario, EngineScenario::CustomFrontendProjection) {
            ryo_benchmark::outbound::engine::factory::FrontendDiagnosticProjection {
                rule: ryo::linter::RuleId::try_from("parser-check")
                    .expect("custom diagnostic identity"),
                query: ryo::query::RyoQlQuery::try_from("let diagnostics = select id, file, span from \"syntax-error\" where message = \"nonlocal declaration not allowed at module level\";\nemit diagnostics subject file key(id);\n")
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            }
        } else {
            ryo_benchmark::outbound::engine::factory::FrontendDiagnosticProjection {
                rule: ryo::linter::RuleId::try_from("invalid-syntax").expect("frontend identity"),
                query: ryo::query::RyoQlQuery::try_from(
                    crate::constants::FRONTEND_DIAGNOSTIC_QUERY,
                )
                .map_err(|error| Error::Invalid(error.to_string()))?,
            }
        };
        factory::local_with_frontend_diagnostic(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::PythonParserSettings::new(ryo_python::PythonTargetVersion::Py310),
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
