//! The ratchet of what ryo costs, read from `tests/fixtures/ratchet.toml`, and the owner's budget in the same file, which a run never
//! changes. A run is judged against the ratchet: over its tolerance is a regression, under it is a win that the ratchet must follow.
//! The gap to the budget is always reported.

use std::path::Path;

/// The parsed `ratchet.toml`.
fn file() -> toml::Table {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ratchet.toml");
    std::fs::read_to_string(&path)
        .expect("the ratchet is read")
        .parse()
        .expect("the ratchet is TOML")
}

/// A whole number of `table`, under `key`.
fn number(table: &toml::Value, key: &str, what: &str) -> u128 {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u128::try_from(value).ok())
        .unwrap_or_else(|| panic!("`{key}` of {what} in the ratchet is a whole number"))
}

/// A number of the owner's budget (the `[budget]` table).
///
/// # Panics
///
/// When the budget or the key is missing.
#[must_use]
pub(crate) fn budget_number(key: &str) -> u128 {
    let table = file();
    let budget = table.get("budget").expect("the ratchet holds the budget");
    number(budget, key, "the budget")
}

/// Judges `measured` against the ratchet of `metric` in the table of `workload`, and reports the gap to `budget`, the owner's number for
/// the metric when it has one: the line to print when the run holds, the message of the failure when it does not.
///
/// # Errors
///
/// When the run is over the tolerance of the ratchet (a regression), or under it (the ratchet must be lowered).
///
/// # Panics
///
/// When the ratchet has no such metric.
pub(crate) fn judge(
    workload: &str,
    metric: &str,
    measured: u128,
    budget: Option<u128>,
) -> Result<String, String> {
    let table = file();
    let entry = table
        .get(workload)
        .and_then(|each| each.get(metric))
        .unwrap_or_else(|| panic!("`{workload}.{metric}` in the ratchet"));
    let what = format!("`{workload}.{metric}`");
    let (best, tolerance) = (
        number(entry, "best", &what),
        number(entry, "tolerance-percent", &what),
    );
    let gap = match budget {
        Some(target) => format!(
            "the budget {target}: {} over it",
            measured.saturating_sub(target)
        ),
        None => "no budget set".to_owned(),
    };
    let line = format!("{measured} against the ratchet {best} and {gap}");
    if measured * 100 > best * (100 + tolerance) {
        return Err(format!(
            "{workload}.{metric} regressed: {line}; more than {tolerance}% over the ratchet"
        ));
    }
    if measured * (100 + tolerance) < best * 100 {
        return Err(format!(
            "{workload}.{metric} beats the ratchet by more than {tolerance}%: {line}; lower `best` of `{workload}.{metric}` in tests/fixtures/ratchet.toml to {measured}"
        ));
    }
    Ok(format!("{workload}.{metric} holds: {line}"))
}

/// Prints the verdicts of a run and fails with the ones that did not hold.
///
/// # Panics
///
/// When a verdict is a failure.
pub(crate) fn hold(verdicts: &[Result<String, String>]) {
    for verdict in verdicts {
        match verdict {
            Ok(line) | Err(line) => eprintln!("ratchet: {line}"),
        }
    }
    let failed: Vec<&String> = verdicts
        .iter()
        .filter_map(|each| each.as_ref().err())
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
}
