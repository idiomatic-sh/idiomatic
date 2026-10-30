//! The harness of the workloads: each is run for its rounds, its CPU time taken per round, and the run prints one Bencher
//! Metric Format document on stdout (everything meant for a person goes to stderr), which Asymptote judges against its
//! ratchet.

use std::io::Write as _;

/// How many rounds a workload is taken over: its `value` is the median of these.
const ROUNDS: usize = 3;

/// How many rounds a cold workload is taken over.
pub(crate) const COLD_ROUNDS: usize = 5;

/// The measure every workload reports.
const MEASURE: &str = "cpu-ms";

/// A workload's id (`workspace/check`) and the CPU time of each of its rounds, in milliseconds.
pub(crate) struct Measured {
    /// The benchmark's name in the document, the workload's command or use case (`ratchet/check`).
    pub id: String,
    /// The CPU time of each round, in milliseconds, in the order they ran.
    pub rounds: Vec<f64>,
}

/// Runs `run` on the input `setup` makes, once per round, taking the CPU time of each run.
///
/// # Errors
///
/// The first error a setup, a run or the CPU clock answers, which ends the bench.
pub(crate) fn rounds<I>(
    id: &str,
    setup: impl FnMut() -> Result<I, Box<dyn std::error::Error>>,
    run: impl FnMut(I) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<Measured, Box<dyn std::error::Error>> {
    rounds_of(id, ROUNDS, setup, run)
}

/// As [`rounds`], over `count` rounds: [`COLD_ROUNDS`] for a cold workload.
///
/// # Errors
///
/// As [`rounds`].
pub(crate) fn rounds_of<I>(
    id: &str,
    count: usize,
    mut setup: impl FnMut() -> Result<I, Box<dyn std::error::Error>>,
    mut run: impl FnMut(I) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<Measured, Box<dyn std::error::Error>> {
    let rounds = std::iter::repeat_with(|| -> Result<f64, Box<dyn std::error::Error>> {
        let input = setup()?;
        let started = cpu_time()?;
        run(input)?;
        Ok(cpu_time()?.saturating_sub(started).as_secs_f64() * 1000.0_f64)
    })
    .take(count)
    .collect::<Result<Vec<f64>, Box<dyn std::error::Error>>>()?;
    Ok(Measured {
        id: id.to_owned(),
        rounds,
    })
}

/// This process's CPU time, read through `cpu_time::ProcessTime`.
///
/// # Errors
///
/// When the CPU clock cannot be read.
pub(crate) fn cpu_time() -> std::io::Result<std::time::Duration> {
    cpu_time::ProcessTime::try_now().map(|now| now.as_duration())
}

/// Prints the workloads as one Bencher Metric Format document on stdout: the median of the rounds is the `value`, the
/// fastest and the slowest are its `lower_value` and `upper_value`.
///
/// # Errors
///
/// When a workload ran no rounds, or stdout or stderr cannot be written.
pub(crate) fn print(measured: &[Measured]) -> Result<(), Box<dyn std::error::Error>> {
    let document = measured
        .iter()
        .map(|each| -> Result<(String, serde_json::Value), Box<dyn std::error::Error>> {
            let mut sorted = each.rounds.clone();
            sorted.sort_by(f64::total_cmp);
            let (Some(lower), Some(median), Some(upper)) =
                (sorted.first(), sorted.get(sorted.len() / 2), sorted.last())
            else {
                return Err(format!("{}: the workload ran no rounds", each.id).into());
            };
            writeln!(
                std::io::stderr(),
                "{}: median CPU {median:.3} ms of {} rounds",
                each.id,
                sorted.len()
            )?;
            Ok((
                each.id.clone(),
                serde_json::json!({ MEASURE: { "value": micro(*median), "lower_value": micro(*lower), "upper_value": micro(*upper) } }),
            ))
        })
        .collect::<Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error>>>()?;
    writeln!(std::io::stdout(), "{}", serde_json::Value::Object(document))?;
    Ok(())
}

/// `milliseconds` to the microsecond, the clock's useful resolution, so the document carries no float noise.
fn micro(milliseconds: f64) -> f64 {
    (milliseconds * 1000.0).round() / 1000.0
}
