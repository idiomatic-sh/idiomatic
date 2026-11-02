#[path = "worker_request_input_constants.rs"]
mod constants;
#[path = "composition/request_input.rs"]
mod request_input;
#[path = "composition/targets.rs"]
mod targets;

#[test]
fn reads_the_exact_explicit_request_file() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let request_path = directory.path().join("request.json");
    let request = r#"{"root":"/tmp/project","files":["a.py"]}"#;
    std::fs::write(&request_path, request)?;
    let mut arguments = [
        request_input::BenchmarkArgument("--request-file".into()),
        request_input::BenchmarkArgument(request_path.to_string_lossy().into_owned()),
    ]
    .into_iter();
    assert_eq!(request_input::worker_input(&mut arguments)?, request);
    assert!(arguments.next().is_none());
    Ok(())
}

#[test]
fn retains_the_explicit_inline_json_debug_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let request = r#"{"root":"/tmp/project","files":["a.py"]}"#;
    let mut arguments = [request_input::BenchmarkArgument(request.into())].into_iter();
    assert_eq!(request_input::worker_input(&mut arguments)?, request);
    assert!(arguments.next().is_none());
    Ok(())
}

#[test]
fn worker_execution_requires_the_actual_target_and_canonical_phase()
-> Result<(), Box<dyn std::error::Error>> {
    let request = serde_json::json!({
        "parser_configuration": "python-target=3.10",
        "rule_applicability": {"UP045": "accepted-syntax", "EXE004": "always"}
    });
    let configuration = Some("python-target=3.10");
    request_input::checked_worker_identity(&request.to_string(), configuration)?;
    for missing_or_wrong in [None, Some("python-target=3.11"), Some("python-target=3.9")] {
        assert!(
            request_input::checked_worker_identity(&request.to_string(), missing_or_wrong).is_err()
        );
    }
    let mut missing = request.clone();
    assert!(
        missing
            .as_object_mut()
            .ok_or("request must be an object")?
            .remove("parser_configuration")
            .is_some()
    );
    assert!(request_input::checked_worker_identity(&missing.to_string(), configuration).is_err());
    for phases in [
        serde_json::Value::Null,
        serde_json::json!({}),
        serde_json::json!({"UP045": "always"}),
        serde_json::json!({"EXE004": "accepted-syntax"}),
        serde_json::json!({"UP045": "unreviewed"}),
    ] {
        let mut changed = request.clone();
        changed["rule_applicability"] = phases;
        assert!(
            request_input::checked_worker_identity(&changed.to_string(), configuration).is_err()
        );
    }
    let mut absent = request;
    assert!(
        absent
            .as_object_mut()
            .ok_or("request must be an object")?
            .remove("rule_applicability")
            .is_some()
    );
    assert!(request_input::checked_worker_identity(&absent.to_string(), configuration).is_err());
    Ok(())
}

#[test]
fn every_explicit_supported_target_preserves_request_and_phase_identity()
-> Result<(), Box<dyn std::error::Error>> {
    for configuration in [
        "python-target=3.7",
        "python-target=3.8",
        "python-target=3.9",
        "python-target=3.10",
        "python-target=3.11",
        "python-target=3.12",
        "python-target=3.13",
        "python-target=3.14",
        "python-target=3.15",
    ] {
        let request = serde_json::json!({
            "parser_configuration": configuration,
            "rule_applicability": {"UP045": "accepted-syntax", "EXE004": "always"}
        });
        request_input::checked_worker_identity(&request.to_string(), Some(configuration))?;
        let wrong = if configuration == "python-target=3.10" {
            "python-target=3.14"
        } else {
            "python-target=3.10"
        };
        assert!(request_input::checked_worker_identity(&request.to_string(), Some(wrong)).is_err());
        let mut missing_phase = request;
        missing_phase["rule_applicability"] = serde_json::Value::Null;
        assert!(
            request_input::checked_worker_identity(&missing_phase.to_string(), Some(configuration))
                .is_err()
        );
    }
    Ok(())
}
