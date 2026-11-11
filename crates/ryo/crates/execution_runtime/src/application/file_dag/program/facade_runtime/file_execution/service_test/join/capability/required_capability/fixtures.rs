//! The support of the `required_capability` cases.
use super::super::super::cases::fixtures::{node, table_of, tree_rows, wide_tree};
use super::super::super::*;
use super::super::fixtures::Witness;
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_cells::types::TreeForm;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn, Words};
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::{Admitted, CompactBacking, DecodeMode, WordSlice};
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The representation the right side of the number join is supplied in.
#[derive(Clone, Copy, Debug)]
pub(super) enum Supply {
    Computed,
    Stored,
    Compact,
}

/// The join a row runs.
#[derive(Clone, Debug)]
pub(super) enum Join {
    /// The one-row left `[7]` joined to the right `[7, 1], [7, 2]` supplied as `right`.
    Number { right: Supply },
    /// A one-node table joined with a tree's elements, the tree on the left or the right.
    Tree { tree_left: bool },
    /// An empty left joined to `[7, 1]` with these faults, delivered or not, under a refusing
    /// witness or the real operators.
    EmptyLeft {
        left: Option<OperatingError>,
        right: Option<OperatingError>,
        delivery: bool,
        refusing: bool,
    },
}

/// What a join answered: its rows, or its error.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Delivered {
    Rows(usize),
    Error(OperatingError),
}

/// One capability run under the refusing witness and its rerun under the real operators.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Capability {
    pub(super) delivery: bool,
    pub(super) optional_limit: Option<usize>,
    /// What the witness-refused run answered.
    pub(super) refused: Delivered,
    /// The rows that run delivered before it was refused.
    pub(super) prefix_rows: usize,
    /// How often the witness was asked for the join.
    pub(super) witness_calls: usize,
    /// What the real operators answered for the same join.
    pub(super) accepted: Delivered,
    /// What the graph owner held once each file finished: the witness-refused run's, then the rerun's.
    pub(super) held: [Held; 2],
}

/// What a join over an empty left observed.
#[derive(Debug, PartialEq)]
pub(super) struct Gate {
    pub(super) delivered: Delivered,
    pub(super) witness_calls: usize,
    pub(super) attempts: Cow<'static, [SourceAttempt]>,
    pub(super) reads: [usize; 2],
    pub(super) index: (usize, usize),
    pub(super) prefix_rows: usize,
    /// What the graph owner held once the file finished.
    pub(super) held: Held,
}

/// What a row observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// Every delivery and optional limit, in order: (false, None), (false, Some(0)), (true, None), (true, Some(0)).
    Capability {
        runs: Cow<'static, [Capability]>,
        /// The names the compact backing of the right table admitted.
        admitted_names: usize,
    },
    Gate(Gate),
}

fn delivered(result: Result<Table, OperatingError>) -> Delivered {
    match result {
        Ok(table) => Delivered::Rows(table.count.0),
        Err(error) => Delivered::Error(error),
    }
}

fn checked(
    op: &Operation,
    rows: [Rows<'_>; 2],
    symbols: &mut Symbols,
) -> Result<Vec<Capability>, OperatingError> {
    let mut runs = Vec::new();
    for delivery in [false, true] {
        for optional_limit in [None, Some(0)] {
            let witness = Arc::new(Witness {
                refuse: true,
                ..Witness::default()
            });
            let observed = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                witness.clone(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )?;
            // A refusing dependency does not alter the immutable compiled meaning.
            let accepted = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )?;
            runs.push(Capability {
                delivery,
                optional_limit,
                refused: delivered(observed.result),
                prefix_rows: observed.prefix.len(),
                witness_calls: witness.calls.load(Ordering::Relaxed),
                accepted: delivered(accepted.result),
                held: [observed.held, accepted.held],
            });
        }
    }
    Ok(runs)
}

fn number_join(right: Supply) -> Result<Observed, OperatingError> {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let table = table_of(
        &[CellKind::Number, CellKind::Number],
        &[
            &[Cell::Number(Integer(7)), Cell::Number(Integer(1))],
            &[Cell::Number(Integer(7)), Cell::Number(Integer(2))],
        ],
    );
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let columns = [FactColumn(vec![7, 7]), FactColumn(vec![1, 2])];
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    let Admitted {
        table: compact,
        names,
    } = admit_compact(
        CompactBacking(Arc::new(KeptWords(pack(WordSlice(&words.0))))),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .expect("the number table packs");
    let rows = match right {
        Supply::Computed => Rows::Computed(&table),
        Supply::Stored => Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &columns,
        }),
        Supply::Compact => Rows::Compact(&compact),
    };
    Ok(Observed::Capability {
        runs: checked(&op, [Rows::Computed(&left), rows], &mut Symbols::default())?.into(),
        admitted_names: names.len(),
    })
}

fn tree_join(tree_left: bool) -> Result<Vec<Capability>, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, true);
    let nodes = table_of(&[CellKind::Node], &[&[node(1)]]);
    let op = operation(&[0], &[0], &[(Side::Right, 0)], vec![], &[]);
    let tree = tree_rows(&elements, TreeForm::Elements);
    let rows = if tree_left {
        [tree, Rows::Computed(&nodes)]
    } else {
        [Rows::Computed(&nodes), tree]
    };
    checked(&op, rows, &mut symbols)
}

fn empty_left(
    left_fault: Option<OperatingError>,
    right_fault: Option<OperatingError>,
    delivery: bool,
    refusing: bool,
) -> Result<Gate, OperatingError> {
    let left = table_of(&[CellKind::Number], &[]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let witness = Arc::new(Witness {
        refuse: true,
        ..Witness::default()
    });
    let port: Arc<dyn crate::outbound::operators::interface::OperatorsPort> = if refusing {
        witness.clone()
    } else {
        real_port()
    };
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        delivery,
        port,
        Faults {
            left: left_fault,
            right: right_fault,
            ..Faults::default()
        },
    )?;
    Ok(Gate {
        delivered: delivered(actual.result),
        witness_calls: witness.calls.load(Ordering::Relaxed),
        attempts: actual.attempts.into(),
        reads: actual.reads,
        index: (actual.work.index_rows, actual.work.index_cells),
        prefix_rows: actual.prefix.len(),
        held: actual.held,
    })
}

/// Runs the join and observes its capability runs or its empty-left gate.
pub(super) fn exercise(join: Join) -> Result<Observed, OperatingError> {
    match join {
        Join::Number { right } => number_join(right),
        Join::Tree { tree_left } => Ok(Observed::Capability {
            runs: tree_join(tree_left)?.into(),
            admitted_names: 0,
        }),
        Join::EmptyLeft {
            left,
            right,
            delivery,
            refusing,
        } => Ok(Observed::Gate(empty_left(left, right, delivery, refusing)?)),
    }
}
