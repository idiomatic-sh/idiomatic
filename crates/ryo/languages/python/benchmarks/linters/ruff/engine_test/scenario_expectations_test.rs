//! The input file, source and expected findings of each engine scenario.
use super::scenario_test::EngineScenario;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;

pub(super) fn input_file(scenario: EngineScenario) -> &'static str {
    match scenario {
        EngineScenario::LexicallyCompleteSyntaxError
        | EngineScenario::PreservedSemanticSyntaxError
        | EngineScenario::OrdinaryAnnotationNamedExpression
        | EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::CustomFrontendProjection
        | EngineScenario::FutureFeatureProjection
        | EngineScenario::LateFutureDirectiveProjection
        | EngineScenario::UnknownFutureWithoutConsumer
        | EngineScenario::FrontendRuleCollision
        | EngineScenario::ARejectedFileLeavesPt020Pt025AndPt026AnsweringTheValidFile
        | EngineScenario::ARejectedFileLeavesI001Plw0406AndRuf100AnsweringTheValidFile
        | EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries
        | EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        | EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule
        | EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders
        | EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory => {
            "input.py"
        }
        EngineScenario::PreservedStubSemanticSyntaxError => "input.pyi",
    }
}

pub(super) fn source_for(scenario: EngineScenario) -> &'static str {
    match scenario {
        EngineScenario::InvalidSyntaxProjectionDisabled
        | EngineScenario::CustomFrontendProjection
        | EngineScenario::FrontendRuleCollision => "global x\nnonlocal x\n",
        EngineScenario::LexicallyCompleteSyntaxError => "value = ur\"hello\"\n",
        EngineScenario::PreservedSemanticSyntaxError
        | EngineScenario::PreservedStubSemanticSyntaxError => {
            "from __future__ import annotations\ndef f(x: (a := b)): ...\n"
        }
        EngineScenario::OrdinaryAnnotationNamedExpression => "def f(x: (a := b)): ...\n",
        EngineScenario::FutureFeatureProjection | EngineScenario::UnknownFutureWithoutConsumer => {
            "from __future__ import no_such_feature\n"
        }
        EngineScenario::LateFutureDirectiveProjection => {
            "x = 1\nfrom __future__ import annotations\n"
        }
        EngineScenario::ARejectedFileLeavesPt020Pt025AndPt026AnsweringTheValidFile
        | EngineScenario::ARejectedFileLeavesI001Plw0406AndRuf100AnsweringTheValidFile
        | EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries
        | EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        | EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule
        | EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders
        | EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory => {
            "try:\n    pass\nexcept:\n    pass\n"
        }
    }
}

pub(super) fn expected_diagnostics(
    scenario: EngineScenario,
) -> Result<Vec<BenchmarkFinding>, Error> {
    use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset as At;
    if matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
    ) {
        let (file, syntax_start, syntax_end, name_start, name_end) =
            if scenario == EngineScenario::PreservedSemanticSyntaxError {
                ("input.py", 45, 51, 50, 51)
            } else {
                ("input.pyi", 45, 51, 50, 51)
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
        EngineScenario::LateFutureDirectiveProjection => ("F404", 6, 40),
        EngineScenario::FrontendRuleCollision
        | EngineScenario::ARejectedFileLeavesPt020Pt025AndPt026AnsweringTheValidFile
        | EngineScenario::ARejectedFileLeavesI001Plw0406AndRuf100AnsweringTheValidFile
        | EngineScenario::SharedSuppressionFragmentMatchesTheHistoricalInlineQueries
        | EngineScenario::ActualI001MixedNamespaceObservationStaysUnknownOnColdAndWarmReplay
        | EngineScenario::LazyFutureStaysVisibleWithoutADedicatedSelectedRule
        | EngineScenario::ListedProjectInputsAndInvalidSourcesKeepDistinctReaders
        | EngineScenario::I001HiddenMatchAndRuf100OutputFollowFirstImportAndUnknownInventory => {
            ("E722", 14, 20)
        }
    };
    Ok(vec![BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from(rule)
            .map_err(|error| Error::Invalid(error.to_string()))?,
        file: ryo::linter::RelativePath("input.py".into()),
        start: At(start),
        end: At(end),
    }])
}
