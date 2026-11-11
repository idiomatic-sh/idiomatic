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
    fn parse(value: &str) -> Option<Self> {
        match value {
            "success" => Some(Self::Success),
            "load-refusal" => Some(Self::LoadRefusal),
            "configure-refusal" => Some(Self::ConfigureRefusal),
            "execute-refusal" => Some(Self::ExecuteRefusal),
            "abandoned" => Some(Self::Abandoned),
            _ => None,
        }
    }
}

/// Each actual global subscriber runs in a fresh process, preserving its one-shot contract: what each child process
/// printed of its own session, after its scenario's label. The child runs the whole table, whose row for this scenario
/// states the parent's answer and not the child's, so the child's exit status says nothing and is not read.
pub(super) fn trace_sessions() -> String {
    let child = std::env::args()
        .find_map(|argument| argument.strip_prefix(CHILD_SCENARIO).map(str::to_owned));
    if let Some(label) = child {
        let observed =
            Scenario::parse(&label).map_or_else(|| "unknown scenario".to_owned(), actual_session);
        println!("{VERIFIED}{label}:{observed}");
        return observed;
    }
    [
        Scenario::Success,
        Scenario::LoadRefusal,
        Scenario::ConfigureRefusal,
        Scenario::ExecuteRefusal,
        Scenario::Abandoned,
    ]
    .into_iter()
    .map(|scenario| {
        let label = scenario.label();
        match std::process::Command::new(std::env::current_exe().unwrap_or_default())
            .args(["--exact", RUNNER, "--test-threads=1", "--nocapture"])
            .args(["--skip", &format!("{CHILD_SCENARIO}{label}")])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let marker = format!("{VERIFIED}{label}:");
                let observed = stdout
                    .lines()
                    .find_map(|line| line.split_once(&marker).map(|(_, observed)| observed))
                    .unwrap_or("no observation");
                format!("{label} {observed}")
            }
            Err(error) => format!("{label} not started: {error}"),
        }
    })
    .collect::<Vec<_>>()
    .join("; ")
}

fn actual_session(scenario: Scenario) -> String {
    let mut seen: Vec<String> = Vec::new();
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
        seen.push(format!(
            "{:?}",
            matches!(loaded, Err(ref error) if error.to_string() == "no candidate queries")
        ));
        seen.push(verify_trace(&trace));
        return seen.join(",");
    }
    let session = loaded.unwrap();
    {
        let span = tracing::info_span!("engine_session_after_load");
        let _entered = span.enter();
    }
    if matches!(scenario, Scenario::Abandoned) {
        drop(session);
        seen.push(verify_trace(&trace));
        return seen.join(",");
    }
    let configured = configure(&prepared.request, session);
    if matches!(scenario, Scenario::ConfigureRefusal) {
        seen.push(format!("{:?}", matches!(configured, Err(ref error) if error.to_string() == "selected or required producer has no registered query: R404")));
        seen.push(verify_trace(&trace));
        return seen.join(",");
    }
    let configured = configured.unwrap();
    {
        let span = tracing::info_span!("engine_session_after_configure");
        let _entered = span.enter();
    }
    let report = configured.execute();
    match scenario {
        Scenario::Success => seen.push(format!("{:?}", report.unwrap().report.diagnostics.len())),
        Scenario::ExecuteRefusal => seen.push(format!(
            "{:?}",
            report
                .unwrap_err()
                .to_string()
                .contains("incomplete candidate")
        )),
        Scenario::LoadRefusal | Scenario::ConfigureRefusal | Scenario::Abandoned => unreachable!(),
    }
    // The SDK's actual compile span happens inside execute, after ownership transfer.
    // Early guard destruction cannot leave it in the final flushed trace.
    seen.push(verify_trace(&trace));
    seen.join(",")
}
/// Which of the three spans the flushed trace holds: after load, after configure, and the compile.
fn verify_trace(path: &Path) -> String {
    let text = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .map_or_else(|| "no complete trace".to_owned(), |value| value.to_string());
    format!(
        "load {} configure {} compile {}",
        text.contains("engine_session_after_load"),
        text.contains("engine_session_after_configure"),
        text.contains("\"compile\"")
    )
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
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            vec![
                RuleId::try_from(if matches!(scenario, Scenario::ConfigureRefusal) {
                    "R404"
                } else {
                    "R1"
                })
                .unwrap(),
            ],
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: mode,
        context_manifest: manifest.map_or(
            ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed,
        ),
        root,
        rules,
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![RelativePath(file.into())],
        state: WorkerState::Fresh,
    }
}
