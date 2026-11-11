//! The support of the `direct` cases.
use super::super::*;
use crate::application::file_dag::program::facade_runtime::file_execution::service_test::fixtures::*;
use fixtures::execution::{Faults, Held, Observation, execute, real_port};
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationKind;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::{service::from_bits, types::Bits};
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
use std::borrow::Cow;
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
    let mut request = plan(PlanShape::ProjectRetry);
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

/// The keys a direct Join row joins `Entry` to `SequenceElement` on.
#[derive(Clone, Copy, Debug)]
pub(super) enum Keying {
    /// The number columns.
    Single,
    /// The flag then the number columns.
    Composite,
    /// The number then the flag columns.
    Reordered,
    /// No keys: every left row meets every right row.
    Empty,
    /// The left number against the right flag.
    Unlike,
    /// A left key column 99 the left side does not have.
    InvalidLeft,
    /// A right key column 99 the right side does not have.
    InvalidRight,
}

/// One run of the request over a supplied right-side representation.
#[derive(Debug, PartialEq)]
pub(super) struct Attempt {
    /// The representation the right side is supplied in.
    pub(super) supplied: RepresentationKind,
    /// Whether rows were delivered to a sink.
    pub(super) delivered: bool,
    /// The representation of the Join index it acquired.
    pub(super) acquired: Option<RepresentationKind>,
    /// The index's rows, cells and capacity.
    pub(super) index: (usize, usize, usize),
    /// The rows delivered before the answer.
    pub(super) prefix_rows: usize,
    /// Whether it answered the computed run's cells, or its refusal.
    pub(super) same_answer: bool,
    /// What the graph owner held once the file finished.
    pub(super) held: Held,
}

/// What a valid keying joins: the rows of the computed run, then the stored and compact runs.
#[derive(Debug, PartialEq)]
pub(super) struct Joined {
    pub(super) rows: Cow<'static, [[Cell; 5]]>,
    pub(super) attempts: [Attempt; 4],
    /// What the computed run's graph owner held once the file finished.
    pub(super) held: Held,
}

/// What an invalid keying refuses with, on the computed and stored runs.
#[derive(Debug, PartialEq)]
pub(super) struct Refused {
    pub(super) error: OperatingError,
    pub(super) attempts: [Attempt; 4],
    /// What the computed run's graph owner held once the file finished.
    pub(super) held: Held,
}

/// The cells of an answer, or its refusal.
fn cells(answer: &Result<Table, OperatingError>) -> Result<&[Cell], &OperatingError> {
    answer.as_ref().map(|table| table.cells.as_slice())
}

/// Joins the fixture tables on `keying`: a computed oracle run, then each supplied representation
/// with and without delivery.
pub(super) fn exercise(keying: Keying) -> Result<Joined, Refused> {
    let n = |n| Cell::Number(Integer(n));
    let left = Relation::Entry;
    let right = Relation::SequenceElement;
    let ln = schema_column(left, CellKind::Number, 0);
    let lf = schema_column(left, CellKind::Flag, 0);
    let rn = schema_column(right, CellKind::Number, 0);
    let rf = schema_column(right, CellKind::Flag, 0);
    let rp = schema_column(right, CellKind::Number, 1);
    let lhs = table(
        left,
        &[(1, true), (1, false), (1, true), (9, true)].map(|(key, flag)| {
            vec![
                (ln, n(key)),
                (
                    lf,
                    Cell::Flag(if flag { Flag::Raised } else { Flag::Lowered }),
                ),
            ]
        }),
    );
    let rhs = table(
        right,
        &[(1, 2, 10), (2, 0, 20), (1, 1, 11), (3, 1, 30), (1, 0, 12)].map(
            |(key, flag, payload)| {
                vec![
                    (rn, n(key)),
                    (rf, from_bits(CellKind::Flag, Bits(flag))),
                    (rp, n(payload)),
                ]
            },
        ),
    );
    let output = [
        (Side::Right, rp),
        (Side::Left, ln),
        (Side::Right, rf),
        (Side::Right, rn),
        (Side::Right, rp),
    ];
    let (l, r) = match keying {
        Keying::Single => (vec![ln], vec![rn]),
        Keying::Composite => (vec![lf, ln], vec![rf, rn]),
        Keying::Reordered => (vec![ln, lf], vec![rn, rf]),
        Keying::Empty => (vec![], vec![]),
        Keying::Unlike => (vec![ln], vec![rf]),
        Keying::InvalidLeft => (vec![99], vec![rn]),
        Keying::InvalidRight => (vec![ln], vec![99]),
    };
    let invalid = matches!(keying, Keying::InvalidLeft | Keying::InvalidRight);
    let request = if invalid {
        request(&l, &r, &[(Side::Right, rp)])
    } else {
        request(&l, &r, &output)
    };
    let run = |supplied, delivered| -> Observation {
        let providers = if invalid {
            simple(&[1], &[(1, 10)], supplied)
        } else {
            [
                encoded(left, lhs.clone(), RepresentationKind::Computed),
                encoded(right, rhs.clone(), supplied),
            ]
        };
        execute(
            request.clone(),
            providers,
            &mut Symbols::default(),
            b"",
            delivered,
            real_port(),
            Faults::default(),
        )
        .unwrap()
    };
    let computed = run(RepresentationKind::Computed, false);
    let (oracle, held) = (computed.result, computed.held);
    let supplied = if invalid {
        [RepresentationKind::Computed, RepresentationKind::Stored]
    } else {
        [RepresentationKind::Stored, RepresentationKind::Compact]
    };
    let attempts = supplied
        .into_iter()
        .flat_map(|supplied| [false, true].map(|delivered| (supplied, delivered)))
        .map(|(supplied, delivered)| {
            let actual = run(supplied, delivered);
            Attempt {
                supplied,
                delivered,
                acquired: actual.work.index_representation,
                index: (
                    actual.work.index_rows,
                    actual.work.index_cells,
                    actual.work.index_capacity,
                ),
                prefix_rows: actual.prefix.len(),
                same_answer: cells(&actual.result) == cells(&oracle),
                held: actual.held,
            }
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    match oracle {
        Ok(table) => Ok(Joined {
            rows: Cow::Owned(
                table
                    .cells
                    .chunks(5)
                    .map(|row| row.to_vec().try_into().unwrap())
                    .collect(),
            ),
            attempts,
            held,
        }),
        Err(error) => Err(Refused {
            error,
            attempts,
            held,
        }),
    }
}
