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
    fn tally<I: IntoIterator>(out: &mut super::types::HostIdentity, items: I)
    where
        I::IntoIter: ExactSizeIterator,
    {
        out.0.extend_from_slice(
            &u64::try_from(items.into_iter().len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
    }
    fn text(out: &mut super::types::HostIdentity, value: super::types::HostIdentityText<'_>) {
        tally(out, value.0);
        out.0.extend_from_slice(value.0);
    }
    let mut out = super::types::HostIdentity(super::constants::IDENTITY_TAG.to_vec());
    tally(&mut out, &relations);
    for relation in relations {
        text(
            &mut out,
            super::types::HostIdentityText(relation.name.0.as_bytes()),
        );
        tally(&mut out, &relation.columns);
        for column in &relation.columns {
            text(
                &mut out,
                super::types::HostIdentityText(column.name.0.as_bytes()),
            );
            out.0.push(match column.kind {
                HostKind::File => 0,
                HostKind::Text => 1,
                HostKind::Number => 2,
            });
        }
        tally(&mut out, &relation.rows);
        for row in &relation.rows {
            tally(&mut out, &row.0);
            for cell in &row.0 {
                match cell {
                    Cell::File(path) => {
                        out.0.push(0);
                        text(&mut out, super::types::HostIdentityText(path.as_bytes()));
                    }
                    Cell::Text(value) => {
                        out.0.push(1);
                        text(&mut out, super::types::HostIdentityText(value.as_bytes()));
                    }
                    Cell::Number(value) => {
                        out.0.push(2);
                        out.0.extend_from_slice(&value.0.to_le_bytes());
                    }
                    Cell::Span(value) => {
                        out.0.push(3);
                        out.0.extend_from_slice(&value.start.0.to_le_bytes());
                        out.0.extend_from_slice(&value.end.0.to_le_bytes());
                    }
                }
            }
        }
    }
    out
}

use crate::domain::records::types::Cell;
