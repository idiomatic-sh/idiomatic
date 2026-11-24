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

pub(super) fn project_receipt(id: &str, result: Result<(usize, String), String>) -> String {
    match result {
        Ok((rows, digest)) => json!({"id":id,"rows":rows,"sha256":digest}),
        Err(reason) => json!({"id":id,"refused":reason}),
    }
    .to_string()
}
pub(super) fn packet_receipt(
    id: &str,
    result: Result<(usize, String, Vec<&(String, String)>), &str>,
) -> String {
    match result {
        Ok((rows, digest, unknowns)) => {
            json!({"id":id,"rows":rows,"sha256":digest,"unknowns":unknowns})
        }
        Err(reason) => json!({"id":id,"refused":reason}),
    }
    .to_string()
}
#[derive(Default)]
pub(super) struct UnknownWriter<'a> {
    encoded: std::collections::HashMap<&'a (String, String), Vec<u8>>,
}

impl<'a> UnknownWriter<'a> {
    pub(super) fn len(&self) -> usize {
        self.encoded.len()
    }

    pub(super) fn write(
        &mut self,
        mut out: impl std::io::Write,
        unknowns: &[&'a (String, String)],
    ) {
        out.write_all(b"[").expect("write unknown findings");
        for (i, &unknown) in unknowns.iter().enumerate() {
            if i > 0 {
                out.write_all(b",").expect("write unknown findings");
            }
            let bytes = self.encoded.entry(unknown).or_insert_with(|| {
                serde_json::to_vec(unknown).expect("encode unknown finding")
            });
            out.write_all(bytes).expect("write unknown findings");
        }
        out.write_all(b"]").expect("write unknown findings");
    }
}
pub(super) fn write_findings(findings: &[Finding], path: &std::path::Path) {
    let mut rows: Vec<_> = findings.iter().map(finding).collect();
    rows.sort_unstable();
    std::fs::write(path, rows.join("\n") + "\n").expect("write SDK rows");
}
