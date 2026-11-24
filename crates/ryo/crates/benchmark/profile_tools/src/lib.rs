//! Process profiling with standard Linux tools. The target program and its
//! output contract are supplied by the caller. This module has no engine or
//! benchmark-suite dependency.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

mod heap;
mod report;
pub use report::{compare, report, run_table};

/// One command over an already prepared input.
#[derive(Clone, Debug)]
pub struct CommandLine {
    /// Absolute executable path.
    pub executable: PathBuf,
    /// Arguments passed without shell interpretation.
    pub arguments: Vec<OsString>,
    /// Per-command environment inherited by the tool and its child.
    pub environment: BTreeMap<OsString, OsString>,
    /// Working directory of the command.
    pub directory: PathBuf,
    /// When the workload writes its answer to a file, compare that payload.
    pub answer: Option<PathBuf>,
    /// Canonicalize workspace references when each pass needs an isolated checkout.
    pub workspace_prefix: Option<PathBuf>,
    /// Optional argv prefix for the caller's resource scheduler.
    pub launcher: Vec<OsString>,
}

/// One observer. Run these separately to avoid counting one observer in another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observer {
    /// The ordinary image with the OS process meter.
    Native,
    /// Software CPU clock samples of the ordinary image.
    Samples,
    /// Allocation sites of the heap-compatible image.
    Heap,
    /// User-space instructions of the ordinary image.
    Instructions,
}

/// One tool run's capture and generated evidence paths.
#[derive(Debug)]
pub struct Observation {
    /// Exit status of the tool or, for an ordinary run, its child.
    pub status: ExitStatus,
    /// Elapsed time including the observer, excluding scheduler wait and report generation.
    pub elapsed: Duration,
    /// One-minute system load readings before and after the run.
    pub load: [Option<f64>; 2],
    /// Tool and target standard output, as produced by the observer.
    pub stdout: PathBuf,
    /// Tool and target standard error, as produced by the observer.
    pub stderr: PathBuf,
    /// GNU time fields, in wall seconds, user seconds, system seconds and KiB.
    pub resources: PathBuf,
}

/// Run one observer with fresh output files in `directory`.
///
/// # Errors
/// Returns OS errors from setting up, launching or waiting for the tool.
pub fn observe(
    command: &CommandLine,
    observer: Observer,
    directory: &Path,
) -> io::Result<Observation> {
    observe_before(command, observer, directory, None)
}

/// Run an observer with an optional wall deadline. The standard timeout tool
/// owns the instrumented process group and returns code 124 after a deadline.
///
/// # Errors
/// Returns OS errors from setting up, launching or waiting for the tool.
pub fn observe_before(
    command: &CommandLine,
    observer: Observer,
    directory: &Path,
    deadline: Option<Duration>,
) -> io::Result<Observation> {
    fs::create_dir_all(directory)?;
    let directory = directory.canonicalize()?;
    let stdout = directory.join("stdout");
    let stderr = directory.join("stderr");
    let resources = directory.join("resources");
    let mut timed = if let Some((program, arguments)) = command.launcher.split_first() {
        let mut wrapper = Command::new(program);
        wrapper.args(arguments).arg("time");
        wrapper
    } else {
        Command::new("time")
    };
    timed.args(["-f", "%e %U %S %M", "-o"]).arg(&resources);
    if let Some(deadline) = deadline {
        timed
            .args(["timeout", "--kill-after=15s"])
            .arg(format!("{}s", deadline.as_secs_f64()));
    }
    match observer {
        Observer::Native => {}
        Observer::Samples => {
            timed
                .args([
                    "perf",
                    "record",
                    "-q",
                    "-e",
                    "cpu-clock:u",
                    "-F",
                    "249",
                    "-o",
                ])
                .arg(directory.join("perf.data"))
                .arg("--");
        }
        Observer::Heap => {
            timed
                .args(["heaptrack", "--raw", "-o"])
                .arg(directory.join("heaptrack"));
        }
        Observer::Instructions => {
            timed
                .args([
                    "valgrind",
                    "--tool=cachegrind",
                    "--cache-sim=no",
                    "--branch-sim=no",
                ])
                .arg(option(
                    "--cachegrind-out-file=",
                    &directory.join("cachegrind.out"),
                ));
        }
    }
    let before = load();
    let start = Instant::now();
    let status = timed
        .arg(&command.executable)
        .args(&command.arguments)
        .envs(&command.environment)
        .current_dir(&command.directory)
        .stdin(Stdio::null())
        .stdout(File::create(&stdout)?)
        .stderr(File::create(&stderr)?)
        .status()?;
    let elapsed = if command.launcher.is_empty() {
        start.elapsed()
    } else {
        // The meter starts inside the resource lease.
        let timing = fs::read_to_string(&resources)?;
        let wall = timing
            .lines()
            .last()
            .and_then(|line| line.split_ascii_whitespace().next());
        wall.and_then(|seconds| seconds.parse().ok())
            .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
            .ok_or_else(|| io::Error::other("invalid GNU time wall clock"))?
    };
    let mut payload = command.answer.clone().unwrap_or(stdout);
    if let Some(prefix) = &command.workspace_prefix
        && payload.is_file()
    {
        let needle = prefix.as_os_str().as_encoded_bytes();
        if !needle.is_empty() {
            let bytes = fs::read(&payload)?;
            payload = directory.join("canonical-answer");
            let mut out = Vec::new();
            let mut rest = bytes.as_slice();
            while let Some(index) = rest
                .windows(needle.len())
                .position(|window| window == needle)
            {
                out.extend_from_slice(&rest[..index]);
                out.extend_from_slice(b"@WORKSPACE@");
                rest = &rest[index + needle.len()..];
            }
            out.extend_from_slice(rest);
            fs::write(&payload, out)?;
        }
    }
    Ok(Observation {
        status,
        elapsed,
        load: [before, load()],
        stdout: payload,
        stderr,
        resources,
    })
}

/// Compare a tool run's payload to the ordinary command's stdout, removing
/// just the Heaptrack driver's framing where present.
///
/// # Errors
/// Returns an error if either output cannot be read.
pub fn same_stdout(
    native: &Observation,
    observed: &Observation,
    observer: Observer,
) -> io::Result<bool> {
    let ordinary = fs::read(&native.stdout)?;
    let wrapped = fs::read(&observed.stdout)?;
    if observer != Observer::Heap {
        return Ok(ordinary == wrapped);
    }
    // Heaptrack 1.5 prints two start lines and a footer around its child's
    // stdout. A driver that writes the banners elsewhere is also accepted.
    if ordinary == wrapped {
        return Ok(true);
    }
    let mut lines = wrapped.splitn(3, |byte| *byte == b'\n');
    let Some((first, second, payload)) = lines
        .next()
        .zip(lines.next())
        .zip(lines.next())
        .map(|((first, second), payload)| (first, second, payload))
    else {
        return Ok(false);
    };
    Ok(first.starts_with(b"heaptrack output will be written to ")
        && second == b"starting application, this might take some time..."
        && payload
            .strip_prefix(ordinary.as_slice())
            .is_some_and(|footer| footer.starts_with(b"Heaptrack finished!")))
}

/// Run the tool's standard reporter. The raw recording remains available.
///
/// # Errors
/// Returns an error if the reporter cannot run or exits unsuccessfully.
pub fn summarize(observer: Observer, directory: &Path) -> io::Result<()> {
    let (mut report, name) = match observer {
        Observer::Native => return Ok(()),
        Observer::Samples => {
            let mut perf = Command::new("perf");
            perf.args([
                "report",
                "--stdio",
                "--no-children",
                "--show-nr-samples",
                "--sort",
                "symbol,dso",
                "--fields",
                "overhead,sample,symbol,dso",
                "--field-separator",
                "\t",
                "--percent-limit",
                "0",
                "-i",
            ])
            .arg(directory.join("perf.data"));
            (perf, "samples.txt")
        }
        Observer::Heap => {
            heap::decode(directory)?;
            let trace = heap_trace(directory)?;
            let mut heaptrack = Command::new("heaptrack_print");
            heaptrack
                .arg("-f")
                .arg(trace)
                .args(["-m", "false", "-n", "20", "-H"])
                .arg(directory.join("allocation-sizes.tsv"));
            (heaptrack, "allocations-raw.txt")
        }
        Observer::Instructions => {
            let mut cachegrind = Command::new("cg_annotate");
            cachegrind
                .args([
                    "--auto=no",
                    "--show=Ir",
                    "--no-show-percs",
                    "--threshold=0.01",
                ])
                .arg(directory.join("cachegrind.out"));
            (cachegrind, "instructions.txt")
        }
    };
    report.stdout(File::create(directory.join(name))?);
    let status = report.status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "{} exited {status}",
            report.get_program().to_string_lossy()
        )));
    }
    if observer == Observer::Heap {
        let status = Command::new("c++filt")
            .arg("--format=rust")
            .stdin(File::open(directory.join(name))?)
            .stdout(File::create(directory.join("allocations.txt"))?)
            .status()?;
        if !status.success() {
            return Err(io::Error::other(format!("c++filt exited {status}")));
        }
    }
    Ok(())
}

fn heap_trace(directory: &Path) -> io::Result<PathBuf> {
    fs::read_dir(directory)?
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .find(|entry| {
            let name = entry.file_name();
            name == "heaptrack.gz" || name == "heaptrack.zst"
        })
        .map(|entry| entry.path())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Heaptrack recording"))
}

/// One ordinary run and the three separately observed passes.
#[derive(Debug)]
pub struct Profile {
    /// Native process, serving as the output oracle.
    pub native: Observation,
    /// Software CPU clock recording.
    pub samples: Observation,
    /// Heaptrack on the allocation-compatible companion image.
    pub heap: Observation,
    /// Cachegrind, including an explicitly timed-out pass if applicable.
    pub instructions: Observation,
}

/// Collect one profile. The caller prepares the command and disposable input
/// state for each pass. Optional clocked detail belongs in the final callback.
/// The accepted exit code is supplied by the workload.
///
/// # Errors
/// A tool cannot be launched, the payloads differ, or a non-deadline run fails.
pub fn collect<T>(
    directory: &Path,
    expected_exits: &[i32],
    instruction_budget: Duration,
    mut invoke: impl FnMut(Observer, &Path) -> io::Result<CommandLine>,
    after_clocked: impl FnOnce() -> io::Result<T>,
) -> io::Result<(Profile, T)> {
    fs::create_dir_all(directory)?;
    let directory = directory.canonicalize()?;
    let mut prepared = |observer, name, core: Option<usize>| {
        let path = directory.join(name);
        let mut command = invoke(observer, &path)?;
        if let Some(core) = core.filter(|_| command.launcher.is_empty()) {
            command.launcher = vec!["taskset".into(), "-c".into(), core.to_string().into()];
        }
        Ok::<_, io::Error>((path, command))
    };
    let cores = available_cores();
    let native = prepared(Observer::Native, "native", cores.first().copied())?;
    let samples = prepared(Observer::Samples, "samples", cores.first().copied())?;
    let heap = prepared(Observer::Heap, "heap", cores.get(1).copied())?;
    let instructions = prepared(
        Observer::Instructions,
        "instructions",
        cores.get(2).copied(),
    )?;
    let collect_one = |observer, (path, command): (PathBuf, CommandLine)| {
        let deadline = (observer == Observer::Instructions).then_some(instruction_budget);
        let run = observe_before(&command, observer, &path, deadline)?;
        if deadline.is_some() && run.status.code() == Some(124) {
            return Ok(run);
        }
        // The Bash Heaptrack launcher completes its postprocessing even if a
        // target with findings returns a nonzero exit. Check its output below.
        let code = run.status.code();
        if !code.is_some_and(|value| expected_exits.contains(&value))
            && !(observer == Observer::Heap && code == Some(0))
        {
            return Err(io::Error::other(format!(
                "{observer:?} exited {}; see {}",
                run.status,
                run.stderr.display()
            )));
        }
        summarize(observer, &path)?;
        if observer == Observer::Heap {
            let log = fs::read_to_string(path.join("interpret-stderr"))?;
            let events = log.lines().find_map(|line| {
                line.trim_start()
                    .strip_prefix("allocations:")
                    .map(str::trim)
            });
            let completed = number(events, "completed Heaptrack allocation count")?;
            if completed != heap_totals(&directory)?.allocations {
                return Err(io::Error::other("Heaptrack allocation counts disagree"));
            }
        }
        Ok(run)
    };
    let clocks = || {
        Ok::<_, io::Error>((
            collect_one(Observer::Native, native)?,
            collect_one(Observer::Samples, samples)?,
            after_clocked()?,
        ))
    };
    let ((native, samples, detail), heap, instructions) = if cores.len() >= 3 {
        std::thread::scope(|scope| {
            let heap = scope.spawn(|| collect_one(Observer::Heap, heap));
            let instructions = scope.spawn(|| collect_one(Observer::Instructions, instructions));
            let clocked = clocks();
            let heap = heap
                .join()
                .map_err(|_| io::Error::other("Heaptrack worker panicked"))?;
            let instructions = instructions
                .join()
                .map_err(|_| io::Error::other("Cachegrind worker panicked"))?;
            Ok::<_, io::Error>((clocked?, heap?, instructions?))
        })?
    } else {
        (
            clocks()?,
            collect_one(Observer::Heap, heap)?,
            collect_one(Observer::Instructions, instructions)?,
        )
    };
    for (observer, run) in [
        (Observer::Samples, &samples),
        (Observer::Heap, &heap),
        (Observer::Instructions, &instructions),
    ] {
        if observer == Observer::Instructions && run.status.code() == Some(124) {
            continue;
        }
        if !same_stdout(&native, run, observer)? {
            return Err(io::Error::other(format!(
                "{observer:?} changed the ordinary output; see {}",
                run.stdout.display()
            )));
        }
    }
    Ok((
        Profile {
            native,
            samples,
            heap,
            instructions,
        },
        detail,
    ))
}

fn available_cores() -> Vec<usize> {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return Vec::new();
    };
    status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .into_iter()
        .flat_map(|list| list.trim().split(','))
        .filter_map(|range| {
            let (first, last) = range.split_once('-').unwrap_or((range, range));
            Some(first.parse::<usize>().ok()?..=last.parse::<usize>().ok()?)
        })
        .flatten()
        .take(3)
        .collect()
}

/// Totals from Heaptrack's exact allocation-size histogram. The tool supplies
/// the number of occurrences of each requested size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapTotals {
    /// Allocation events from the companion image.
    pub allocations: u64,
    /// Cumulative bytes requested by those events.
    pub bytes: u128,
}

/// Read the standard histogram written by Heaptrack.
///
/// # Errors
/// The histogram is missing or an emitted row has invalid numbers.
pub fn heap_totals(profile: &Path) -> io::Result<HeapTotals> {
    let histogram = fs::read_to_string(profile.join("heap/allocation-sizes.tsv"))?;
    let mut result = HeapTotals {
        allocations: 0,
        bytes: 0,
    };
    for line in histogram.lines() {
        let mut row = line.split_ascii_whitespace();
        let size = number(row.next(), "Heaptrack allocation size")?;
        let count = number(row.next(), "Heaptrack allocation count")?;
        result.allocations += count;
        result.bytes += u128::from(size) * u128::from(count);
    }
    Ok(result)
}

/// Return the Cachegrind total, or no total after a prescribed timeout.
///
/// # Errors
/// A completed run has an unreadable or invalid recording.
pub fn instruction_total(profile: &Profile, directory: &Path) -> io::Result<Option<u64>> {
    if profile.instructions.status.code() == Some(124) {
        Ok(None)
    } else {
        instructions(directory).map(Some)
    }
}

/// Read the ordinary process's observed maximum resident set.
///
/// # Errors
/// The process meter did not emit a valid peak.
pub fn peak_rss(profile: &Profile) -> io::Result<u64> {
    resources(&profile.native)?
        .1
        .checked_mul(1024)
        .ok_or_else(|| io::Error::other("native peak exceeds u64 bytes"))
}

fn resources(run: &Observation) -> io::Result<(f64, u64)> {
    let meter = fs::read_to_string(&run.resources)?;
    let fields: Vec<_> = meter
        .lines()
        .last()
        .unwrap_or_default()
        .split_ascii_whitespace()
        .collect();
    let [_, user, system, rss] = fields.as_slice() else {
        return Err(io::Error::other("invalid GNU time report"));
    };
    let cpu = |word: &str| {
        word.parse::<f64>()
            .map_err(|_| io::Error::other("invalid GNU time CPU clock"))
    };
    Ok((cpu(user)? + cpu(system)?, number(Some(rss), "peak KiB")?))
}

fn instructions(profile: &Path) -> io::Result<u64> {
    let trace = fs::read_to_string(profile.join("instructions/cachegrind.out"))?;
    let total = trace.lines().find_map(|line| line.strip_prefix("summary:"));
    number(total.map(str::trim), "Cachegrind instruction total")
}

fn number(word: Option<&str>, name: &str) -> io::Result<u64> {
    word.and_then(|word| word.parse().ok()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("missing or invalid {name}"),
        )
    })
}

/// The one-minute load reported by Linux.
#[must_use]
pub fn load() -> Option<f64> {
    fs::read_to_string("/proc/loadavg")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn option(prefix: &str, path: &Path) -> OsString {
    let mut result = OsString::from(prefix);
    result.push(path);
    result
}
