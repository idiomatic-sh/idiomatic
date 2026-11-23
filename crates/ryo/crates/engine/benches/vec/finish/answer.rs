//! The typed answer held by the caller. Only the reference-report writer renders cells.
use super::{NULL, Run, Table, Ty};
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::row_texts::types::Cell;
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Row {
    pub rule: Box<str>,
    pub columns: Vec<(Box<str>, Option<Cell>)>,
    /// The subject is first, followed by the ordered key columns.
    pub identity: Vec<usize>,
}

pub(crate) fn typed(
    run: &Run,
    table: &Table,
    row: usize,
    rule: &str,
    file: Option<usize>,
    identity: &[usize],
) -> Result<Row, String> {
    let mut columns = Vec::with_capacity(table.names.len());
    for (index, name) in table.names.iter().enumerate() {
        let word = table.cols[index][row];
        let cell = if word == NULL {
            None
        } else {
            match table.tys[index] {
                Ty::Node => {
                    let (id, at) = match file {
                        Some(id) => (id, word),
                        None => ((word >> 32) as usize, word & u64::from(u32::MAX)),
                    };
                    let path = run.paths.get(id).ok_or("node has no source origin")?;
                    Some(Cell::Text(format!("{path}#{at}").into()))
                }
                Ty::File => {
                    let path = run
                        .paths
                        .get(usize::try_from(word).map_err(|e| e.to_string())?)
                        .ok_or_else(|| format!("file outside the run: {word}"))?;
                    Some(Cell::File(path.as_str().into()))
                }
                Ty::Sym if run.absent_word == Some(word as u32) => None,
                Ty::Sym => Some(Cell::Text(
                    std::str::from_utf8(run.text(word as u32))
                        .map_err(|e| e.to_string())?
                        .into(),
                )),
                Ty::Span => Some(Cell::Span(Span {
                    start: Offset((word >> 32) as u32),
                    end: Offset(word as u32),
                })),
                Ty::Num => Some(Cell::Number(Integer(word as i64))),
                Ty::Flag => Some(Cell::Text(
                    (if word == 0 { "false" } else { "true" }).into(),
                )),
            }
        };
        columns.push((name.as_str().into(), cell));
    }
    Ok(Row {
        rule: rule.into(),
        columns,
        identity: identity.to_vec(),
    })
}

/// The release benchmark's canonical final finding, including the declared column order.
pub(crate) fn finding(row: &Row) -> String {
    let columns: Vec<_> = row
        .columns
        .iter()
        .map(|(name, cell)| {
            let value = match cell {
                None => Value::Null,
                Some(Cell::Text(text)) => json!(text),
                Some(Cell::File(path)) => json!({"file":path}),
                Some(Cell::Number(n)) => json!(n.0),
                Some(Cell::Span(s)) => json!({"span":[s.start.0,s.end.0]}),
            };
            (name, value)
        })
        .collect();
    let key: Vec<_> = row.identity[1..].iter().map(|&i| &columns[i].1).collect();
    json!({"columns":columns,"key":key,"rule":&row.rule,"subject":&columns[row.identity[0]].1})
        .to_string()
}

pub(in super::super) fn row(
    run: &Run,
    table: &Table,
    row: usize,
    rule: &str,
    file: Option<usize>,
    identity: &[usize],
) -> Result<String, String> {
    typed(run, table, row, rule, file, identity).map(|r| finding(&r))
}
