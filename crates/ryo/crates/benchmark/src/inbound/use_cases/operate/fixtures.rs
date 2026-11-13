//! A bench directory with one profile, run through the operation over a scripted artifacts boundary and the local
//! suite and inventory.
#![expect(
    clippy::expect_used,
    reason = "fixture directories and files are test preconditions"
)]
use super::interface::OperateUseCase as _;
use super::types::{OperationRequest, OperationVerdict};
use super::use_case::OperateUseCaseImpl;
use crate::domain::operations::types::{BenchDirectory, Operation};
use crate::outbound::artifacts::fixtures::fake::{ArtifactsCall, ArtifactsFake};
use crate::outbound::execution::fixtures::fake::ExecutionFake;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use std::path::PathBuf;
use std::sync::Arc;

/// The bench directory, below the fixture root, every scenario runs on.
pub const BENCH: &str = "languages/fixture/benchmarks/linters/lint";

/// The profile of the fixture bench: the tool `lint` and one default, `R1`.
pub const PROFILE: &str = "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ndefaults = [\"R1\"]\n";

/// What one scenario varies.
#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    /// The operation run.
    pub operation: Operation,
    /// Whether the reference tool is on the search path.
    pub tool_found: bool,
    /// What the sealed worker answers.
    pub verdict: OperationVerdict,
}

/// What one scenario observed: the operation's answer, its standard output, and every artifacts request with the
/// fixture root removed from its paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observed {
    /// The verdict, or the refusal's text.
    pub answer: Result<OperationVerdict, String>,
    /// The bytes the operation wrote to its output.
    pub stdout: Vec<u8>,
    /// Every artifacts request, in order.
    pub calls: Vec<ArtifactsCall>,
}

/// Run one scenario.
#[must_use]
pub fn run(scenario: Scenario) -> Observed {
    let root = tempfile::tempdir().expect("fixture root");
    let root_path = root.path().canonicalize().expect("canonical root");
    let bench = root_path.join(BENCH);
    std::fs::create_dir_all(&bench).expect("bench directory");
    std::fs::write(bench.join("benchmark.toml"), PROFILE).expect("bench profile");
    let tool = root_path.join("lint");
    std::fs::write(&tool, "reference").expect("reference tool");
    let artifacts = Arc::new(ArtifactsFake {
        target: root_path.join("target"),
        tools: if scenario.tool_found {
            vec![(ToolName::from("lint"), tool)]
        } else {
            Vec::new()
        },
        stdout: b"{\"metric\":{}}\n".to_vec(),
        verdict: scenario.verdict,
        history: std::sync::Mutex::default(),
    });
    let operate = OperateUseCaseImpl::new(
        artifacts.clone(),
        Arc::new(crate::outbound::suite::factory::local()),
        Arc::new(crate::outbound::inventory::factory::local()),
        Arc::new(crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        )),
        Arc::new(ExecutionFake::default()),
    );
    let mut stdout = Vec::new();
    let answer = operate
        .operate(
            &OperationRequest {
                operation: scenario.operation,
                bench: BenchDirectory(bench),
            },
            &mut stdout,
        )
        .map(|report| report.verdict)
        .map_err(|error| error.to_string().replace(&root_path.display().to_string(), "<root>"));
    let relative = |path: &PathBuf| PathBuf::from("<root>").join(path.strip_prefix(&root_path).unwrap_or(path));
    let calls = artifacts
        .history
        .lock()
        .expect("history")
        .iter()
        .map(|call| match call {
            ArtifactsCall::SealWorker(path) => ArtifactsCall::SealWorker(relative(path)),
            ArtifactsCall::SealCurrent(path) => ArtifactsCall::SealCurrent(relative(path)),
            ArtifactsCall::Run(path, arguments) => ArtifactsCall::Run(
                relative(path),
                arguments
                    .iter()
                    .map(|argument| {
                        argument
                            .to_string_lossy()
                            .replace(&root_path.display().to_string(), "<root>")
                            .into()
                    })
                    .collect(),
            ),
        })
        .collect();
    Observed {
        answer,
        stdout,
        calls,
    }
}
