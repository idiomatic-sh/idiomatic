use crate::engine_test::constants::{
    ACCEPTED_SYNTAX, ALWAYS, EXE004, PARSER_CONFIGURATION, PYTHON_TARGET_3_7, PYTHON_TARGET_3_8,
    PYTHON_TARGET_3_9, PYTHON_TARGET_3_10, PYTHON_TARGET_3_11, PYTHON_TARGET_3_12,
    PYTHON_TARGET_3_13, PYTHON_TARGET_3_14, PYTHON_TARGET_3_15, REQUEST_FILE, REQUEST_JSON,
    REQUEST_MUST_BE_AN_OBJECT, ROOT_TMP_PROJECT_FILES_A_PY, RULE_APPLICABILITY, UNREVIEWED, UP045,
};
#[path = "composition/constants.rs"]
mod constants;
#[path = "composition/request_input.rs"]
mod request_input;
#[path = "composition/targets.rs"]
mod targets;

#[test]
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
    assert_eq!(request_input::worker_input(&mut arguments)?, request);
    assert!(arguments.next().is_none());
    Ok(())
}

#[test]
fn retains_the_explicit_inline_json_debug_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let request = ROOT_TMP_PROJECT_FILES_A_PY;
    let mut arguments = [request_input::BenchmarkArgument(request.into())].into_iter();
    assert_eq!(request_input::worker_input(&mut arguments)?, request);
    assert!(arguments.next().is_none());
    Ok(())
}

#[test]
fn worker_execution_requires_the_actual_target_and_canonical_phase()
-> Result<(), Box<dyn std::error::Error>> {
    let request = serde_json::json!({
        (PARSER_CONFIGURATION): PYTHON_TARGET_3_10,
        (RULE_APPLICABILITY): {(UP045): ACCEPTED_SYNTAX, (EXE004): ALWAYS}
    });
    let configuration = Some(PYTHON_TARGET_3_10);
    request_input::checked_worker_identity(&request.to_string(), configuration)?;
    for missing_or_wrong in [None, Some(PYTHON_TARGET_3_11), Some(PYTHON_TARGET_3_9)] {
        assert!(
            request_input::checked_worker_identity(&request.to_string(), missing_or_wrong).is_err()
        );
    }
    let mut missing = request.clone();
    assert!(
        missing
            .as_object_mut()
            .ok_or(REQUEST_MUST_BE_AN_OBJECT)?
            .remove(PARSER_CONFIGURATION)
            .is_some()
    );
    assert!(request_input::checked_worker_identity(&missing.to_string(), configuration).is_err());
    for phases in [
        serde_json::Value::Null,
        serde_json::json!({}),
        serde_json::json!({(UP045): ALWAYS}),
        serde_json::json!({(EXE004): ACCEPTED_SYNTAX}),
        serde_json::json!({(UP045): UNREVIEWED}),
    ] {
        let mut changed = request.clone();
        changed[RULE_APPLICABILITY] = phases;
        assert!(
            request_input::checked_worker_identity(&changed.to_string(), configuration).is_err()
        );
    }
    let mut absent = request;
    assert!(
        absent
            .as_object_mut()
            .ok_or(REQUEST_MUST_BE_AN_OBJECT)?
            .remove(RULE_APPLICABILITY)
            .is_some()
    );
    assert!(request_input::checked_worker_identity(&absent.to_string(), configuration).is_err());
    Ok(())
}

#[test]
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
        request_input::checked_worker_identity(&request.to_string(), Some(configuration))?;
        let wrong = if configuration == PYTHON_TARGET_3_10 {
            PYTHON_TARGET_3_14
        } else {
            PYTHON_TARGET_3_10
        };
        assert!(request_input::checked_worker_identity(&request.to_string(), Some(wrong)).is_err());
        let mut missing_phase = request;
        missing_phase[RULE_APPLICABILITY] = serde_json::Value::Null;
        assert!(
            request_input::checked_worker_identity(&missing_phase.to_string(), Some(configuration))
                .is_err()
        );
    }
    Ok(())
}
