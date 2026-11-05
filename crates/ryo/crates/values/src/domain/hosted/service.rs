//! The `hosted` service: free functions over the relations the host declares, with no IO and no ports.

use super::types::{ColumnLabel, HostKind, HostRelation, HostedColumn, HostedName, HostedRelation};
use ryo_relations::domain::relations::types::CellKind;

/// What a kind of column the host declares holds in the tables of the engine.
#[must_use]
pub fn kind_of(kind: HostKind) -> CellKind {
    match kind {
        HostKind::File => CellKind::File,
        HostKind::Text => CellKind::Symbol,
        HostKind::Number => CellKind::Number,
    }
}

/// The relations as the compiler is told of them: the name a query writes and the columns, in the order the host gave them.
#[must_use]
pub fn declarations(relations: &[HostRelation]) -> Vec<HostedRelation> {
    relations
        .iter()
        .map(|relation| HostedRelation {
            name: HostedName(relation.name.0.clone()),
            columns: relation
                .columns
                .iter()
                .map(|column| HostedColumn {
                    name: ColumnLabel(column.name.0.clone()),
                    kind: kind_of(column.kind),
                })
                .collect(),
        })
        .collect()
}

/// Versioned typed identity of hosted facts. Presentation lines are deliberately not an encoding:
/// arbitrary text may itself contain tabs, newlines or the prefixes printed by `lines`.
#[must_use]
pub fn identity(relations: &[HostRelation]) -> super::types::HostIdentity {
    use crate::domain::hosted::types::HostKind;
    fn count(out: &mut Vec<u8>, value: usize) {
        out.extend_from_slice(&u64::try_from(value).unwrap_or(u64::MAX).to_le_bytes());
    }
    fn text(out: &mut Vec<u8>, value: &str) {
        count(out, value.len());
        out.extend_from_slice(value.as_bytes());
    }
    let mut out = b"ryo-hosted-facts-v1".to_vec();
    count(&mut out, relations.len());
    for relation in relations {
        text(&mut out, &relation.name.0);
        count(&mut out, relation.columns.len());
        for column in &relation.columns {
            text(&mut out, &column.name.0);
            out.push(match column.kind {
                HostKind::File => 0,
                HostKind::Text => 1,
                HostKind::Number => 2,
            });
        }
        count(&mut out, relation.rows.len());
        for row in &relation.rows {
            count(&mut out, row.0.len());
            for cell in &row.0 {
                match cell {
                    Cell::File(path) => {
                        out.push(0);
                        text(&mut out, path);
                    }
                    Cell::Text(value) => {
                        out.push(1);
                        text(&mut out, value);
                    }
                    Cell::Number(value) => {
                        out.push(2);
                        out.extend_from_slice(&value.0.to_le_bytes());
                    }
                    Cell::Span(value) => {
                        out.push(3);
                        out.extend_from_slice(&value.start.0.to_le_bytes());
                        out.extend_from_slice(&value.end.0.to_le_bytes());
                    }
                }
            }
        }
    }
    super::types::HostIdentity(out)
}

use crate::domain::records::types::Cell;
