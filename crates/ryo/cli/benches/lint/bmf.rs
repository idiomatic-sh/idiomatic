//! What ryo costs over ruff, as one Bencher Cost Format document on stdout (everything meant for a person goes to
//! stderr): `{ "<target>": { "<metric>": { "value": 831, "lower_value": 794, "upper_value": 893 } } }`. The benchmark is
//! the target the cost is of (`ruff`), the measure is the metric (`cold-milliseconds`,
//! `cold-cpu-milliseconds`, `spike-cpu-milliseconds`, `warm-milliseconds`, `edit-milliseconds`, `peak-megabytes`,
//! `store-megabytes`, `store-files`); `value` is the median of the rounds, `lower_value` and `upper_value` the fastest
//! and the slowest. Asymptote's `perf` group judges the document against the ratchet it keeps in its lock (best,
//! tolerance, machine class); this bench asserts no budget and keeps no ratchet of its own.
//!
//! The method: a cost is judged by the MEDIAN of several rounds, not by one run, because one run moves with the load of
//! the machine and, for the peak, with how many workers hold their tables at once. The cold cost of ruff and its CPU
//! are the median of 5 rounds ([`COLD_ROUNDS`], `cold.rs`), every other cost the median of 3 ([`ROUNDS`]; the cache
//! rows of `cache.rs`, `--rounds` to change it). "Cold" means no store of ryo's own, not a cold disk: before every
//! round the target is read once, in one pass, outside the timer (`phases::pre_read`), so that the page cache holds it
//! and is the same in every round. A tolerance is the spread of that median measured when the ratchet is seeded, over
//! 6 or more rounds, plus two points. What the seed of 2026-10-17 measured:
//!
//! - peak: 6 rounds 547 536 547 546 534 550 MB: median 546; medians of three in a row differ by 0.2%
//! - cold ms: 15 rounds in 3 sets of 5: medians 831, 830, 890 ms; they differ by 7.2%
//! - edit: 6 rounds 252 241 259 259 269 244 ms: median 255; medians of three in a row differ by 2.7%
//! - warm: 6 rounds 84 98 96 81 79 81 ms: median 82; medians of three in a row differ by 18%
//!
//! Before the page cache was made equal the edit read 243 to 522 ms and its tolerance was 45%. Warm keeps a wide
//! tolerance for a named reason: the run is a process that starts, reads one record and exits in 80 ms, and 15 ms of
//! that is the machine (the same file read twice varies by it). The first round of a process is 5 to 15 percent slower
//! than the rest (heap growth, first touch of the pools); the median of five takes it out, and a round that is not the
//! first never needed the fix.
//!
//! The store a warm run leaves in the cache directory of a target (v1 left an 18 GB store; the store here is bounded)
//! is one file.

/// How many rounds every cost is taken over, but the cold cost of ruff: the median of these is the `value`.
pub(crate) const ROUNDS: usize = 3;

/// How many rounds the cold cost of ruff and its CPU are taken over.
pub(crate) const COLD_ROUNDS: usize = 5;

/// One cost of one target, a metric of it, and the value of each of its rounds.
pub(crate) struct Cost {
    /// The target the cost is of, the document's benchmark (`ruff`).
    pub target: &'static str,
    /// The metric, the document's measure (`cold-milliseconds`).
    pub metric: &'static str,
    /// The value of each round, in the order they ran.
    pub rounds: Vec<u128>,
}

/// The median of the rounds of a cost: the middle one, or the mean of the two in the middle.
///
/// # Panics
///
/// When there are no rounds.
#[must_use]
pub(crate) fn median(rounds: &[u128]) -> u128 {
    assert!(
        !rounds.is_empty(),
        "a cost is taken over at least one round"
    );
    let mut sorted = rounds.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    let below = if sorted.len() % 2 == 1 {
        middle
    } else {
        middle.saturating_sub(1)
    };
    match (sorted.get(below), sorted.get(middle)) {
        (Some(low), Some(high)) => low.midpoint(*high),
        _ => 0,
    }
}

/// The rounds as a person reads them, in the order they ran: `831, 830, 890`.
#[must_use]
pub(crate) fn listed(rounds: &[u128]) -> String {
    rounds
        .iter()
        .map(u128::to_string)
        .collect::<Vec<String>>()
        .join(", ")
}

/// Prints the metrics as one document on stdout, and each with its rounds on stderr.
///
/// # Panics
///
/// When a metric has no rounds, or a value does not fit a JSON number.
pub(crate) fn print(metrics: &[Cost]) {
    let mut document = serde_json::Map::new();
    for each in metrics {
        eprintln!(
            "{}/{}: median {} of [{}]",
            each.target,
            each.metric,
            median(&each.rounds),
            listed(&each.rounds)
        );
        let number = |value: u128| {
            serde_json::Value::from(u64::try_from(value).expect("a value that fits a JSON number"))
        };
        let (lower, upper) = (
            each.rounds.iter().min().copied().unwrap_or_default(),
            each.rounds.iter().max().copied().unwrap_or_default(),
        );
        let measures = document
            .entry(each.target)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(measures) = measures.as_object_mut() {
            // A target names each metric once, so no value is replaced.
            drop(measures.insert(
                each.metric.to_owned(),
                serde_json::json!({
                    "value": number(median(&each.rounds)),
                    "lower_value": number(lower),
                    "upper_value": number(upper),
                }),
            ));
        }
    }
    println!("{}", serde_json::Value::Object(document));
}
