//! Scalar columns, byte offsets and line-ending boundaries.
use super::schemas::offset;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

#[derive(Debug, PartialEq, Eq)]
enum BoundaryFailure {
    Invalid,
    Unexpected,
}
fn failure(error: &Error) -> BoundaryFailure {
    match error {
        Error::Invalid(_) => BoundaryFailure::Invalid,
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => BoundaryFailure::Unexpected,
    }
}
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};

type Input = (&'static str, usize, usize);
type Value = usize;
type Failure = BoundaryFailure;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode_scalar_is_not_a_byte"),
        word: Word::Succeeds,
        input: || ("é = 1\r\npass\n", 1, 2),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("crlf_counts_as_one_line_break"),
        word: Word::Succeeds,
        input: || ("é = 1\r\npass\n", 2, 1),
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("exclusive_end_after_last_newline"),
        word: Word::Succeeds,
        input: || ("pass\n", 2, 1),
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("bom_is_excluded_from_first_scalar_column"),
        word: Word::Succeeds,
        input: || ("\u{feff}import os\n", 1, 8),
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("bom_does_not_shift_exclusive_end"),
        word: Word::Succeeds,
        input: || ("\u{feff}import os\n", 1, 10),
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("lone_carriage_return_ends_a_line"),
        word: Word::Succeeds,
        input: || ("x = 1\rtry:\r    pass\rexcept:\r    pass\r", 4, 1),
        expect: Expect::Value(20),
    },
    Case {
        name: CaseName("mixed_line_endings"),
        word: Word::Succeeds,
        input: || ("one\r\ntwo\rthree\nfour", 4, 1),
        expect: Expect::Value(15),
    },
    Case {
        name: CaseName("zero_row_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 0, 1),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("zero_column_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 1, 0),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("row_outside_source_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 2, 1),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("column_outside_source_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 1, 5),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
];

fn subject((source, row, column): Input) -> Result<Value, Failure> {
    offset(source, row, column).map_err(|error| failure(&error))
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
fn unavailable_source_accepts_only_declared_file_origin_without_a_fix() {
    use ryo::linter::{RelativePath, RuleId};
    use ryo_benchmark_contracts::domain::context::types::ContextManifest;
    use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
    use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
    use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
    use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
    use ryo_benchmark_contracts::domain::requests::types::RunRequest;
    let root = std::path::PathBuf::from("/snapshot/project");
    let file = RelativePath("missing.py".into());
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        staged_sources: None,
        file_metadata: None,
        context_manifest: Some(ContextManifest {
            boundary: "/snapshot".into(),
            logical_root: "project".into(),
            auxiliary_files: Default::default(),
            source_files: Default::default(),
            missing: vec!["project/missing.py".into()],
        }),
        root: root.clone(),
        files: vec![file],
        state: root.join("cache"),
    };
    let defaults = [RuleId::try_from("E902").unwrap()].into();
    let mut finding = serde_json::json!({"code":"E902", "filename":root.join("missing.py"), "location":{"row":1,"column":1}, "end_location":{"row":1,"column":1}, "cell":null, "fix":null});
    let decode = |request: &RunRequest, finding| {
        super::schemas::decode(
            request,
            &ProcessOutput {
                signal: None,
                status: ProcessExitCode(1),
                stdout: CapturedBytes(serde_json::to_vec(&vec![finding]).unwrap()),
                stderr: CapturedBytes(Vec::new()),
                wall: Default::default(),
                resources: ResourceObservation::WallOnly,
            },
            &defaults,
            &Default::default(),
        )
    };
    let report = decode(&request, finding.clone()).unwrap();
    assert_eq!(report.diagnostics[0].start.0, 0);
    assert_eq!(report.diagnostics[0].end.0, 0);
    let mut undeclared = request.clone();
    undeclared.context_manifest = None;
    assert!(decode(&undeclared, finding.clone()).is_err());
    finding["end_location"]["column"] = serde_json::json!(2);
    assert!(decode(&request, finding.clone()).is_err());
    finding["end_location"]["column"] = serde_json::json!(1);
    finding["fix"] = serde_json::json!({"edits":[]});
    assert!(decode(&request, finding).is_err());
}

#[test]
fn rule_configuration_arguments_round_trip_as_toml_including_control_characters() {
    use ryo_benchmark_contracts::domain::context::types::RuleConfiguration;
    let configuration = RuleConfiguration {
        external_prefixes: vec!["V\u{7f}\n\"".into(), "é\\".into()],
        disabled_rules: std::collections::BTreeMap::from([(
            ryo::linter::RelativePath("pkg/[é]\u{7f}.py".into()),
            vec!["F401".try_into().unwrap()],
        )]),
    };
    let args =
        super::schemas::configuration_arguments(&configuration, std::path::Path::new("/root"))
            .unwrap();
    assert_eq!(args.len(), 4);
    let external: toml::Value = toml::from_str(args[1].to_str().unwrap()).unwrap();
    assert_eq!(
        external["lint"]["external"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        configuration.external_prefixes
    );
    let disabled: toml::Value = toml::from_str(args[3].to_str().unwrap()).unwrap();
    let expected =
        ryo_benchmark::domain::suite::rule_context::literal_glob("/root/pkg/[é]\u{7f}.py");
    assert_eq!(
        disabled["lint"]["per-file-ignores"][expected.as_str()][0].as_str(),
        Some("F401")
    );
}
