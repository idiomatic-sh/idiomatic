//! Encode final findings, full query rows and diagnostic records.
use super::{NULL, Run, Table, Ty};
use serde::Serialize;
use std::borrow::Cow;

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Cell<'a> {
    Absent,
    Text(Cow<'a, str>),
    Number(i64),
    File { file: &'a str },
    Span { span: [u64; 2] },
}

pub(crate) fn query_row(
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

fn row(
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

pub(crate) fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("encode output string")
}
pub(crate) fn buckets(keys: &[Vec<String>]) -> String {
    serde_json::to_string(keys).expect("encode bucket names")
}

/// Completed output records. Readers can contribute rows without carrying a wire format.
#[derive(Default)]
pub(crate) struct Records {
    lines: Vec<String>,
}

impl Records {
    pub(crate) fn query(
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

    pub(crate) fn wire(mut self) -> (usize, String) {
        self.lines.sort_unstable();
        (self.lines.len(), self.lines.join("\n") + "\n")
    }

    pub(crate) fn write(self, path: &str) {
        std::fs::write(path, self.wire().1).expect("write findings");
    }
}

/// Lint diagnostics use byte offsets. Query comparisons carry the complete output record.
#[derive(Default)]
pub(crate) struct Findings {
    locations: Vec<Location>,
    pub(crate) rows: Records,
}
struct Location {
    rule: usize,
    file: usize,
    start: u64,
    end: u64,
}

pub(crate) enum Projection<'a> {
    All,
    DistinctLocations,
    Collect(&'a mut Records),
}
impl Findings {
    pub(crate) fn location(&mut self, rule: usize, file: usize, start: u64, end: u64) {
        self.locations.push(Location {
            rule,
            file,
            start,
            end,
        });
    }

    pub(crate) fn finish(
        mut self,
        plan: &super::Plan,
        run: &Run,
        out: Option<&str>,
        projection: Projection<'_>,
    ) -> usize {
        if !self.locations.is_empty() {
            let mut rules = vec![None; plan.emits.len()];
            let mut files = vec![None; run.paths.len()];
            self.rows.lines.reserve(self.locations.len());
            for Location {
                rule,
                file,
                start,
                end,
            } in self.locations
            {
                let rule = rules[rule].get_or_insert_with(|| quoted(&plan.emits[rule].0));
                let file = files[file].get_or_insert_with(|| quoted(&run.paths[file]));
                self.rows.lines.push(format!(
                    r#"{{"rule":{rule},"file":{file},"start":{start},"end":{end}}}"#
                ));
            }
        }
        if out.is_some() || matches!(projection, Projection::DistinctLocations) {
            self.rows.lines.sort_unstable();
        }
        if matches!(projection, Projection::DistinctLocations) {
            self.rows.lines.dedup();
        }
        let count = self.rows.lines.len();
        if let Projection::Collect(rows) = projection {
            rows.lines.append(&mut self.rows.lines);
        } else if let Some(path) = out {
            std::fs::write(path, self.rows.lines.join("\n") + "\n").expect("write findings");
        }
        count
    }
}

/// Bytes for an explicitly requested external plan snapshot.
pub(crate) fn snapshot(plan: &ryo_values::domain::finish_plans::types::FinishPlan) -> Vec<u8> {
    use ryo_engine::outbound::plan_codec::{factory, interface::PlanCodecPort as _};
    factory::json().encoded(plan)
}

/// The cold-list/profile TSV embeds a complete node using the public plan wire schema.
pub(crate) fn step(step: &ryo_values::domain::finish_plans::types::FinishStep) -> String {
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
struct Published(Value);
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

/// Only the command writer needs escaped column labels and the canonical cell ranges.
struct FindingWriter<'a> {
    layout: &'a super::answer::Layout,
    labels: Vec<Vec<u8>>,
    rule: Vec<u8>,
    values: Vec<std::ops::Range<usize>>,
}

impl<'a> FindingWriter<'a> {
    fn new(layout: &'a super::answer::Layout) -> Self {
        let labels = layout.names.iter().map(|name| {
            let mut label = vec![b'['];
            serde_json::to_writer(&mut label, name).expect("serialize column name");
            label.push(b',');
            label
        }).collect();
        Self {
            layout,
            labels,
            rule: serde_json::to_vec(&layout.rule).expect("serialize rule"),
            values: Vec::with_capacity(layout.names.len()),
        }
    }

    fn write(&mut self, dst: &mut Vec<u8>, row: &Row) {
        self.values.clear();
        dst.extend_from_slice(br#"{"columns":["#);
        for (at, (label, value)) in self.labels.iter().zip(row.cells()).enumerate() {
            if at != 0 { dst.push(b','); }
            dst.extend_from_slice(label);
            let start = dst.len();
            serde_json::to_writer(&mut *dst, &Published(value)).expect("serialize cell");
            self.values.push(start..dst.len());
            dst.push(b']');
        }
        dst.extend_from_slice(br#"],"key":["#);
        for (at, &index) in self.layout.identity[1..].iter().enumerate() {
            if at != 0 { dst.push(b','); }
            dst.extend_from_within(self.values[index].clone());
        }
        dst.extend_from_slice(br#"],"rule":"#);
        dst.extend_from_slice(&self.rule);
        dst.extend_from_slice(br#","subject":"#);
        dst.extend_from_within(self.values[self.layout.identity[0]].clone());
        dst.push(b'}');
    }
}

/// Canonical ordering is lexical over the complete output bytes, including escapes.
pub(crate) fn ordered_rows<'a>(
    rows: impl IntoIterator<Item = &'a Row>,
    mut emit: impl FnMut(&[u8]),
) {
    let mut bytes = Vec::new();
    let mut ranges = Vec::new();
    let mut rows = rows.into_iter();
    let Some(first) = rows.next() else { return };
    let mut writer = FindingWriter::new(&first.layout);
    for row in std::iter::once(first).chain(rows) {
        let layout = row.layout.as_ref();
        if !std::ptr::eq(writer.layout, layout) {
            writer = FindingWriter::new(layout);
        }
        let start = bytes.len();
        writer.write(&mut bytes, row);
        ranges.push(start..bytes.len());
    }
    ranges.sort_unstable_by(|a, b| bytes[a.clone()].cmp(&bytes[b.clone()]));
    for range in ranges {
        emit(&bytes[range]);
    }
}
