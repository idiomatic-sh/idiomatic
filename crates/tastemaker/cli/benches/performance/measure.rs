//! The harness of the workloads: each is run for its rounds, its CPU time taken per round, and the run prints one Bencher
//! Metric Format document on stdout (everything meant for a person goes to stderr), which Asymptote judges against its
//! ratchet.

use std::io::Write as _;

/// How many rounds a workload is taken over: its `value` is the median of these.
const ROUNDS: usize = 3;

/// How many rounds a cold workload is taken over.
pub(crate) const COLD_ROUNDS: usize = 5;

/// The measure of a workload that takes the CPU time of its run.
const MEASURE: &str = "cpu-ms";

/// The measure of a workload that takes the peak memory of a process it starts.
const PEAK: &str = "peak-kib";

/// A workload's id (`workspace/check`) and the CPU time of each of its rounds, in milliseconds.
pub(crate) struct Measured {
    /// The benchmark's name in the document, the workload's command or use case (`workspace/check`).
    pub id: String,
    /// The measure of the document it is reported as: `cpu-ms`, or `peak-kib` for a peak.
    pub measure: &'static str,
    /// The measure of each round, in the order they ran: milliseconds of CPU time, or kibibytes of memory.
    pub rounds: Vec<f64>,
}

/// Runs `run` on the input `setup` makes, once per round, taking the CPU time of each run. A setup or a run that fails
/// panics, which fails the bench.
pub(crate) fn rounds<I>(id: &str, setup: impl FnMut() -> I, run: impl FnMut(I)) -> Measured {
    rounds_of(id, ROUNDS, setup, run)
}

/// `rounds` over `count` rounds: a cold cost is noisier than a warm one, so it is taken over more.
pub(crate) fn rounds_of<I>(
    id: &str,
    count: usize,
    mut setup: impl FnMut() -> I,
    mut run: impl FnMut(I),
) -> Measured {
    let rounds = std::iter::repeat_with(|| {
        let input = setup();
        let started = cpu_time();
        run(input);
        cpu_time().saturating_sub(started).as_secs_f64() * 1000.0_f64
    })
    .take(count)
    .collect();
    Measured {
        id: id.to_owned(),
        measure: MEASURE,
        rounds,
    }
}

/// Runs `run` on the input `setup` makes, once per round, and takes the kibibytes of memory it answers: the peak of the
/// process the run started. A setup or a run that fails panics, which fails the bench.
pub(crate) fn peaks<I>(
    id: &str,
    count: usize,
    mut setup: impl FnMut() -> I,
    mut run: impl FnMut(I) -> f64,
) -> Measured {
    let rounds = std::iter::repeat_with(|| run(setup())).take(count).collect();
    Measured {
        id: id.to_owned(),
        measure: PEAK,
        rounds,
    }
}

/// This process's CPU time, read through `cpu_time::ProcessTime`.
pub(crate) fn cpu_time() -> std::time::Duration {
    let now = cpu_time::ProcessTime::try_now();
    assert!(
        now.is_ok(),
        "the CPU clock cannot be read: {:?}",
        now.as_ref().err()
    );
    now.map_or(std::time::Duration::ZERO, |now| now.as_duration())
}

/// Prints the workloads as one Bencher Metric Format document on stdout: the median of the rounds is the `value`, the
/// fastest and the slowest are its `lower_value` and `upper_value`.
///
/// # Panics
///
/// When a workload ran no rounds.
#[expect(
    clippy::panic,
    clippy::expect_used,
    reason = "a benchmark fails by panicking: a workload that ran no rounds, or a stdout that refuses the document, leaves nothing to judge, and the message names which"
)]
pub(crate) fn print(measured: &[Measured]) {
    let mut document = serde_json::Map::new();
    for each in measured {
        let mut sorted = each.rounds.clone();
        sorted.sort_by(f64::total_cmp);
        let (Some(lower), Some(median), Some(upper)) =
            (sorted.first(), sorted.get(sorted.len() / 2), sorted.last())
        else {
            panic!("{}: the workload ran no rounds", each.id);
        };
        // The line is for a person; a stderr that refuses it leaves no one to tell.
        drop(writeln!(
            std::io::stderr(),
            "{}: median {} {median:.3} of {} rounds",
            each.id,
            each.measure,
            sorted.len()
        ));
        document.extend([(
            each.id.clone(),
            serde_json::json!({ each.measure: { "value": micro(*median), "lower_value": micro(*lower), "upper_value": micro(*upper) } }),
        )]);
    }
    writeln!(std::io::stdout(), "{}", serde_json::Value::Object(document))
        .expect("stdout takes the document");
}

/// `milliseconds` to the microsecond, the clock's useful resolution, so the document carries no float noise.
fn micro(milliseconds: f64) -> f64 {
    (milliseconds * 1000.0).round() / 1000.0
}
