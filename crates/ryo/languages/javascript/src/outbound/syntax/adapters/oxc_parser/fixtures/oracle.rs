//! Semantic facts independently pinned from Acorn, compared without engine-assigned identities.
#![expect(
    clippy::expect_used,
    reason = "malformed or missing independent reference evidence must fail the fixture rather than silently passing"
)]
use super::walk::records;
use ryo::languages::{
    Relation,
    contract::{Cell, OracleRows},
};

/// Stable textual representation of the observable cell value.
fn value(cell: &Cell) -> String {
    match cell {
        Cell::Name(text) => {
            use std::fmt::Write as _;
            text.bytes().fold(String::from("x"), |mut encoded, byte| {
                write!(&mut encoded, "{byte:02x}").expect("cannot encode reference fact");
                encoded
            })
        }
        Cell::Unknown(_) => String::from("?"),
        Cell::Span(start, end) => format!(
            "{}:{} {}:{}",
            start.line, start.column, end.line, end.column
        ),
        Cell::Number(number) => number.to_string(),
        // The reference has no identity columns, so a projection that selects one shows as a difference.
        Cell::Unlocated | Cell::Node(_) | Cell::File(_) | Cell::Flag(_) => {
            String::from("<an identity column the reference does not hold>")
        }
    }
}
/// Compare every supported semantic relation, including excess or missing facts.
#[must_use]
pub(crate) fn oracle(source: &str, rows: &OracleRows) -> Vec<String> {
    let records = records(source);
    if records
        .first()
        .is_some_and(|line| line.ends_with(" refused"))
    {
        return Vec::new();
    }
    let mut differences = Vec::new();
    for relation in &rows.0 {
        let projections: &[(Relation, &str, &[usize])] = &[
            (Relation::Comment, "comment", &[3, 1]),
            (Relation::StringLiteral, "string", &[4, 2]),
            (Relation::Function, "function", &[3, 1]),
            (Relation::Parameter, "parameter", &[7, 1, 4, 3]),
            (Relation::StaticCall, "call", &[4, 2]),
            (Relation::StaticImport, "import", &[5, 2, 3]),
        ];
        let Some((_, name, columns)) = projections
            .iter()
            .find(|(kind, _, _)| *kind == relation.relation)
        else {
            continue;
        };
        let prefix = format!("{name} ");
        let mut expected: Vec<_> = records
            .iter()
            .filter(|line| line.starts_with(&prefix))
            .map(|line| (*line).to_owned())
            .collect();
        let mut actual: Vec<_> = relation
            .rows
            .iter()
            .map(|row| {
                let cells: Vec<_> = columns
                    .iter()
                    .map(|column| match row.get(*column) {
                        Some(Cell::Name(kind)) if *name == "parameter" && *column == 1 => {
                            kind.clone()
                        }
                        Some(cell) => value(cell),
                        None => String::from("<a column the row does not hold>"),
                    })
                    .collect();
                format!("{name} {}", cells.join(" "))
            })
            .collect();
        expected.sort();
        actual.sort();
        if expected != actual {
            differences.push(format!(
                "{name}: reference-only {:?}; runtime-only {:?}",
                expected.iter().find(|item| !actual.contains(item)),
                actual.iter().find(|item| !expected.contains(item))
            ));
        }
    }
    differences
}
