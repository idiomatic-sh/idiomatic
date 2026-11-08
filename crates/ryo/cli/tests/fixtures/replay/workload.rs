//! The runbook workloads over Django: the sealed Python worker run over a staged request (`verify_profiles.py --stage-request`: its
//! rules, fragments and benchmark.toml come from the measured source, and its `staged.json` records their digest), measured from outside as the shared collector measures it (wall, CPU and peak resident memory of the process). The inputs
//! are on disk or the workload is not available, and the reason is a value a test asserts.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The environment variable that names the sealed worker, as the preflight of the Python ruff benchmark reads it. The staged
/// Django run is the worker's directory: `request.json` (the corpus of 2,875 files and the staged rules) and the `staged.json`
/// receipt that `--stage-request` wrote beside it.
pub(crate) const WORKER: &str = "RYO_PREFLIGHT_WORKER";

/// Why a workload cannot run here.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NotAvailable {
    /// The staged run is not on this machine.
    CorpusAbsent(PathBuf),
    /// The request has no staging receipt: `staged.json` is absent or records no staged digest.
    RequestNotStaged(PathBuf),
    /// The receipt `staged.json` does not describe what is beside it: its staged digest is not the digest of `profile/`, or its
    /// `request_sha256` is not the sha256 of `request.json`.
    StagedDiffers(PathBuf),
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
            Self::StagedDiffers(path) => write!(
                formatter,
                "{} does not match the staged profile/ or request.json beside it: stage the request again with verify_profiles.py --stage-request",
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
    pub staged: PathBuf,
    /// The sealed worker.
    pub worker: PathBuf,
}

/// The inputs at the staged run `staged` with the worker `worker` names, or why not.
///
/// # Errors
///
/// The input that is absent.
pub(crate) fn inputs_at(staged: &Path, worker: Option<OsString>) -> Result<Inputs, NotAvailable> {
    if !staged.join("request.json").is_file() {
        return Err(NotAvailable::CorpusAbsent(staged.to_path_buf()));
    }
    let receipt = staged.join("staged.json");
    let recorded = std::fs::read_to_string(&receipt)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let Some(digest) = recorded
        .as_ref()
        .and_then(|value| value["staged_digest"].as_str())
        .filter(|digest| !digest.is_empty())
    else {
        return Err(NotAvailable::RequestNotStaged(receipt));
    };
    let recorded_request = recorded
        .as_ref()
        .and_then(|value| value["request_sha256"].as_str());
    let request = std::fs::read(staged.join("request.json")).ok();
    let holds = staged_digest(&staged.join("profile")).is_ok_and(|found| found == digest)
        && request.is_some_and(|bytes| recorded_request == Some(sha256(&bytes).as_str()));
    if !holds {
        return Err(NotAvailable::StagedDiffers(receipt));
    }
    let named = PathBuf::from(worker.ok_or(NotAvailable::WorkerNotNamed)?);
    if !named.is_file() {
        return Err(NotAvailable::WorkerAbsent(named));
    }
    Ok(Inputs {
        staged: staged.to_path_buf(),
        worker: named,
    })
}

/// The sha256 of `bytes`, in lower-case hex.
fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    format!("{:x}", sha2::Sha256::digest(bytes))
}

/// The digest `verify_profiles.py --stage-request` records for the staged files under `profile`: the sha256 of the lines
/// `<sha256>  <path>` of every file, its path relative to `profile` with `/` between parts, in path order.
///
/// # Errors
///
/// A file or directory that cannot be read.
fn staged_digest(profile: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    let mut pending = vec![profile.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                let relative = path.strip_prefix(profile).unwrap_or(&path);
                let parts: Vec<_> = relative.iter().map(|part| part.to_string_lossy()).collect();
                files.push((parts.join("/"), sha256(&std::fs::read(&path)?)));
            }
        }
    }
    files.sort();
    let lines: String = files
        .iter()
        .map(|(path, digest)| format!("{digest}  {path}\n"))
        .collect();
    Ok(sha256(lines.as_bytes()))
}

/// The inputs of this machine.
///
/// # Errors
///
/// The input that is absent.
pub(crate) fn inputs() -> Result<Inputs, NotAvailable> {
    let worker = std::env::var_os(WORKER).ok_or(NotAvailable::WorkerNotNamed)?;
    let staged = Path::new(&worker)
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    inputs_at(&staged, Some(worker))
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
    let text = std::fs::read_to_string(inputs.staged.join("request.json"))
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

/// The corpus root `request` names, which the worker runs in.
///
/// # Panics
///
/// When the request cannot be read or names no root.
fn root(request: &Path) -> PathBuf {
    let text = std::fs::read_to_string(request).expect("the request is read");
    let request: serde_json::Value = serde_json::from_str(&text).expect("a request is JSON");
    PathBuf::from(request["root"].as_str().expect("the request names its corpus root"))
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
        .current_dir(root(request))
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
