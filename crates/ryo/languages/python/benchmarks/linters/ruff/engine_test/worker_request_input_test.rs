use super::constants::{
    ACCEPTED_SYNTAX, ALWAYS, DIAGNOSTIC_RYO, DIAGNOSTIC_UNKNOWN, EXE004, EXPLICIT_LEGACY_TARGET,
    EXPLICIT_NATIVE_ANNOTATION_TARGET, PARSER_CONFIGURATION, PROFILE_RYO, PYTHON_TARGET_3_6,
    PYTHON_TARGET_3_7, PYTHON_TARGET_3_8, PYTHON_TARGET_3_09, PYTHON_TARGET_3_9,
    PYTHON_TARGET_3_10, PYTHON_TARGET_3_10_2, PYTHON_TARGET_3_11, PYTHON_TARGET_3_12,
    PYTHON_TARGET_3_13, PYTHON_TARGET_3_14, PYTHON_TARGET_3_15, PYTHON_TARGET_3_16,
    PYTHON_TARGET_LATEST, REQUEST_FILE, REQUEST_JSON, REQUEST_MUST_BE_AN_OBJECT,
    ROOT_TMP_PROJECT_FILES_A_PY, RULE_APPLICABILITY, RUN_RYO, UNREVIEWED, UP045,
};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
#[path = "../composition/diagnostic_protocol.rs"]
mod diagnostic_protocol;
#[path = "../composition/request_input.rs"]
mod request_input;
#[path = "../composition/targets.rs"]
mod targets;

fn reads_the_exact_explicit_request_file() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let request_path = directory.path().join(REQUEST_JSON);
    let request = ROOT_TMP_PROJECT_FILES_A_PY;
    std::fs::write(&request_path, request)?;
    let mut arguments = [
        request_input::BenchmarkArgument(REQUEST_FILE.into()),
        request_input::BenchmarkArgument(request_path.to_string_lossy().into_owned()),
    ]
    .into_iter();
    assert_eq!(request_input::worker_input(&mut arguments)?.0, request);
    assert!(arguments.next().is_none());
    Ok(())
}

fn retains_the_explicit_inline_json_debug_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let request = ROOT_TMP_PROJECT_FILES_A_PY;
    let mut arguments = [request_input::BenchmarkArgument(request.into())].into_iter();
    assert_eq!(request_input::worker_input(&mut arguments)?.0, request);
    assert!(arguments.next().is_none());
    Ok(())
}

fn worker_execution_requires_the_actual_target_and_canonical_phase()
-> Result<(), Box<dyn std::error::Error>> {
    let request = serde_json::json!({
        (PARSER_CONFIGURATION): PYTHON_TARGET_3_10,
        (RULE_APPLICABILITY): {(UP045): ACCEPTED_SYNTAX, (EXE004): ALWAYS}
    });
    let configuration = Some(PYTHON_TARGET_3_10);
    request_input::checked_worker_identity(
        &request_input::WorkerInput(request.to_string()),
        configuration.map(targets::TargetConfiguration),
    )?;
    for missing_or_wrong in [None, Some(PYTHON_TARGET_3_11), Some(PYTHON_TARGET_3_9)] {
        assert!(matches!(
            request_input::checked_worker_identity(
                &request_input::WorkerInput(request.to_string()),
                missing_or_wrong.map(targets::TargetConfiguration)
            ),
            Err(Error::Invalid(_))
        ));
    }
    let mut missing = request.clone();
    assert!(
        missing
            .as_object_mut()
            .ok_or(REQUEST_MUST_BE_AN_OBJECT)?
            .remove(PARSER_CONFIGURATION)
            .is_some()
    );
    assert!(matches!(
        request_input::checked_worker_identity(
            &request_input::WorkerInput(missing.to_string()),
            configuration.map(targets::TargetConfiguration)
        ),
        Err(Error::Invalid(_))
    ));
    for phases in [
        serde_json::Value::Null,
        serde_json::json!({}),
        serde_json::json!({(UP045): ALWAYS}),
        serde_json::json!({(EXE004): ACCEPTED_SYNTAX}),
        serde_json::json!({(UP045): UNREVIEWED}),
    ] {
        let mut changed = request.clone();
        changed[RULE_APPLICABILITY] = phases;
        assert!(matches!(
            request_input::checked_worker_identity(
                &request_input::WorkerInput(changed.to_string()),
                configuration.map(targets::TargetConfiguration)
            ),
            Err(Error::Invalid(_))
        ));
    }
    let mut absent = request;
    assert!(
        absent
            .as_object_mut()
            .ok_or(REQUEST_MUST_BE_AN_OBJECT)?
            .remove(RULE_APPLICABILITY)
            .is_some()
    );
    assert!(matches!(
        request_input::checked_worker_identity(
            &request_input::WorkerInput(absent.to_string()),
            configuration.map(targets::TargetConfiguration)
        ),
        Err(Error::Invalid(_))
    ));
    Ok(())
}

fn every_explicit_supported_target_preserves_request_and_phase_identity()
-> Result<(), Box<dyn std::error::Error>> {
    for configuration in [
        PYTHON_TARGET_3_7,
        PYTHON_TARGET_3_8,
        PYTHON_TARGET_3_9,
        PYTHON_TARGET_3_10,
        PYTHON_TARGET_3_11,
        PYTHON_TARGET_3_12,
        PYTHON_TARGET_3_13,
        PYTHON_TARGET_3_14,
        PYTHON_TARGET_3_15,
    ] {
        let request = serde_json::json!({
            (PARSER_CONFIGURATION): configuration,
            (RULE_APPLICABILITY): {(UP045): ACCEPTED_SYNTAX, (EXE004): ALWAYS}
        });
        request_input::checked_worker_identity(
            &request_input::WorkerInput(request.to_string()),
            Some(targets::TargetConfiguration(configuration)),
        )?;
        let wrong = if configuration == PYTHON_TARGET_3_10 {
            PYTHON_TARGET_3_14
        } else {
            PYTHON_TARGET_3_10
        };
        assert!(matches!(
            request_input::checked_worker_identity(
                &request_input::WorkerInput(request.to_string()),
                Some(targets::TargetConfiguration(wrong))
            ),
            Err(Error::Invalid(_))
        ));
        let mut missing_phase = request;
        missing_phase[RULE_APPLICABILITY] = serde_json::Value::Null;
        assert!(matches!(
            request_input::checked_worker_identity(
                &request_input::WorkerInput(missing_phase.to_string()),
                Some(targets::TargetConfiguration(configuration))
            ),
            Err(Error::Invalid(_))
        ));
    }
    Ok(())
}

fn worker_modes_are_explicit_and_keep_diagnostics_out_of_timed_run() {
    use diagnostic_protocol::{WorkerTracing, tracing_for_command};

    assert_eq!(
        tracing_for_command(diagnostic_protocol::WorkerCommand(RUN_RYO)),
        Some(WorkerTracing::Disabled)
    );
    assert_eq!(
        tracing_for_command(diagnostic_protocol::WorkerCommand(PROFILE_RYO)),
        Some(WorkerTracing::Chrome)
    );
    assert_eq!(
        tracing_for_command(diagnostic_protocol::WorkerCommand(DIAGNOSTIC_RYO)),
        Some(WorkerTracing::JsonLines)
    );
    assert_eq!(
        tracing_for_command(diagnostic_protocol::WorkerCommand(DIAGNOSTIC_UNKNOWN)),
        None
    );
}

fn legacy_and_native_annotation_boundaries_select_the_requested_parser() {
    use ryo_python::settings::PythonTargetVersion;
    use targets::{TargetConfiguration, resolve};

    let legacy =
        resolve(Some(TargetConfiguration(PYTHON_TARGET_3_7))).expect(EXPLICIT_LEGACY_TARGET);
    assert_eq!(legacy.parser, PythonTargetVersion::Py37);
    assert_eq!(legacy.reference.0, "py37");
    let native = resolve(Some(TargetConfiguration(PYTHON_TARGET_3_14)))
        .expect(EXPLICIT_NATIVE_ANNOTATION_TARGET);
    assert_eq!(native.parser, PythonTargetVersion::Py314);
    assert_eq!(native.reference.0, "py314");
}

fn missing_moving_unknown_and_noncanonical_targets_are_refused() {
    use targets::{TargetConfiguration, resolve};

    for value in [
        None,
        Some(PYTHON_TARGET_LATEST),
        Some(PYTHON_TARGET_3_6),
        Some(PYTHON_TARGET_3_16),
        Some(PYTHON_TARGET_3_09),
        Some(PYTHON_TARGET_3_10_2),
    ] {
        assert!(
            matches!(
                resolve(value.map(TargetConfiguration)),
                Err(Error::Invalid(_))
            ),
            "{value:?}"
        );
    }
}

#[derive(Clone, Copy)]
enum WorkerRequestScenario {
    WorkerModesAreExplicitAndKeepDiagnosticsOutOfTimedRun,
    ReadsTheExactExplicitRequestFile,
    RetainsTheExplicitInlineJsonDebugProtocol,
    WorkerExecutionRequiresTheActualTargetAndCanonicalPhase,
    EveryExplicitSupportedTargetPreservesRequestAndPhaseIdentity,
    LegacyAndNativeAnnotationBoundariesSelectTheRequestedParser,
    MissingMovingUnknownAndNoncanonicalTargetsAreRefused,
}
type Input = WorkerRequestScenario;
type Value = bool;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("worker_modes_are_explicit_and_keep_diagnostics_out_of_timed_run"),
        word: Word::Succeeds,
        input: || WorkerRequestScenario::WorkerModesAreExplicitAndKeepDiagnosticsOutOfTimedRun,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("reads_the_exact_explicit_request_file"),
        word: Word::Succeeds,
        input: || WorkerRequestScenario::ReadsTheExactExplicitRequestFile,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("retains_the_explicit_inline_json_debug_protocol"),
        word: Word::Succeeds,
        input: || WorkerRequestScenario::RetainsTheExplicitInlineJsonDebugProtocol,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("worker_execution_requires_the_actual_target_and_canonical_phase"),
        word: Word::Succeeds,
        input: || WorkerRequestScenario::WorkerExecutionRequiresTheActualTargetAndCanonicalPhase,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_explicit_supported_target_preserves_request_and_phase_identity"),
        word: Word::Succeeds,
        input: || {
            WorkerRequestScenario::EveryExplicitSupportedTargetPreservesRequestAndPhaseIdentity
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("legacy_and_native_annotation_boundaries_select_the_requested_parser"),
        word: Word::Succeeds,
        input: || {
            WorkerRequestScenario::LegacyAndNativeAnnotationBoundariesSelectTheRequestedParser
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("missing_moving_unknown_and_noncanonical_targets_are_refused"),
        word: Word::Succeeds,
        input: || WorkerRequestScenario::MissingMovingUnknownAndNoncanonicalTargetsAreRefused,
        expect: Expect::Value(true),
    },
];

fn subject(input: Input) -> Result<bool, Failure> {
    match input {
        WorkerRequestScenario::WorkerModesAreExplicitAndKeepDiagnosticsOutOfTimedRun => {
            worker_modes_are_explicit_and_keep_diagnostics_out_of_timed_run();
            Ok(true)
        }
        WorkerRequestScenario::ReadsTheExactExplicitRequestFile => {
            reads_the_exact_explicit_request_file()
                .map(|()| true)
                .map_err(|error| error.to_string())
        }
        WorkerRequestScenario::RetainsTheExplicitInlineJsonDebugProtocol => {
            retains_the_explicit_inline_json_debug_protocol()
                .map(|()| true)
                .map_err(|error| error.to_string())
        }
        WorkerRequestScenario::WorkerExecutionRequiresTheActualTargetAndCanonicalPhase => {
            worker_execution_requires_the_actual_target_and_canonical_phase()
                .map(|()| true)
                .map_err(|error| error.to_string())
        }
        WorkerRequestScenario::EveryExplicitSupportedTargetPreservesRequestAndPhaseIdentity => {
            every_explicit_supported_target_preserves_request_and_phase_identity()
                .map(|()| true)
                .map_err(|error| error.to_string())
        }
        WorkerRequestScenario::LegacyAndNativeAnnotationBoundariesSelectTheRequestedParser => {
            legacy_and_native_annotation_boundaries_select_the_requested_parser();
            Ok(true)
        }
        WorkerRequestScenario::MissingMovingUnknownAndNoncanonicalTargetsAreRefused => {
            missing_moving_unknown_and_noncanonical_targets_are_refused();
            Ok(true)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
