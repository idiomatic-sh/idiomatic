//! The support of the `direct` cases.
use super::super::super::super::*;
use super::super::*;
use crate::application::file_dag::program::facade_runtime::file_execution::service_test::fixtures::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::table_model::types::RepresentationKind;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Integer, Relation, Value as Cell},
};
use ryo_relations::domain::relations::{service::to_bits, types::FactColumn};
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::{DecodeMode, WordSlice};
use ryo_values::domain::{
    plan_parts::types::{Side, StepId, StepScope},
    plans::types::{Plan, PlanStep},
    rows::types::{RowCount, Table},
};
use std::sync::Arc;

pub(crate) fn encoded(
    relation: Relation,
    table: Table,
    representation: RepresentationKind,
) -> Provider<'static> {
    assert_eq!(table.kinds, kinds(relation));
    let columns = (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .chunks_exact(table.kinds.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect();
    let compact = if representation == RepresentationKind::Compact {
        let mut words = ryo_relations::domain::relations::types::Words::default();
        encode_table(&table, &[], &mut words);
        let ryo_values::domain::table_words::types::Admitted {
            table: compact,
            names,
        } = ryo_values::domain::compact_table::service::admit_compact(
            ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(pack(
                WordSlice(&words.0),
            )))),
            &mut vec![],
            DecodeMode::Complete,
        )
        .unwrap();
        assert!(names.is_empty());
        Some(compact)
    } else {
        None
    };
    Provider {
        relation,
        table,
        columns,
        compact,
        tree: None,
        representation,
    }
}

pub(crate) fn schema_column(relation: Relation, kind: CellKind, ordinal: usize) -> u32 {
    kinds(relation)
        .iter()
        .enumerate()
        .filter(|(_, found)| **found == kind)
        .nth(ordinal)
        .unwrap()
        .0 as u32
}

pub(crate) fn table(relation: Relation, rows: &[Vec<(u32, Cell)>]) -> Table {
    Table {
        kinds: kinds(relation).to_vec(),
        count: RowCount(rows.len()),
        cells: rows
            .iter()
            .enumerate()
            .flat_map(|(at, values)| {
                let mut row = kinds(relation)
                    .iter()
                    .copied()
                    .map(|kind| defaults(kind, at))
                    .collect::<Vec<_>>();
                for (column, value) in values {
                    row[*column as usize] = *value;
                }
                row
            })
            .collect(),
    }
}

pub(crate) fn request(left: &[u32], right: &[u32], output: &[(Side, u32)]) -> Plan {
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: operation(left, right, output, vec![], &[]),
    }];
    request.emit.step = StepId(0);
    request
}

pub(crate) fn simple(
    left: &[i64],
    right: &[(i64, i64)],
    representation: RepresentationKind,
) -> [Provider<'static>; 2] {
    let n = |n| Cell::Number(Integer(n));
    let l = schema_column(Relation::Entry, CellKind::Number, 0);
    let r = schema_column(Relation::SequenceElement, CellKind::Number, 0);
    let payload = schema_column(Relation::SequenceElement, CellKind::Number, 1);
    [
        encoded(
            Relation::Entry,
            table(
                Relation::Entry,
                &left.iter().map(|n0| vec![(l, n(*n0))]).collect::<Vec<_>>(),
            ),
            RepresentationKind::Computed,
        ),
        encoded(
            Relation::SequenceElement,
            table(
                Relation::SequenceElement,
                &right
                    .iter()
                    .map(|(key, value)| vec![(r, n(*key)), (payload, n(*value))])
                    .collect::<Vec<_>>(),
            ),
            representation,
        ),
    ]
}
