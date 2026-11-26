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

pub fn query_row(
    columns: &[(&str, Cell<'_>)],
    key: &[&Cell<'_>],
    rule: &str,
    subject: &Cell<'_>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Row<'a> {
        columns: &'a [(&'a str, Cell<'a>)],
        key: &'a [&'a Cell<'a>],
        rule: &'a str,
        subject: &'a Cell<'a>,
    }
    serde_json::to_string(&Row {
        columns,
        key,
        rule,
        subject,
    })
    .map_err(|e| e.to_string())
}

pub fn row(
    run: &Run,
    table: &Table,
    row: usize,
    rule: &str,
    file: Option<usize>,
    identity: &[usize],
) -> Result<String, String> {
    let mut columns = Vec::with_capacity(table.names.len());
    for (index, name) in table.names.iter().enumerate() {
        let word = table.cols[index][row];
        let value = if word == NULL {
            Cell::Absent
        } else {
            match table.tys[index] {
                Ty::Node => {
                    let path = file
                        .and_then(|id| run.paths.get(id))
                        .ok_or("node has no source origin")?;
                    Cell::Text(format!("{path}#{word}").into())
                }
                Ty::File => Cell::File {
                    file: run
                        .paths
                        .get(usize::try_from(word).map_err(|e| e.to_string())?)
                        .ok_or_else(|| format!("file outside the run: {word}"))?,
                },
                Ty::Sym if run.absent_word == Some(word as u32) => Cell::Absent,
                Ty::Sym => Cell::Text(
                    std::str::from_utf8(run.text(word as u32))
                        .map_err(|e| e.to_string())?
                        .into(),
                ),
                Ty::Span => Cell::Span {
                    span: [word >> 32, word & 0xffff_ffff],
                },
                Ty::Num => Cell::Number(word as i64),
                Ty::Flag => Cell::Text((if word == 0 { "false" } else { "true" }).into()),
            }
        };
        columns.push((name.as_str(), value));
    }
    let key: Vec<_> = identity[1..]
        .iter()
        .map(|&index| &columns[index].1)
        .collect();
    query_row(&columns, &key, rule, &columns[identity[0]].1)
}

pub fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("encode output string")
}
pub fn buckets(keys: &[Vec<String>]) -> String {
    serde_json::to_string(keys).expect("encode bucket names")
}

/// Completed output records. Readers can contribute rows without carrying a wire format.
#[derive(Default)]
pub struct Records {
    pub lines: Vec<String>,
}

impl Records {
    pub fn query(
        &mut self,
        run: &Run,
        table: &Table,
        index: usize,
        rule: &str,
        file: Option<usize>,
        identity: &[usize],
    ) -> Result<(), String> {
        self.lines
            .push(row(run, table, index, rule, file, identity)?);
        Ok(())
    }

    pub fn wire(mut self) -> (usize, String) {
        self.lines.sort_unstable();
        (self.lines.len(), self.lines.join("\n") + "\n")
    }

    pub fn write(self, path: &str) {
        std::fs::write(path, self.wire().1).expect("write findings");
    }
}

/// Lint diagnostics use byte offsets. Query comparisons carry the complete output record.
pub fn snapshot(plan: &ryo_values::domain::finish_plans::types::FinishPlan) -> Vec<u8> {
    use ryo_engine::outbound::plan_codec::{factory, interface::PlanCodecPort as _};
    factory::json().encoded(plan)
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
