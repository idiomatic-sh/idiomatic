//! A memory measurement a test hands to asymptote's ratchet: one Bencher Metric Format document on stdout, which the tool's
//! command picks out of the test's output (`asymptote.toml`). Asymptote keeps each measure's best in its lock and fails a run
//! whose `lower_value` is over that best, so the tolerance is the distance of `lower_value` under `value`: a run passes while it
//! is within the stated percent of the best. `upper_value` is the value itself, so a run under the best lowers it.

use std::collections::BTreeMap;

/// One measure of a benchmark: its name, its value, and how many percent over its best a run may be.
pub(crate) struct Measure {
    pub name: &'static str,
    pub value: u64,
    pub tolerance_percent: u64,
}

/// Prints the measures of one benchmark as one document, and each on stderr for the person running the test.
///
/// # Panics
///
/// When a value does not fit the arithmetic of its tolerance.
#[expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the document on stdout is what the asymptote tool reads, and the line on stderr is what a person reads beside it"
)]
pub(crate) fn print(benchmark: &str, measures: &[Measure]) {
    let mut each = BTreeMap::new();
    for measure in measures {
        // `lower_value` is the value taken down by the tolerance, so a run fails when its value is over its best by more than the tolerance.
        let lower = u128::from(measure.value) * 100 / u128::from(100 + measure.tolerance_percent);
        let lower = u64::try_from(lower).expect("a lower value that fits a JSON number");
        eprintln!(
            "{benchmark}/{}: {} (tolerance {} percent)",
            measure.name, measure.value, measure.tolerance_percent
        );
        drop(each.insert(
            measure.name,
            serde_json::json!({
                "value": measure.value,
                "lower_value": lower,
                "upper_value": measure.value,
            }),
        ));
    }
    println!("{}", serde_json::json!({ benchmark: each }));
}
