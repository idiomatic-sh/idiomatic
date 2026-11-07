//! The harness of the workloads: each is run for its rounds, its CPU time taken per round, and the run prints one Bencher
//! Metric Format document on stdout (everything meant for a person goes to stderr), which Asymptote judges against its
//! ratchet.

use std::io::Write as _;

/// How many rounds a workload is taken over: its `value` is the median of these.
const ROUNDS: usize = 3;

/// The measure every workload reports.
const MEASURE: &str = "cpu-ms";

/// The edge every workload runs its command with: `-q`, looking for its project from `start`, keeping nothing.
pub(crate) fn quiet_edge(start: &str) -> ryo_cli::edge::Edge {
    ryo_cli::resources::edge_at(
        ryo_language::inbound::use_cases::project::types::RootPath(start.into()),
        ryo_language::inbound::use_cases::project::types::RootPath(".".into()),
        ryo_cli::output::Format::Quiet,
    )
}

/// A workload's id (`workspace/check`) and the CPU time of each of its rounds, in milliseconds.
pub(crate) struct Measured {
    /// The benchmark's name in the document, the workload's resource and command (`query/run`).
    pub id: String,
    /// The CPU time of each round, in milliseconds, in the order they ran.
    pub rounds: Vec<f64>,
}

/// Runs `run` on the input `setup` makes, once per round, taking the CPU time of each run. A setup or a run that fails
/// panics, which fails the bench.
pub(crate) fn rounds<I>(id: &str, setup: impl FnMut() -> I, run: impl FnMut(I)) -> Measured {
    rounds_of(id, ROUNDS, setup, run)
}

/// As [`rounds`], over `count` rounds.
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
        cpu_time().saturating_sub(started).as_secs_f64() * 1_000.0_f64
    })
    .take(count)
    .collect();
    Measured {
        id: id.to_owned(),
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
