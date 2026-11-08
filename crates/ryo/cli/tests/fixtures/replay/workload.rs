//! The runbook workloads over Django: the sealed Python worker run over a staged request (`verify_profiles.py --stage-request`: its
//! rules, fragments and benchmark.toml come from the measured source, and its `staged.json` records their digest), measured from outside as the shared collector measures it (wall, CPU and peak resident memory of the process). The inputs
//! are on disk or the workload is not available, and the reason is a value a test asserts.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The directory of the staged Django run: `request.json` (the corpus of 2,875 files and the staged rules) and the `staged.json`
/// receipt that `--stage-request` wrote beside it.
pub(crate) const PINNED: &str = "/tmp/ryo-django326";

/// The environment variable that names the sealed worker, as the preflight of the Python ruff benchmark reads it.
pub(crate) const WORKER: &str = "RYO_PREFLIGHT_WORKER";

/// Why a workload cannot run here.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NotAvailable {
    /// The staged run is not on this machine.
    CorpusAbsent(PathBuf),
    /// The request has no staging receipt: `staged.json` is absent or records no staged digest.
    RequestNotStaged(PathBuf),
    /// The sealed worker is not named.
    WorkerNotNamed,
    /// The sealed worker is named, and is not there.
    WorkerAbsent(PathBuf),
}

impl std::fmt::Display for NotAvailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorpusAbsent(path) => {
                write!(formatter, "the staged Django run {} is absent", path.display())
            }
            Self::RequestNotStaged(path) => write!(
                formatter,
                "{} records no staged digest: stage the request with verify_profiles.py --stage-request",
                path.display()
            ),
            Self::WorkerNotNamed => write!(formatter, "{WORKER} does not name a worker"),
            Self::WorkerAbsent(path) => {
                write!(formatter, "the worker {} is absent", path.display())
            }
        }
    }
}

/// What a workload runs on.
#[derive(Debug)]
pub(crate) struct Inputs {
    /// The staged run.
    pub corpus: PathBuf,
    /// The sealed worker.
    pub worker: PathBuf,
}

/// The inputs at the staged run `corpus` with the worker `worker` names, or why not.
///
/// # Errors
///
/// The input that is absent.
pub(crate) fn inputs_at(corpus: &Path, worker: Option<OsString>) -> Result<Inputs, NotAvailable> {
    if !corpus.join("request.json").is_file() {
        return Err(NotAvailable::CorpusAbsent(corpus.to_path_buf()));
    }
    let receipt = corpus.join("staged.json");
    let digest = std::fs::read_to_string(&receipt)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value["staged_digest"].as_str().map(str::to_owned));
    if digest.is_none_or(|digest| digest.is_empty()) {
        return Err(NotAvailable::RequestNotStaged(receipt));
    }
    let named = PathBuf::from(worker.ok_or(NotAvailable::WorkerNotNamed)?);
    if !named.is_file() {
        return Err(NotAvailable::WorkerAbsent(named));
    }
    Ok(Inputs {
        corpus: corpus.to_path_buf(),
        worker: named,
    })
}

/// The inputs of this machine.
///
/// # Errors
///
/// The input that is absent.
pub(crate) fn inputs() -> Result<Inputs, NotAvailable> {
    inputs_at(Path::new(PINNED), std::env::var_os(WORKER))
}

/// Which rules and files a run is asked for.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Workload {
    /// One rule over every file.
    OneRule(&'static str),
    /// Every selected rule over one file.
    OneFile,
    /// Every selected rule over every file.
    Everything,
}

/// What a run of the worker cost.
#[derive(Debug)]
pub(crate) struct Cost {
    /// Wall time, in milliseconds.
    pub wall: u128,
    /// CPU time of the process, user and kernel, in milliseconds.
    pub cpu: u128,
    /// The peak resident memory, in megabytes.
    pub peak: u128,
}

/// The request of `workload` cut from the staged request, with its cache at `cache`, written into `directory`; its path.
///
/// # Panics
///
/// When the staged request cannot be read or the request cannot be written.
pub(crate) fn request(
    inputs: &Inputs,
    workload: Workload,
    cache: &Path,
    directory: &Path,
) -> PathBuf {
    let text = std::fs::read_to_string(inputs.corpus.join("request.json"))
        .expect("the staged request is read");
    let mut request: serde_json::Value = serde_json::from_str(&text).expect("a request is JSON");
    request["cache"] = cache.to_string_lossy().as_ref().into();
    // The staged request is cut from a pinned one that predates the worker: the performance section the worker reads holds the diagnostics alone.
    let diagnostics = request["performance"]["diagnostics"].clone();
    request["performance"] = serde_json::json!({ "diagnostics": diagnostics });
    match workload {
        Workload::OneRule(rule) => request["selected_rules"] = serde_json::json!([rule]),
        Workload::OneFile => {
            let first = request["files"][0]
                .as_str()
                .expect("a file is a path")
                .to_owned();
            let hash = request["context_manifest"]["source_files"][&first].clone();
            request["context_manifest"]["source_files"] = serde_json::json!({ first.clone(): hash });
            request["files"] = serde_json::json!([first]);
        }
        Workload::Everything => {}
    }
    let path = directory.join("request.json");
    std::fs::write(&path, request.to_string()).expect("the request is written");
    path
}

/// The CPU, in milliseconds, and the peak memory, in kilobytes, of process `pid` so far, when it is alive.
fn sampled(pid: u32) -> Option<(u128, u128)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command is parenthesised and may hold spaces: the fields count from after its closing parenthesis.
    let fields: Vec<&str> = stat.get(stat.rfind(')')? + 2..)?.split(' ').collect();
    // `utime` and `stime` are the 14th and 15th fields of the line, the 12th and 13th after the state's place, in ticks of 10 ms.
    let ticks: u128 = fields.get(11)?.parse::<u128>().ok()? + fields.get(12)?.parse::<u128>().ok()?;
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kilobytes = status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse::<u128>()
            .ok()
    })?;
    Some((ticks * 10, kilobytes))
}

/// Runs the worker over `request` on one worker thread and measures it from outside: the last sample before it exits is its cost.
///
/// # Panics
///
/// When the worker cannot be started or fails.
pub(crate) fn run(inputs: &Inputs, request: &Path) -> Cost {
    let said = request.with_file_name("worker.stderr");
    let started = Instant::now();
    let mut child = Command::new(&inputs.worker)
        .args(["run-ryo", "--request-file"])
        .arg(request)
        .arg("python-target=3.10")
        .current_dir(inputs.corpus.join("corpus"))
        .env("RAYON_NUM_THREADS", "1")
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&said).expect("the worker's stderr file is made"))
        .spawn()
        .expect("the worker starts");
    let (mut cpu, mut kilobytes) = (0, 0);
    let status = loop {
        if let Some((read_cpu, read_peak)) = sampled(child.id()) {
            (cpu, kilobytes) = (read_cpu, read_peak);
        }
        if let Some(status) = child.try_wait().expect("the worker is waited for") {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let wall = started.elapsed().as_millis();
    assert!(
        status.success(),
        "the worker failed ({status}): {}",
        std::fs::read_to_string(&said)
            .unwrap_or_default()
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
    );
    Cost {
        wall,
        cpu,
        peak: kilobytes / 1024,
    }
}
