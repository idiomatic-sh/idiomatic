#[path = "worker_request_input_constants.rs"]
mod constants;
#[path = "composition/request_input.rs"]
mod request_input;

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
