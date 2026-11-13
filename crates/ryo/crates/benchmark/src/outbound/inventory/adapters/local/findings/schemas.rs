//! The expected findings of a case, as the authoring normalizes and orders them.
use super::super::canonical::schemas::{Canonical, invalid, table, text};
use super::super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// One expected finding as the authoring normalizes it, with its rule defaulting to the case's owner.
pub(in super::super) fn finding(row: &Canonical, owner: &str) -> Result<Canonical, Error> {
    let row = row
        .as_table()
        .ok_or_else(|| invalid(constants::EXPECTED_ROW))?;
    let field = |key: &str| {
        row.get(key)
            .cloned()
            .ok_or_else(|| invalid(constants::EXPECTED_ROW))
    };
    Ok(table([
        (
            constants::RULE,
            row.get(constants::RULE)
                .cloned()
                .unwrap_or_else(|| text(owner)),
        ),
        (constants::FILE, field(constants::FILE)?),
        (constants::START, field(constants::START)?),
        (constants::END, field(constants::END)?),
    ]))
}

/// The order expected findings are written in: rule, file, start, end.
fn finding_key(row: &Canonical) -> (String, String, i64, i64) {
    let field = |key| row.as_table().and_then(|row| row.get(key));
    (
        field(constants::RULE)
            .and_then(Canonical::as_text)
            .unwrap_or_default()
            .to_owned(),
        field(constants::FILE)
            .and_then(Canonical::as_text)
            .unwrap_or_default()
            .to_owned(),
        field(constants::START)
            .and_then(Canonical::as_integer)
            .unwrap_or_default(),
        field(constants::END)
            .and_then(Canonical::as_integer)
            .unwrap_or_default(),
    )
}

/// Findings sorted as the authoring writes them.
pub(in super::super) fn sorted_findings(mut rows: Vec<Canonical>) -> Canonical {
    rows.sort_by_key(finding_key);
    Canonical::List(rows)
}
