//! The one writer of a memory measurement for asymptote's ratchet, a dev-dependency of the tests of ryo and of Tastemaker. A test
//! hands it the measures it took; the tool that runs the test says where the Bencher Metric Format document goes and how far
//! over its best each measure may rise, so the tolerance lives in the tool's definition and nowhere else:
//!
//! - `RATCHET_BMF_FILE`: the file the document is written to. Unset, the test is run for its assertions alone and nothing is written.
//! - `RATCHET_TOLERANCES`: `<measure>=<percent>` pairs, comma separated; a measure with none is refused.
//!
//! Asymptote keeps each measure's best in its lock and fails a run whose `lower_value` is over that best, so `lower_value` is the
//! value taken down by the tolerance; `upper_value` is the value itself, so a run under the best lowers it.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot write its measures has nothing to assert: the panic is the failure of the test"
)]

use std::collections::BTreeMap;

/// The variable naming the file the document is written to.
const FILE: &str = "RATCHET_BMF_FILE";

/// The variable naming each measure's tolerance in percent.
const TOLERANCES: &str = "RATCHET_TOLERANCES";

/// One measure of a benchmark: its name and its value.
pub struct Measure {
    /// The measure's name in the lock.
    pub name: &'static str,
    /// Its value.
    pub value: u64,
}

/// The percent the tool allows `name` to rise over its best.
fn tolerance(name: &str) -> u64 {
    let said = std::env::var(TOLERANCES).unwrap_or_default();
    said.split(',')
        .find_map(|pair| {
            let (measure, percent) = pair.split_once('=')?;
            (measure.trim() == name).then(|| percent.trim().parse::<u64>().ok())?
        })
        .unwrap_or_else(|| panic!("{TOLERANCES} ({said:?}) holds no percent for the measure {name}"))
}

/// Writes the measures of one benchmark as one document to the file the tool named, when it named one.
///
/// # Examples
///
/// ```
/// // No tool named a file, so nothing is written.
/// idiomatic_sh_ratchet::write("ruff", &[idiomatic_sh_ratchet::Measure { name: "store-bytes", value: 1 }]);
/// ```
///
/// # Panics
///
/// When the tool named a file and a measure has no tolerance, or the file cannot be written.
pub fn write(benchmark: &str, measures: &[Measure]) {
    let Some(file) = std::env::var_os(FILE) else {
        return;
    };
    let mut each = BTreeMap::new();
    for measure in measures {
        let percent = tolerance(measure.name);
        let lower = u128::from(measure.value) * 100 / u128::from(100 + percent);
        let lower = u64::try_from(lower).expect("a lower value that fits a JSON number");
        drop(each.insert(
            measure.name,
            serde_json::json!({
                "value": measure.value,
                "lower_value": lower,
                "upper_value": measure.value,
            }),
        ));
    }
    std::fs::write(&file, serde_json::json!({ benchmark: each }).to_string())
        .expect("the document is written to the file the tool named");
}
