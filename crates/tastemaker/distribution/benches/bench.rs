//! tastemaker's distribution perf: every registry's targets through its installed command: five
//! rounds of a cold run, and three of a warm run and a run after the target's one-file
//! edit (the first three rounds), timed with hyperfine, the process tree's memory sampled. It prints one Bencher
//! Metric Format document on stdout (benchmark = `<registry>/<target>`, measures
//! `cold-ms`, `warm-ms` and `edit-ms`, wall milliseconds: `value` the median of the rounds,
//! `lower_value` the fastest, `upper_value` the slowest) and every line meant for a person
//! on stderr; Asymptote's `perf` group judges the document against the ratchet, and the
//! bench asserts no budget. `-- --memory` runs the cargo registry's targets alone and takes `cold-peak-kib`, the peak memory of the cold check,
//! and `validate-cold-ms`, `validate-warm-ms` and `validate-peak-kib` of `validate` over the target's definitions, each run on four CPUs.
//! The bench asserts no budget. The performance job runs
//! `cargo bench -p idiomatic-sh-tastemaker --features distribution --bench distribution`, and it is never
//! part of a test run. It fails when its instrument fails its own checks, or a target
//! cannot run or prints other findings than it expects.

// The bench installs each registry's package and checks out each target through the
// `distribution` test target's own modules. Their tests, and what only the tests use,
// compile out of this bench; the test target holds that code to `dead_code` itself.
#[expect(
    dead_code,
    unused_imports,
    reason = "the smoke tests' modules, whose tests compile out of the bench"
)]
#[path = "../smoke_test.rs"]
mod smoke;

mod instrument;
mod measure;
mod resources;

use measure::{Numbers, ROUNDS, WARM_ROUNDS, allocation, measure};
use smoke::{run, target_test};
use std::cell::RefCell;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode};

/// What `--memory` takes of the cargo registry's targets alone instead of the timings: the peak of the cold check, and `validate`
/// over the target's definitions cold and warm.
#[derive(Default)]
struct Memory {
    /// The cold check's peak memory in kibibytes, one per round.
    check_peak: Vec<u128>,
    /// `validate`'s wall milliseconds on a fresh checkout, one per round.
    validate_cold: Vec<u128>,
    /// `validate`'s wall milliseconds over what the cold run proved, one per round.
    validate_warm: Vec<u128>,
    /// The larger peak memory of the two validations in kibibytes, one per round.
    validate_peak: Vec<u128>,
}

/// The wall milliseconds of each round of one target: cold, warm, and after the edit.
struct Measured {
    /// The benchmark's name in the document: `<registry>/<target>`.
    benchmark: String,
    /// The cold run's wall milliseconds, one per round.
    cold: Vec<u128>,
    /// The warm run's wall milliseconds, one per round that took it.
    warm: Vec<u128>,
    /// The wall milliseconds of the run after the edit, one per round that took it.
    edit: Vec<u128>,
    /// What `--memory` took, none otherwise.
    memory: Memory,
}

/// One round over a fresh checkout of `snapshot`: the cold run and, when `full`, the warm run, then
/// `edit.patch` applied and a third run, whose findings are those of `edit-findings.toml`.
fn round(command: &Path, target: &Path, snapshot: &Path, full: bool) -> std::io::Result<Numbers> {
    let scratch = tempfile::tempdir()?;
    let (tree, project) = target_test::checkout(command, target, snapshot, scratch.path())?;
    let findings = target.join("findings.toml");
    let (cold, cold_cpu, cold_peak) = measure(command, &project, scratch.path(), &findings)?;
    let mut numbers = Numbers {
        wall: vec![cold],
        cpu: vec![cold_cpu],
        peak: u128::from(cold_peak),
    };
    if !full {
        return Ok(numbers);
    }
    let (warm, warm_cpu, warm_peak) = measure(command, &project, scratch.path(), &findings)?;
    run(Command::new("git")
        .arg("apply")
        .arg(target.join("edit.patch"))
        .current_dir(&tree))?;
    let edited = target.join("edit-findings.toml");
    let (edit, edit_cpu, edit_peak) = measure(command, &project, scratch.path(), &edited)?;
    numbers.wall.extend([warm, edit]);
    numbers.cpu.extend([warm_cpu, edit_cpu]);
    numbers.peak = numbers.peak.max(u128::from(warm_peak.max(edit_peak)));
    Ok(numbers)
}

/// One round of `--memory` over a fresh checkout of `snapshot`, kept in `taken`: the cold check, whose peak is kept, then `validate`
/// over a second fresh checkout, cold and again warm.
fn memory_round(
    command: &Path,
    target: &Path,
    snapshot: &Path,
    taken: &mut Memory,
) -> std::io::Result<()> {
    let kibibytes = |bytes: u64| u128::from(bytes) / 1024;
    let scratch = tempfile::tempdir()?;
    let (_, project) = target_test::checkout(command, target, snapshot, scratch.path())?;
    let (_, _, check_peak) = measure(
        command,
        &project,
        scratch.path(),
        &target.join("findings.toml"),
    )?;
    taken.check_peak.push(kibibytes(check_peak));
    let proving = tempfile::tempdir()?;
    let (_, project) = target_test::checkout(command, target, snapshot, proving.path())?;
    let (cold, _, cold_peak) = measure::validate(command, &project, proving.path())?;
    let (warm, _, warm_peak) = measure::validate(command, &project, proving.path())?;
    taken.validate_cold.push(cold);
    taken.validate_warm.push(warm);
    taken.validate_peak.push(kibibytes(cold_peak.max(warm_peak)));
    Ok(())
}

/// Five rounds of the cold run, the first three of them with the warm run and the run after the edit; no fastest-run selection. Answers the target's line, its rounds,
/// and keeps the wall times in `kept` under `<registry>/<target>`.
fn prove(
    registry: &str,
    command: &Path,
    target: &Path,
    kept: &RefCell<Vec<Measured>>,
    memory: bool,
) -> std::io::Result<String> {
    allocation()?;
    let snapshot = target_test::preparation::snapshot(target)?;
    if memory {
        let mut taken = Memory::default();
        for _ in 0..ROUNDS {
            memory_round(command, target, &snapshot, &mut taken)?;
        }
        kept.borrow_mut().push(Measured {
            benchmark: format!(
                "{registry}/{}",
                target.file_name().unwrap_or_default().to_string_lossy()
            ),
            cold: Vec::new(),
            warm: Vec::new(),
            edit: Vec::new(),
            memory: taken,
        });
        return Ok(format!(
            "memory rounds={ROUNDS} workers={}",
            measure::WORKERS
        ));
    }
    let mut rounds = Vec::new();
    for index in 0..ROUNDS {
        let measured = round(command, target, &snapshot, index < WARM_ROUNDS)?;
        writeln!(
            std::io::stderr().lock(),
            "round={} wall-ms={} cpu-ms={} sampled-or-kernel-peak-bytes={}",
            index + 1,
            listed(&measured.wall),
            listed(&measured.cpu),
            measured.peak
        )?;
        rounds.push(measured);
    }
    let phase = |at: usize| {
        rounds
            .iter()
            .filter_map(|each| each.wall.get(at).copied())
            .collect::<Vec<_>>()
    };
    kept.borrow_mut().push(Measured {
        benchmark: format!(
            "{registry}/{}",
            target.file_name().unwrap_or_default().to_string_lossy()
        ),
        cold: phase(0),
        warm: phase(1),
        edit: phase(2),
        memory: Memory::default(),
    });
    Ok(format!(
        "cold rounds={ROUNDS} warm and edit rounds={WARM_ROUNDS} workers={} rounds={rounds:?}",
        measure::WORKERS
    ))
}

/// `values` as a person reads a list: `[1, 2, 3]`.
fn listed(values: &[u128]) -> String {
    let shown: Vec<String> = values.iter().map(u128::to_string).collect();
    format!("[{}]", shown.join(", "))
}

/// The median of the rounds of a cost: the middle one, or the mean of the two in the middle; 0 for
/// no rounds.
fn median(rounds: &[u128]) -> u128 {
    let mut sorted = rounds.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    let upper = sorted.get(middle).copied().unwrap_or_default();
    if sorted.len().is_multiple_of(2)
        && let Some(lower) = middle.checked_sub(1).and_then(|index| sorted.get(index))
    {
        lower.midpoint(upper)
    } else {
        upper
    }
}

/// The measures as one Bencher Metric Format document: the timings of every target, or with `memory` the memory measures
/// of the targets that took them. A measure is held under one asymptote tool, so no document carries both.
fn document(kept: &[Measured], memory: bool) -> serde_json::Value {
    let measure = |rounds: &[u128]| {
        let number = |value: u128| u64::try_from(value).unwrap_or(u64::MAX);
        serde_json::json!({
            "value": number(median(rounds)),
            "lower_value": number(rounds.iter().min().copied().unwrap_or_default()),
            "upper_value": number(rounds.iter().max().copied().unwrap_or_default()),
        })
    };
    serde_json::Value::Object(
        kept.iter()
            .map(|each| {
                let measures = if memory {
                    serde_json::json!({
                        "cold-peak-kib": measure(&each.memory.check_peak),
                        "validate-cold-ms": measure(&each.memory.validate_cold),
                        "validate-warm-ms": measure(&each.memory.validate_warm),
                        "validate-peak-kib": measure(&each.memory.validate_peak),
                    })
                } else {
                    serde_json::json!({
                        "cold-ms": measure(&each.cold),
                        "warm-ms": measure(&each.warm),
                        "edit-ms": measure(&each.edit),
                    })
                };
                (each.benchmark.clone(), measures)
            })
            .collect(),
    )
}

/// Checks the instrument, then measures every registry's targets and prints the document:
/// success unless the instrument fails a check or a target cannot run.
fn main() -> ExitCode {
    // `--memory` takes the memory measures of the cargo registry's targets alone, which a pull request can afford.
    let memory = std::env::args().any(|each| each == "--memory");
    let mut failed = Vec::new();
    let kept = RefCell::new(Vec::new());
    if let Err(error) = instrument::check() {
        failed.push(format!("the instrument fails its check: {error}"));
    } else {
        for (name, install, _) in target_test::REGISTRIES
            .iter()
            .filter(|(name, ..)| !memory || *name == "cargo")
        {
            // The registry's targets through the command its package installs; only a
            // target that cannot run is a failure (`target_test::each`).
            let measured = tempfile::tempdir().and_then(|directory| {
                target_test::each(
                    name,
                    &target_test::targets(name)?,
                    || install(directory.path()),
                    |command, target| prove(name, command, target, &kept, memory),
                    &mut std::io::stderr(),
                )
            });
            match measured {
                Ok(lines) => failed.extend(lines),
                Err(error) => failed.push(format!("{name}: cannot run: {error}")),
            }
        }
    }
    if let Err(error) = writeln!(
        std::io::stdout().lock(),
        "{}",
        document(&kept.borrow(), memory)
    ) {
        failed.push(format!("the document cannot be printed: {error}"));
    }
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        let mut error = std::io::stderr().lock();
        for line in &failed {
            // A closed standard error leaves nothing to report to: the exit code still says the bench failed.
            drop(writeln!(error, "{line}"));
        }
        ExitCode::FAILURE
    }
}
