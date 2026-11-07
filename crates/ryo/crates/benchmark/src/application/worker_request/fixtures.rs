//! Real SDK session lifecycle probes; child selection is confined to test support.
use super::service::{configure, prepare};
use crate::outbound::engine::{factory, interface::EnginePort as _};
use ryo::linter::{Languages, RelativePath, RuleId};
use ryo_benchmark_contracts::domain::{
    context::types::{CaseContext, project_context_mode::value::ProjectContextMode},
    requests::types::{WorkerRequest, WorkerState},
};
use std::{collections::BTreeMap, path::Path};

const CHILD_SCENARIO: &str = "engine-trace-child=";
const RUNNER: &str = "application::worker_request::service_test::succeeds";
const VERIFIED: &str = "engine-trace-session-verified:";
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Success,
    LoadRefusal,
    ConfigureRefusal,
    ExecuteRefusal,
    Abandoned,
}
impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::LoadRefusal => "load-refusal",
            Self::ConfigureRefusal => "configure-refusal",
            Self::ExecuteRefusal => "execute-refusal",
            Self::Abandoned => "abandoned",
        }
    }
    fn parse(value: &str) -> Self {
        match value {
            "success" => Self::Success,
            "load-refusal" => Self::LoadRefusal,
            "configure-refusal" => Self::ConfigureRefusal,
            "execute-refusal" => Self::ExecuteRefusal,
            "abandoned" => Self::Abandoned,
            _ => panic!("unknown test child scenario"),
        }
    }
}

/// Each actual global subscriber runs in a fresh process, preserving its one-shot contract.
pub(super) fn trace_sessions() {
    let child = std::env::args()
        .find_map(|argument| argument.strip_prefix(CHILD_SCENARIO).map(str::to_owned));
    match child {
        Some(scenario) => {
            let scenario = Scenario::parse(&scenario);
            actual_session(scenario);
            println!("{VERIFIED}{}", scenario.label());
        }
        None => {
            for scenario in [
                Scenario::Success,
                Scenario::LoadRefusal,
                Scenario::ConfigureRefusal,
                Scenario::ExecuteRefusal,
                Scenario::Abandoned,
            ] {
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", RUNNER, "--test-threads=1", "--nocapture"])
                    .args(["--skip", &format!("{CHILD_SCENARIO}{}", scenario.label())])
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "scenario {:?}: stdout={} stderr={}",
                    scenario,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    String::from_utf8_lossy(&output.stdout)
                        .contains(&format!("{VERIFIED}{}", scenario.label())),
                    "the child must execute the actual lifecycle case; zero-test success is not proof"
                );
            }
        }
    }
}

fn actual_session(scenario: Scenario) {
    let scratch = tempfile::tempdir().unwrap();
    let trace = scratch.path().join("trace.json");
    let rules = scratch.path().join("rules");
    std::fs::create_dir_all(&rules).unwrap();
    if !matches!(scenario, Scenario::LoadRefusal) {
        std::fs::create_dir(rules.join("R1")).unwrap();
        std::fs::write(rules.join("R1/query.ryoql"), "let failures = select path as file, span from \"filesystem.read-failure\"; emit failures subject file key(file);").unwrap();
    }
    let request = request(scenario, scratch.path(), rules);
    let prepared = prepare(request).unwrap();
    let engine = factory::local(Languages::default(), Some(trace.clone()));
    let loaded = engine.load(&prepared.request);
    if matches!(scenario, Scenario::LoadRefusal) {
        assert!(matches!(loaded, Err(ref error) if error.to_string() == "no candidate queries"));
        verify_trace(&trace, false, false, false);
        return;
    }
    let session = loaded.unwrap();
    {
        let span = tracing::info_span!("engine_session_after_load");
        let _entered = span.enter();
    }
    if matches!(scenario, Scenario::Abandoned) {
        drop(session);
        verify_trace(&trace, true, false, false);
        return;
    }
    let configured = configure(&prepared.request, session);
    if matches!(scenario, Scenario::ConfigureRefusal) {
        assert!(
            matches!(configured, Err(ref error) if error.to_string() == "selected or required producer has no registered query: R404")
        );
        verify_trace(&trace, true, false, false);
        return;
    }
    let configured = configured.unwrap();
    {
        let span = tracing::info_span!("engine_session_after_configure");
        let _entered = span.enter();
    }
    let report = configured.execute();
    match scenario {
        Scenario::Success => assert_eq!(report.unwrap().diagnostics.len(), 1),
        Scenario::ExecuteRefusal => assert!(
            report
                .unwrap_err()
                .to_string()
                .contains("incomplete candidate")
        ),
        Scenario::LoadRefusal | Scenario::ConfigureRefusal | Scenario::Abandoned => unreachable!(),
    }
    // The SDK's actual compile span happens inside execute, after ownership transfer.
    // Early guard destruction cannot leave it in the final flushed trace.
    verify_trace(&trace, true, true, true);
}
fn verify_trace(path: &Path, loaded: bool, configured: bool, executed: bool) {
    let bytes = std::fs::read(path).unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("guard flushes complete Chrome JSON");
    let text = value.to_string();
    assert_eq!(text.contains("engine_session_after_load"), loaded);
    assert_eq!(text.contains("engine_session_after_configure"), configured);
    assert_eq!(text.contains("\"compile\""), executed);
}
fn request(scenario: Scenario, scratch: &Path, rules: std::path::PathBuf) -> WorkerRequest {
    let (root, manifest, mode, file) = if matches!(scenario, Scenario::ExecuteRefusal) {
        std::fs::write(scratch.join("input.txt"), "value = 1\n").unwrap();
        (
            scratch.to_owned(),
            None,
            ProjectContextMode::Isolated,
            "input.txt",
        )
    } else {
        let boundary = scratch.join("snapshot");
        let manifest = crate::outbound::context::interface::ContextPort::materialize(
            &crate::outbound::context::factory::local(),
            &boundary,
            &CaseContext {
                logical_root: "project".into(),
                auxiliary_files: BTreeMap::new(),
                missing: vec!["project/missing.py".into()],
            },
            &BTreeMap::new(),
        )
        .unwrap();
        (
            boundary.join("project"),
            Some(manifest),
            ProjectContextMode::ListedDisk,
            "missing.py",
        )
    };
    WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(vec![
            RuleId::try_from(if matches!(scenario, Scenario::ConfigureRefusal) {
                "R404"
            } else {
                "R1"
            })
            .unwrap(),
        ]),
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: mode,
        context_manifest: manifest,
        root,
        rules,
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![RelativePath(file.into())],
        state: WorkerState::Fresh,
    }
}
