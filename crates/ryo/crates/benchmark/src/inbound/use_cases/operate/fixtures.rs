//! A bench directory with one profile, run through the operation over a scripted executables boundary and the local
//! suite and inventory.
#![expect(
    clippy::expect_used,
    reason = "fixture directories and files are test preconditions"
)]
use super::interface::OperateUseCase as _;
use super::types::{OperationRequest, OperationVerdict};
use super::use_case::OperateUseCaseImpl;
use crate::cases::slashed;
use crate::domain::operations::types::{BenchDirectory, BenchOperation};
use crate::outbound::executables::fixtures::fake::{ExecutablesCall, ExecutablesFake};
use crate::outbound::executables::types::RunVerdict;
use crate::outbound::execution::fixtures::fake::ExecutionFake;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The bench directory, below the fixture root, every scenario runs on.
pub const BENCH: &str = "languages/fixture/benchmarks/linters/lint";

/// The profile of the fixture bench: the tool `lint` and one default, `R1`.
pub const PROFILE: &str = "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"52367a6622b19f08825e915fad80c542ad4f4c34dbcebad9f5007994b3e39208\"]\ndefaults = [\"R1\"]\n";

/// The document the scripted worker writes: a project's store and peak memory and its CPU time, and one rule's
/// verification times.
const WORKER_DOCUMENT: &str = "{\"fixture/project\":{\"cpu-s\":{\"value\":2.5},\"peak-rss-bytes\":{\"value\":4096.0},\"store-bytes\":{\"value\":512.0}},\"fixture/verify/R1\":{\"total-ms\":{\"value\":3.0}}}\n";

/// The profile's deferral of its one default: the verification passes it as DEFERRED and runs no case.
pub const DEFERRED: &str = "\n[deferred]\ncontrol_flow = [\"R1\"]\n";

/// What one scenario varies.
#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    /// The operation run.
    pub operation: BenchOperation,
    /// Whether the reference tool is on the search path.
    pub tool_found: bool,
    /// What the sealed worker answers.
    pub verdict: OperationVerdict,
    /// Whether the bench's `rules/` holds `R9`, a rule that is neither a default nor a supporting rule.
    pub stray_rule: bool,
    /// Whether the profile defers its one default, so the verification passes without running a case.
    pub deferred: bool,
    /// How many processors the host lets the process run on.
    pub processors: usize,
}

/// What one scenario observed: the operation's answer, its standard output, and every executables request with the
/// fixture root removed from its paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observed {
    /// The verdict, or the refusal's text.
    pub answer: Result<OperationVerdict, String>,
    /// The bytes the operation wrote to its output.
    pub stdout: Vec<u8>,
    /// Every executables request, in order.
    pub calls: Vec<ExecutablesCall>,
    /// Whether the operation left a registration in its output directory.
    pub registered: bool,
}

/// Run one scenario through the use case over its scripted executables and its host.
#[must_use]
pub fn run(scenario: Scenario) -> Observed {
    observe(scenario, |executables| {
        use_case_on(executables, scenario.processors)
    })
}

/// The use case over the scripted executables, a host of `processors` processors and the local suite, inventory and
/// receipts.
#[must_use]
pub fn use_case_on(executables: Arc<ExecutablesFake>, processors: usize) -> OperateUseCaseImpl {
    OperateUseCaseImpl::new(
        executables,
        Arc::new(crate::outbound::suite::factory::local()),
        Arc::new(crate::outbound::inventory::factory::local()),
        Arc::new(crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        )),
        Arc::new(ExecutionFake::default()),
        Arc::new(crate::outbound::scheduler::factory::threads()),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake {
            processors: crate::outbound::host::types::ProcessorCount(
                std::num::NonZeroUsize::new(processors).unwrap_or(std::num::NonZeroUsize::MIN),
            ),
            ..crate::outbound::host::fixtures::fake::HostFake::default()
        }),
        Arc::new(crate::outbound::evidence::factory::bmf(
            Box::new(std::io::sink()),
            Box::new(std::io::sink()),
        )),
    )
}

/// The use case over the scripted executables on the four processors a bench is measured on.
#[must_use]
pub fn use_case(executables: Arc<ExecutablesFake>) -> OperateUseCaseImpl {
    use_case_on(
        executables,
        ryo_benchmark_contracts::domain::measurements::constants::MEASURED_WORKERS,
    )
}

/// A spelling with the fixture root as `<root>` and `/` between the components that follow it, on every platform.
///
/// A spelling that is `name=<path>` is rooted after its `=`; one that does not start at the root is as it was.
fn rooted(root: &Path, text: &str) -> String {
    if let Some((name, path)) = text.split_once('=') {
        return format!("{name}={}", rooted(root, path));
    }
    match Path::new(text).strip_prefix(root) {
        Ok(rest) if rest.as_os_str().is_empty() => "<root>".to_owned(),
        Ok(rest) => format!("<root>/{}", slashed(rest).0),
        Err(_outside) => text.to_owned(),
    }
}

/// Run one scenario through the use case `build` makes, observing its answer, output and requests.
///
/// # Panics
///
/// When the scenario's temporary bench directory, profile or reference tool cannot be written.
#[must_use]
pub fn observe(
    scenario: Scenario,
    build: impl Fn(Arc<ExecutablesFake>) -> OperateUseCaseImpl,
) -> Observed {
    let root = tempfile::tempdir().expect("fixture root");
    let root_path = root.path().canonicalize().expect("canonical root");
    let bench = root_path.join(BENCH);
    std::fs::create_dir_all(&bench).expect("bench directory");
    let profile = if scenario.deferred {
        format!("{PROFILE}{DEFERRED}")
    } else {
        PROFILE.to_owned()
    };
    std::fs::write(bench.join("benchmark.toml"), profile).expect("bench profile");
    std::fs::create_dir_all(bench.join("rules")).expect("the bench's rules directory");
    if scenario.stray_rule {
        std::fs::create_dir_all(bench.join("rules/R9")).expect("the stray rule's directory");
        std::fs::write(bench.join("rules/R9/query.ryoql"), "emit nothing;\n")
            .expect("the stray rule's query");
    }
    let tool = root_path.join("lint");
    std::fs::write(&tool, "reference").expect("reference tool");
    let executables = Arc::new(ExecutablesFake {
        target: root_path.join("target"),
        tools: if scenario.tool_found {
            vec![(ToolName::from("lint"), tool)]
        } else {
            Vec::new()
        },
        stdout: WORKER_DOCUMENT.as_bytes().to_vec(),
        verdict: match scenario.verdict {
            OperationVerdict::Passed => RunVerdict::Passed,
            OperationVerdict::Failed => RunVerdict::Failed,
        },
        history: std::sync::Mutex::default(),
    });
    let registration = crate::domain::operations::service::layout(
        &root_path.join("target"),
        &BenchDirectory(bench.clone()),
        scenario.operation,
    )
    .expect("the fixture bench is a bench directory")
    .root
    .join(crate::domain::operations::constants::REGISTRATION);
    let operate = build(Arc::clone(&executables));
    let mut stdout = Vec::new();
    let answer = operate
        .operate(
            &OperationRequest {
                operation: scenario.operation,
                bench: BenchDirectory(bench),
            },
            &mut stdout,
            &mut std::io::sink(),
        )
        .map(|report| report.verdict)
        .map_err(|error| {
            error
                .to_string()
                .replace(&root_path.display().to_string(), "<root>")
        });
    let relative = |path: &PathBuf| PathBuf::from(rooted(&root_path, &path.to_string_lossy()));
    let calls = executables
        .history
        .lock()
        .expect("history")
        .iter()
        .map(|call| match call {
            ExecutablesCall::SealWorker(path) => ExecutablesCall::SealWorker(relative(path)),
            ExecutablesCall::SealCurrent(path) => ExecutablesCall::SealCurrent(relative(path)),
            ExecutablesCall::Run(path, arguments) => ExecutablesCall::Run(
                relative(path),
                arguments
                    .iter()
                    .map(|argument| rooted(&root_path, &argument.to_string_lossy()).into())
                    .collect(),
            ),
            ExecutablesCall::Check(path, arguments) => ExecutablesCall::Check(
                relative(path),
                arguments
                    .iter()
                    .map(|argument| rooted(&root_path, &argument.to_string_lossy()).into())
                    .collect(),
            ),
        })
        .collect();
    Observed {
        answer,
        stdout,
        calls,
        registered: registration.is_file(),
    }
}
