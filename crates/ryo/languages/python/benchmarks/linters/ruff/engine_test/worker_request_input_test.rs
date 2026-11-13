use super::constants::{
    EXPLICIT_LEGACY_TARGET, EXPLICIT_NATIVE_ANNOTATION_TARGET, PYTHON_TARGET_3_6,
    PYTHON_TARGET_3_7, PYTHON_TARGET_3_8, PYTHON_TARGET_3_09, PYTHON_TARGET_3_9,
    PYTHON_TARGET_3_10, PYTHON_TARGET_3_10_2, PYTHON_TARGET_3_11, PYTHON_TARGET_3_12,
    PYTHON_TARGET_3_13, PYTHON_TARGET_3_14, PYTHON_TARGET_3_15, PYTHON_TARGET_3_16,
    PYTHON_TARGET_LATEST,
};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
#[path = "../composition/request_input.rs"]
mod request_input;
#[path = "../composition/targets.rs"]
mod targets;

fn worker_execution_requires_the_actual_target_and_canonical_phase()
-> Result<(), Box<dyn std::error::Error>> {
    for missing_or_unsupported in [None, Some(PYTHON_TARGET_3_6), Some(PYTHON_TARGET_LATEST)] {
        assert!(matches!(
            request_input::worker_expectation(
                missing_or_unsupported.map(targets::TargetConfiguration)
            ),
            Err(Error::Invalid(_))
        ));
    }
    let expectation =
        request_input::worker_expectation(Some(targets::TargetConfiguration(PYTHON_TARGET_3_10)))?;
    assert_eq!(expectation.parser_configuration.0, PYTHON_TARGET_3_10);
    assert!(!expectation.pinned_applicability.is_empty());
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
        let expectation =
            request_input::worker_expectation(Some(targets::TargetConfiguration(configuration)))?;
        assert_eq!(expectation.parser_configuration.0, configuration);
        assert!(!expectation.pinned_applicability.is_empty());
    }
    Ok(())
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
