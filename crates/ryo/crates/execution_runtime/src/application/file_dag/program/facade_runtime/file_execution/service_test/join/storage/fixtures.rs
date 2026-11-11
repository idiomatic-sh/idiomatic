//! The support of the `storage` cases.
use super::super::cases::fixtures::table_of;
use super::super::*;
use crate::application::file_dag::program::facade_runtime::file_execution::service_test::fixtures::*;
use fixtures::execution::*;
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::{
    plan_parts::types::{Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::Table,
};
use std::borrow::Cow;

pub(super) fn fact_column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|n| to_bits(Cell::Number(Integer(*n))).0)
            .collect(),
    )
}

/// Compares the Join over `right` with the Join over its computed copy, delivered and not, and
/// joins an empty primary.
pub(super) fn parity(op: &Operation, left: &Table, right: Rows<'_>, computed: &Table) -> Parity {
    let expected = made(
        op,
        [Rows::Computed(left), Rows::Computed(computed)],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    let actual = made(
        op,
        [Rows::Computed(left), right.clone()],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    let delivered = invoke(
        op,
        [Rows::Computed(left), right],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    let in_rows = delivered
        .prefix
        .iter()
        .map(Vec::as_slice)
        .eq(expected.cells.chunks_exact(expected.kinds.len()));
    let empty = table_of(&left.kinds, &[]);
    let skipped = invoke(
        op,
        [Rows::Computed(&empty), Rows::Computed(computed)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    Parity {
        rows: expected.count.0,
        made_equal: actual == expected,
        delivered_equal: (
            delivered.result.as_ref().unwrap().cells == expected.cells,
            in_rows,
        ),
        skipped: (skipped.result.unwrap().count.0, skipped.work.index_rows),
    }
}

/// The fault an unsorted duplicate Join runs with.
#[derive(Clone, Copy, Debug)]
pub(super) enum Fault {
    None,
    /// The sink refuses after one row.
    SinkAfterOne,
    /// The left source refuses as not run.
    Left,
    /// The right source refuses as unread.
    Right,
}

/// The Join a row runs.
#[derive(Clone, Copy, Debug)]
pub(super) enum Join {
    /// Two scalar rows joined to three.
    Scalar,
    /// A stored right side, on these left and right keys.
    StoredRight(&'static [u32], &'static [u32]),
    /// A stored selection repeating its projected positions, on these left and right keys.
    Repeated(&'static [u32], &'static [u32]),
    /// Stored columns of unequal length, materialized then joined.
    MalformedStorage,
    /// Unsorted duplicate stored rows under a fault.
    Unsorted(Fault),
    /// A stored provider truncated by the fixture.
    MalformedProvider,
}

/// How a stored or skipped right side compares with its computed copy.
#[derive(Debug, PartialEq)]
pub(super) struct Parity {
    /// The computed copy's output rows.
    pub(super) rows: usize,
    /// Whether the stored side makes the computed copy's table.
    pub(super) made_equal: bool,
    /// Whether delivering answers the computed copy's cells, and delivers them row by row.
    pub(super) delivered_equal: (bool, bool),
    /// An empty primary's output rows and Join index rows.
    pub(super) skipped: (usize, usize),
}

/// What the Join answered.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// The output's rows and cells.
    Rows {
        rows: usize,
        cells: Cow<'static, [Cell]>,
    },
    /// The stored side against its computed copy.
    Parity(Parity),
    /// The materialized rows and cells, and the cells joined from them.
    Materialized {
        rows: usize,
        logical: Cow<'static, [Cell]>,
        joined: Cow<'static, [Cell]>,
    },
    /// The delivered cells and the Join index's cells.
    Delivered {
        prefix: Cow<'static, [Cell]>,
        index_cells: usize,
    },
}

/// A refused Join: the error, the cells delivered before it and the Join index's rows.
#[derive(Debug, PartialEq)]
pub(super) struct Refused {
    pub(super) error: OperatingError,
    pub(super) prefix: Cow<'static, [Cell]>,
    pub(super) index_rows: usize,
}

/// The delivered rows, or the refusal.
fn delivered(actual: Observation) -> Result<Observed, Refused> {
    match actual.result.err() {
        None => Ok(Observed::Delivered {
            prefix: Cow::Owned(actual.prefix.concat()),
            index_cells: actual.work.index_cells,
        }),
        Some(error) => Err(Refused {
            error,
            prefix: Cow::Owned(actual.prefix.concat()),
            index_rows: actual.work.index_rows,
        }),
    }
}

/// Joins one left row to unsorted duplicate stored rows, with the run's fault.
pub(super) fn unsorted(fault: Fault) -> Result<Observed, Refused> {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(2)]]);
    let columns = [fact_column(&[2, 1, 2, 2]), fact_column(&[20, 10, 21, 22])];
    let right = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let faults = match fault {
        Fault::None => Faults::default(),
        Fault::SinkAfterOne => Faults {
            sink_after: Some(1),
            ..Faults::default()
        },
        Fault::Left => Faults {
            left: Some(OperatingError::NotRun),
            ..Faults::default()
        },
        Fault::Right => Faults {
            right: Some(OperatingError::NotRead),
            ..Faults::default()
        },
    };
    let actual = invoke(
        &op,
        [Rows::Computed(&left), right],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        faults,
    )
    .unwrap();
    delivered(actual)
}

/// Joins through a stored provider whose second column the fixture truncates to two rows.
pub(super) fn malformed_provider() -> Result<Observed, Refused> {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(1)]]);
    let mut steps = vec![];
    let (left, l) = provider(Rows::Computed(&left), 0, &mut steps);
    let right_columns = [FactColumn(vec![1, 1, 1]), FactColumn(vec![10, 11, 12])];
    let (mut right, r) = provider(
        Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &right_columns,
        }),
        1,
        &mut steps,
    );
    if let Operation::Select { kept, .. } = &steps[1].operation {
        right.columns[kept[1].0 as usize].0.truncate(2);
    }
    let mut join = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    if let Operation::Join {
        left: from,
        right: other,
        ..
    } = &mut join
    {
        *from = l;
        *other = r;
    }
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: join,
    });
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = steps;
    request.emit.step = StepId(2);
    let actual = execute(
        request,
        [left, right],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    delivered(actual)
}
