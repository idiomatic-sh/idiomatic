//! Semantic facts from the independently executed reference, compared with the supported relation rows.

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "Only the listed independently observable relations have reference facts; unsupported cells are rejected"
)]

use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows};

use super::walk::{Acceptance, acceptance, block};

/// Encode the observable part of a row in the reference dump's interchange format.
fn encoded(cell: &Cell) -> String {
    match cell {
        Cell::Name(text) => text.bytes().fold(String::from("x"), |mut hex, byte| {
            for digit in [byte >> 4_u8, byte & 0x0f_u8] {
                hex.extend(char::from_digit(u32::from(digit), 16));
            }
            hex
        }),
        Cell::Unknown(_) => String::from("?"),
        Cell::Span(start, end) => format!(
            "{}:{} {}:{}",
            start.line, start.column, end.line, end.column
        ),
        Cell::Number(number) => number.to_string(),
        // The reference has no such cell, so a projection that selects one shows as a difference.
        Cell::Unlocated | Cell::Node(_) | Cell::File(_) | Cell::Flag(_) => {
            String::from("<a cell the reference does not hold>")
        }
    }
}

/// The reference fact vocabulary and row columns to compare, excluding engine-assigned identities.
fn projection(relation: Relation) -> Option<(&'static str, &'static [usize])> {
    match relation {
        Relation::Comment => Some(("comment", &[3, 1])),
        Relation::StringLiteral => Some(("string", &[4, 2])),
        Relation::Function => Some(("function", &[3, 1])),
        Relation::StaticImport => Some(("import", &[5, 2, 3])),
        Relation::Parameter => Some(("parameter", &[7, 1, 4, 3])),
        Relation::StaticCall => Some(("call", &[4, 2])),
        _ => None,
    }
}

/// Compare every supported fact relation with the independent reference, including missing and excess rows.
#[must_use]
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    match acceptance(text) {
        Acceptance::Unknown => {
            return vec![String::from(
                "the independent reference has no pinned evidence for these bytes",
            )];
        }
        Acceptance::Refused => return Vec::new(),
        Acceptance::Accepted => {}
    }
    let records = block(text);
    let mut differences = Vec::new();
    for relation in &rows.0 {
        let Some((tag, columns)) = projection(relation.relation) else {
            continue;
        };
        let prefix = format!("{tag} ");
        let mut expected: Vec<_> = records
            .iter()
            .filter(|line| line.starts_with(&prefix))
            .map(|line| (*line).to_owned())
            .collect();
        let mut actual: Vec<_> = relation
            .rows
            .iter()
            .map(|row| {
                let fields: Vec<_> = columns
                    .iter()
                    .map(|column| match row.get(*column) {
                        Some(Cell::Name(name)) if tag == "parameter" && *column == 1 => {
                            name.clone()
                        }
                        Some(value) => encoded(value),
                        None => String::from("<a column the row does not hold>"),
                    })
                    .collect();
                format!("{tag} {}", fields.join(" "))
            })
            .collect();
        expected.sort();
        actual.sort();
        if actual != expected {
            differences.push(format!("{tag}: reference has {} facts, rows have {}; first only in reference {:?}; first only in rows {:?}", expected.len(), actual.len(), expected.iter().find(|fact| !actual.contains(fact)), actual.iter().find(|fact| !expected.contains(fact))));
        }
    }
    differences
}
