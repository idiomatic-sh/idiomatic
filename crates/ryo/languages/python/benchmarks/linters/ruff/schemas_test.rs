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

#[derive(Clone, Copy)]
enum SchemaInput {
    Offset(&'static str, usize, usize),
    UnavailableSource,
    RuleConfiguration,
    PinnedPhases,
}
type Input = SchemaInput;
type Value = usize;
type Failure = BoundaryFailure;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode_scalar_is_not_a_byte"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("é = 1\r\npass\n", 1, 2),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("crlf_counts_as_one_line_break"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("é = 1\r\npass\n", 2, 1),
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("exclusive_end_after_last_newline"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("pass\n", 2, 1),
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("bom_is_excluded_from_first_scalar_column"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("\u{feff}import os\n", 1, 8),
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("bom_does_not_shift_exclusive_end"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("\u{feff}import os\n", 1, 10),
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("lone_carriage_return_ends_a_line"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("x = 1\rtry:\r    pass\rexcept:\r    pass\r", 4, 1),
        expect: Expect::Value(20),
    },
    Case {
        name: CaseName("mixed_line_endings"),
        word: Word::Succeeds,
        input: || SchemaInput::Offset("one\r\ntwo\rthree\nfour", 4, 1),
        expect: Expect::Value(15),
    },
    Case {
        name: CaseName("zero_row_is_invalid"),
        word: Word::Refuses,
        input: || SchemaInput::Offset("é", 0, 1),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("zero_column_is_invalid"),
        word: Word::Refuses,
        input: || SchemaInput::Offset("é", 1, 0),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("row_outside_source_is_invalid"),
        word: Word::Refuses,
        input: || SchemaInput::Offset("é", 2, 1),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("column_outside_source_is_invalid"),
        word: Word::Refuses,
        input: || SchemaInput::Offset("é", 1, 5),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("unavailable_source_accepts_only_declared_file_origin_without_a_fix"),
        word: Word::Succeeds,
        input: || SchemaInput::UnavailableSource,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "rule_configuration_arguments_round_trip_as_toml_including_control_characters",
        ),
        word: Word::Succeeds,
        input: || SchemaInput::RuleConfiguration,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("the_pinned_registry_decodes_the_phase_of_every_rule"),
        word: Word::Succeeds,
        input: || SchemaInput::PinnedPhases,
        expect: Expect::Value(2),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        SchemaInput::Offset(source, row, column) => {
            offset(source, row, column).map_err(|error| failure(&error))
        }
        SchemaInput::UnavailableSource => {
            Ok(unavailable_source_accepts_only_declared_file_origin_without_a_fix())
        }
        SchemaInput::RuleConfiguration => {
            Ok(rule_configuration_arguments_round_trip_as_toml_including_control_characters())
        }
        SchemaInput::PinnedPhases => Ok(the_pinned_registry_decodes_the_phase_of_every_rule()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn unavailable_source_accepts_only_declared_file_origin_without_a_fix() -> usize {
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
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(ContextManifest {
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
                signal: ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
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
    let diagnostics = report.diagnostics.len();
    let mut undeclared = request.clone();
    undeclared.context_manifest =
        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed;
    assert!(matches!(
        decode(&undeclared, finding.clone()),
        Err(Error::Invalid(_))
    ));
    finding["end_location"]["column"] = serde_json::json!(2);
    assert!(matches!(
        decode(&request, finding.clone()),
        Err(Error::Invalid(_))
    ));
    finding["end_location"]["column"] = serde_json::json!(1);
    finding["fix"] = serde_json::json!({"edits":[]});
    assert!(matches!(decode(&request, finding), Err(Error::Invalid(_))));
    diagnostics
}

fn rule_configuration_arguments_round_trip_as_toml_including_control_characters() -> usize {
    use ryo_benchmark_contracts::domain::context::types::RuleConfiguration;
    let configuration = RuleConfiguration {
        external_prefixes: vec![
            ryo_benchmark_contracts::domain::context::types::ExternalPrefix("V\u{7f}\n\"".into()),
            ryo_benchmark_contracts::domain::context::types::ExternalPrefix("é\\".into()),
        ],
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
        configuration
            .external_prefixes
            .iter()
            .map(|prefix| prefix.0.as_str())
            .collect::<Vec<_>>()
    );
    let disabled: toml::Value = toml::from_str(args[3].to_str().unwrap()).unwrap();
    let expected =
        ryo_benchmark::domain::rule_context::service::literal_glob(&ryo::linter::RelativePath(
            "/root/pkg/[é]\u{7f}.py".into(),
        ));
    assert_eq!(
        disabled["lint"]["per-file-ignores"][expected.0.as_str()][0].as_str(),
        Some("F401")
    );
    args.len()
}

fn the_pinned_registry_decodes_the_phase_of_every_rule() -> usize {
    use ryo_benchmark_contracts::domain::requests::types::{ApplicabilityPolicy, RuleLabel};
    let phases = super::schemas::pinned_rule_phases().unwrap();
    [("UP045", "accepted-syntax"), ("EXE004", "always")]
        .into_iter()
        .filter(|(rule, policy)| {
            phases.get(&RuleLabel((*rule).into())) == Some(&ApplicabilityPolicy((*policy).into()))
        })
        .count()
}
