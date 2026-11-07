//! The concrete ryo implementation executes the complete shared contract.
use super::adapter::LinterRyoAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::fixtures::contract::check;
use crate::outbound::linter::interface::LinterPort as _;
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::collections::BTreeMap;
fn concrete_implementation_answers_the_shared_contract() -> Result<(), Box<dyn std::error::Error>> {
    check(&LinterRyoAdapter {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        rule_observations: Default::default(),
        defaults: vec![ryo::linter::RuleId::try_from("fixture").unwrap()],
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        executable: "fixture".into(),
        rules: "rules".into(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
    });
    Ok(())
}
fn oversized_request_is_staged_as_json_file_instead_of_argv()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let state = root.path().join("state");
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.path().to_owned(),
        files: (0_usize..8_000_usize)
            .map(|index| RelativePath(format!("sources/module_{index:05}.py").into_boxed_str()))
            .collect(),
        state: state.clone(),
    };
    let adapter = LinterRyoAdapter {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        rule_observations: Default::default(),
        defaults: vec![ryo::linter::RuleId::try_from("fixture").unwrap()],
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        executable: "candidate".into(),
        rules: "rules".into(),
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(
            "python-target=3.10".into(),
        )),
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(ryo_benchmark_contracts::domain::requests::types::SourceLanguageName(
            "explicit-fixture-provider".into(),
        )),
    };
    let invocation = adapter.prepare(&request)?;
    let request_path = state.join("request.json");
    let bytes = std::fs::read(&request_path)?;
    let decoded: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert!(
        bytes.len() > 132_199,
        "fixture must exceed the Linux argv limit"
    );
    assert_eq!(invocation.arguments[0], "run-ryo");
    assert_eq!(invocation.arguments[1], "--request-file");
    assert_eq!(invocation.arguments[2], request_path.as_os_str());
    assert_eq!(invocation.arguments[3], "python-target=3.10");
    assert_eq!(decoded["files"].as_array().map(Vec::len), Some(8_000));
    assert_eq!(decoded["parser_configuration"], "python-target=3.10");
    assert_eq!(decoded["source_language"], "explicit-fixture-provider");
    assert!(
        invocation
            .arguments
            .iter()
            .all(|argument| argument.len() < 4096)
    );
    let spawned = std::process::Command::new(std::env::current_exe()?)
        .args(&invocation.arguments)
        .output()?;
    assert!(
        spawned.status.code().is_some(),
        "small file-based argv must spawn"
    );
    Ok(())
}

fn authored_observations_survive_actual_parent_to_worker_transport_and_explicit_override()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::outbound::worker::interface::WorkerPort as _;
    use ryo::linter::{
        ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationName, ObservationProducer, RuleId,
    };
    let root = tempfile::tempdir().unwrap();
    let consumer = RuleId::try_from("A001").unwrap();
    let producer = RuleId::try_from("B002").unwrap();
    let declarations = |output: &str| {
        BTreeMap::from([(
            consumer.clone(),
            vec![ObservationBinding {
                name: ObservationBindingName("published".into()),
                columns: [
                    ObservationKind::Node,
                    ObservationKind::Symbol,
                    ObservationKind::File,
                    ObservationKind::Span,
                    ObservationKind::Number,
                    ObservationKind::Flag,
                ]
                .into_iter()
                .enumerate()
                .map(|(index, kind)| ObservationColumn {
                    name: ObservationColumnName(format!("column-{index}").into()),
                    kind,
                })
                .collect(),
                producers: vec![ObservationProducer::Output {
                    rule: producer.clone(),
                    output: ObservationName(output.into()),
                }],
            }],
        )])
    };
    let original = declarations("matched λ\t\\source");
    let mut adapter = LinterRyoAdapter {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        rule_observations: original.clone().into(),
        defaults: vec![consumer.clone(), producer.clone()],
        project_context_mode: Default::default(),
        executable: "worker".into(),
        rules: "rules".into(),
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
    };
    use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared;
    use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Selected;
    for (index, (policy, explicit)) in [
        (Declared, None),
        (Declared, Some(declarations("different"))),
        (Declared, Some(BTreeMap::new())),
        (Selected, None),
        (Selected, Some(declarations("different"))),
        (Selected, Some(BTreeMap::new())),
    ]
    .into_iter()
    .enumerate()
    {
        adapter.rule_observation_selection = policy;
        let mut expected = explicit.clone().unwrap_or_else(|| original.clone());
        if policy == Selected && explicit.is_none() {
            expected.get_mut(&consumer).unwrap()[0].producers.clear();
        }
        let request = RunRequest {
            performance: Default::default(),
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
            selected_rules: if policy == Selected {
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(vec![consumer.clone()])
            } else {
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen
            },
            rule_observations: explicit.map_or(ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared, |declarations| {
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Declared(declarations.into())
            }),
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
            file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            root: root.path().to_owned(),
            files: vec![RelativePath("input.py".into())],
            state: root.path().join(index.to_string()),
        };
        let invocation = adapter.prepare(&request).unwrap();
        let path = std::path::PathBuf::from(&invocation.arguments[2]);
        let json = std::fs::read_to_string(&path).unwrap();
        let worker = crate::outbound::worker::factory::json(
            crate::outbound::worker::types::WorkerSource::Stored(path),
            Box::new(Vec::<u8>::new()),
        );
        let received = worker.receive().unwrap();
        assert!(matches!(
            &received.rule_observations,
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Declared(declared) if std::ops::Deref::deref(declared) == &expected
        ));
        assert_eq!(
            *adapter.rule_observations, original,
            "case override cannot mutate profile defaults"
        );
        if index == 0 {
            for kind in ["not-a-kind", "file"] {
                let mut forged: serde_json::Value = serde_json::from_str(&json).unwrap();
                if kind == "not-a-kind" {
                    forged["rule_observations"]["A001"][0]["columns"][0]["kind"] =
                        serde_json::json!(kind);
                } else {
                    forged["rule_observations"]["A001"][0]["unknown"] = serde_json::json!(true);
                }
                let worker = crate::outbound::worker::factory::json(
                    crate::outbound::worker::types::WorkerSource::Argument(forged.to_string().into()),
                    Box::new(Vec::<u8>::new()),
                );
                assert!(matches!(worker.receive(), Err(_refused)));
            }
        }
    }
    Ok(())
}

type Scenario = fn() -> Result<(), Box<dyn std::error::Error>>;
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("concrete_implementation_answers_the_shared_contract"),
        word: Word::Succeeds,
        input: || concrete_implementation_answers_the_shared_contract as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("oversized_request_is_staged_as_json_file_instead_of_argv"),
        word: Word::Succeeds,
        input: || oversized_request_is_staged_as_json_file_instead_of_argv as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("authored_observations_survive_actual_parent_to_worker_transport_and_explicit_override"),
        word: Word::Succeeds,
        input: || authored_observations_survive_actual_parent_to_worker_transport_and_explicit_override as Scenario,
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario().expect("the scenario holds");
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
