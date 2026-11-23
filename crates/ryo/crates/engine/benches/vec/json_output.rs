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
    query_rows: Vec<(usize, usize, String)>,
}
struct Location {
    source: usize,
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
    pub(crate) fn merge(&mut self, mut other: Self) {
        self.locations.append(&mut other.locations);
        self.query_rows.append(&mut other.query_rows);
    }

    pub(crate) fn query(&mut self, run: &Run, table: &Table, index: usize, emit: usize, rule: &str, source: usize, identity: &[usize]) -> Result<(), String> {
        self.query_rows.push((source, emit, row(run, table, index, rule, Some(source), identity)?));
        Ok(())
    }

    pub(crate) fn location(&mut self, source: usize, rule: usize, file: usize, start: u64, end: u64) {
        self.locations.push(Location {
            source,
            rule,
            file,
            start,
            end,
        });
    }

    pub(crate) fn finish(
        self,
        plan: &super::Plan,
        run: &Run,
        out: Option<&str>,
        projection: Projection<'_>,
        cutoff: &[usize],
    ) -> usize {
        let mut records = Records {
            lines: self.query_rows.into_iter().filter_map(|(source, rule, wire)|
                (source <= cutoff[rule]).then_some(wire)).collect(),
        };
        if !self.locations.is_empty() {
            let mut rules = vec![None; plan.emits.len()];
            let mut files = vec![None; run.paths.len()];
            records.lines.reserve(self.locations.len());
            for Location {
                source,
                rule,
                file,
                start,
                end,
            } in self.locations
            {
                if source > cutoff[rule] { continue; }
                let rule = rules[rule].get_or_insert_with(|| quoted(&plan.emits[rule].0));
                let file = files[file].get_or_insert_with(|| quoted(&run.paths[file]));
                records.lines.push(format!(
                    r#"{{"rule":{rule},"file":{file},"start":{start},"end":{end}}}"#
                ));
            }
        }
        if out.is_some() || matches!(projection, Projection::DistinctLocations) {
            records.lines.sort_unstable();
        }
        if matches!(projection, Projection::DistinctLocations) {
            records.lines.dedup();
        }
        let count = records.lines.len();
        if let Projection::Collect(rows) = projection {
            rows.lines.append(&mut records.lines);
        } else if let Some(path) = out {
            std::fs::write(path, records.lines.join("\n") + "\n").expect("write findings");
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
