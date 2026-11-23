//! Wire rows for the shipping comparison and project replay receipts.
use super::finish::json_output::{self as wire, Cell as WireCell};
use ryo::linter::{Cell, Finding, Slot};
use serde_json::json;
fn value(slot: &Slot) -> WireCell<'_> {
    match slot {
        Slot::Nothing => WireCell::Absent,
        Slot::Is(Cell::File(file)) => WireCell::File { file: &file.0 },
        Slot::Is(Cell::Text(text)) => WireCell::Text(text.0.as_ref().into()),
        Slot::Is(Cell::Number(number)) => WireCell::Number(number.0),
        Slot::Is(Cell::Span(span)) => WireCell::Span {
            span: [span.start.0 as u64, span.end.0 as u64],
        },
        other => panic!("unrepresented result cell {other:?}"),
    }
}

fn finding(finding: &Finding) -> String {
    let columns: Vec<_> = finding
        .columns
        .iter()
        .map(|col| (&*col.name.0, value(&col.value)))
        .collect();
    let keys: Vec<_> = finding.key.0.iter().map(value).collect();
    wire::query_row(
        &columns,
        &keys.iter().collect::<Vec<_>>(),
        &finding.rule.to_string(),
        &value(&finding.subject),
    )
    .expect("shipping query row")
}

pub(super) fn receipt(id: &str, result: Result<wire::Records, String>) -> String {
    use sha2::{Digest as _, Sha256};
    match result {
        Ok(rows) => {
            let (count, stream) = rows.wire();
            let digest = Sha256::digest(stream.as_bytes());
            json!({"id": id, "rows": count, "sha256": format!("{digest:x}")})
        }
        Err(reason) => json!({"id": id, "refused": reason}),
    }
    .to_string()
}

pub(super) fn write_findings(findings: &[Finding], path: &std::path::Path) {
    let mut rows: Vec<_> = findings.iter().map(finding).collect();
    rows.sort_unstable();
    std::fs::write(path, rows.join("\n") + "\n").expect("write SDK rows");
}
