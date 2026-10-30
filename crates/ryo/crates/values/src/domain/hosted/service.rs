//! The `hosted` service: free functions over the relations the host declares, with no IO and no ports.

use super::types::{ColumnLabel, HostKind, HostRelation, HostedColumn, HostedName, HostedRelation};
use ryo_language::domain::relations::types::CellKind;

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
