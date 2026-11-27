//! Encode final findings, full query rows and diagnostic records.
use super::{NULL, Run, Table, Ty};
use serde::Serialize;
use std::borrow::Cow;

#[derive(Serialize)]
#[serde(untagged)]
pub enum Cell<'a> {
    Absent,
    Text(Cow<'a, str>),
    Number(i64),
    File { file: &'a str },
    Span { span: [u64; 2] },
}




/// Completed output records. Readers can contribute rows without carrying a wire format.
#[derive(Default)]
pub struct Records {
    pub lines: Vec<String>,
}

impl Records {


}


/// The cold-list/profile TSV embeds a complete node using the public plan wire schema.
pub fn step(step: &ryo_values::domain::finish_plans::types::FinishStep) -> String {
    use ryo_engine::outbound::plan_codec::{factory, interface::PlanCodecPort as _};
    let mut bytes = Vec::new();
    factory::json()
        .write_step(step, &mut bytes)
        .expect("encode diagnostic step");
    String::from_utf8(bytes).expect("text plan format")
}

use super::answer::{Row, Value};
use ryo_values::domain::row_texts::types::Cell as AnswerCell;
// These wrappers serialize directly into the final command output. Subject and
// key columns are borrowed from their declared slots, including null values.
pub struct Published(Value);
impl Serialize for Published {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match &self.0 {
            None => out.serialize_none(),
            Some(AnswerCell::Text(text)) => out.serialize_str(text),
            Some(AnswerCell::Number(n)) => out.serialize_i64(n.0),
            Some(AnswerCell::File(path)) => {
                let mut entry = out.serialize_map(Some(1))?;
                entry.serialize_entry("file", path.as_ref())?;
                entry.end()
            }
            Some(AnswerCell::Span(span)) => {
                let mut entry = out.serialize_map(Some(1))?;
                entry.serialize_entry("span", &[span.start.0, span.end.0])?;
                entry.end()
            }
        }
    }
}

pub struct Columns<'a>(&'a Row);
impl Serialize for Columns<'_> {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.collect_seq(
            self.0
                .layout
                .names
                .iter()
                .zip(self.0.cells().map(Published)),
        )
    }
}

pub struct Keys<'a>(&'a Row);
impl Serialize for Keys<'_> {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.collect_seq(
            self.0.layout.identity[1..]
                .iter()
                .map(|&i| Published(self.0.value(i))),
        )
    }
}

/// Canonical command output, with the release's column order and escaping.
pub fn write_finding(dst: &mut Vec<u8>, row: &Row) {
    #[derive(Serialize)]
    struct Finding<'a> {
        columns: Columns<'a>,
        key: Keys<'a>,
        rule: &'a str,
        subject: Published,
    }
    serde_json::to_writer(
        dst,
        &Finding {
            columns: Columns(row),
            key: Keys(row),
            rule: &row.layout.rule,
            subject: Published(row.value(row.layout.identity[0])),
        },
    )
    .expect("serialize finding");
}

/// Canonical ordering is lexical over the complete output bytes, including escapes.
pub fn ordered_rows<'a>(
    rows: impl IntoIterator<Item = &'a Row>,
    mut emit: impl FnMut(&[u8]),
) {
    let mut bytes = Vec::new();
    let mut ranges = Vec::new();
    for row in rows {
        let start = bytes.len();
        write_finding(&mut bytes, row);
        ranges.push(start..bytes.len());
    }
    ranges.sort_unstable_by(|a, b| bytes[a.clone()].cmp(&bytes[b.clone()]));
    for range in ranges {
        emit(&bytes[range]);
    }
}
