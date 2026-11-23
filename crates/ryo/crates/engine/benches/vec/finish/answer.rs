//! Full emitted rows for comparing the vector benchmark to a shipping query.
use super::{NULL, Run, Table, Ty};
use serde_json::{Value, json};

pub(in super::super) fn row(
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
            Value::Null
        } else {
            match table.tys[index] {
                Ty::Node => {
                    let path = file.and_then(|id| run.paths.get(id)).ok_or("node has no source origin")?;
                    json!(format!("{path}#{word}"))
                }
                Ty::File => {
                    json!({"file": run.paths.get(usize::try_from(word).map_err(|e| e.to_string())?)
                    .ok_or_else(|| format!("file outside the run: {word}"))?})
                }
                Ty::Symbol if run.absent_word == Some(word as u32) => Value::Null,
                Ty::Symbol => {
                    json!(std::str::from_utf8(run.text(word as u32)).map_err(|e| e.to_string())?)
                }
                Ty::Span => {
                    let start = (word >> 32) as u32;
                    let end = word as u32;
                    json!({"span": [start, end]})
                }
                Ty::Number => json!(word as i64),
                Ty::Flag => json!(if word == 0 { "false" } else { "true" }),
            }
        };
        columns.push((name, value));
    }
    let key: Vec<_> = identity[1..]
        .iter()
        .map(|&index| &columns[index].1)
        .collect();
    Ok(
        json!({"columns": columns, "key": key, "rule": rule, "subject": &columns[identity[0]].1})
            .to_string(),
    )
}
