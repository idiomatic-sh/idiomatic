//! The `replay` service's cases, on tables built in the cases.

use ryo_language::domain::relations::types::{CellKind, FileId, Relation, Value as Cell};
use ryo_language::domain::symbols::types::Symbols;

use super::service::{derive, replay_computed, scan};
use super::types::{Collect, Pipe};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply};
use crate::domain::table_model::service::{copy_table, push_row};
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Scans the three rows of a table into a table.
    ScansATable,
    /// Scans a table into a taker that refuses the first row.
    TakerRefuses,
    /// Derives the context of a step from the context of the step before it.
    DerivesAContext,
    /// Holds the two rows of an input, computes them again and passes them on.
    ReplaysComputedRows,
    /// Holds the input, and the computing refuses.
    ComputingRefuses,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many rows were passed on, or how many bytes of source the derived context reads.
type Value = usize;
type Failure = OperatingError;

const SOURCE: &[u8] = b"replayed";

/// A table of `rows` files, one column.
fn files(rows: u32) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File],
        ..Table::default()
    };
    for file in 0..rows {
        push_row(&mut table, &[Cell::File(FileId(file))]);
    }
    table
}

/// An input that holds its table at hand.
struct Held<'f> {
    table: &'f Table,
}

impl<'f> Feed<'f> for Held<'f> {
    fn kinds(&self, _which: InputPlace) -> Result<&[CellKind], OperatingError> {
        Ok(&self.table.kinds)
    }

    fn found(&mut self, _which: InputPlace, _rows: RowCount) {}

    fn streams(&self, _which: InputPlace) -> Supply {
        Supply::AtHand
    }

    fn stream(
        &mut self,
        _which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        scan(&Rows::Computed(self.table), &mut Vec::new(), context, emit)
    }

    fn hold<'t>(
        &mut self,
        _which: InputPlace,
        _context: &mut OperatorContext<'_>,
        _into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        Ok(Rows::Computed(self.table))
    }
}

/// A step that reads one input.
fn operation() -> Operation {
    Operation::Project {
        from: PlanInput::Relation(Relation::Function),
        columns: Vec::new(),
        distinct: RepeatedRows::Dropped,
    }
}

/// Computes the rows of the input as they are.
fn copy(
    _operation: &Operation,
    inputs: &[Rows<'_>],
    _context: &mut OperatorContext<'_>,
    out: &mut Table,
) -> Result<(), OperatingError> {
    match inputs.first() {
        Some(Rows::Computed(table)) => {
            copy_table(table, out);
            Ok(())
        }
        _ => Err(OperatingError::NotRead),
    }
}

/// Computes nothing, and refuses.
fn refuse(
    _operation: &Operation,
    _inputs: &[Rows<'_>],
    _context: &mut OperatorContext<'_>,
    _out: &mut Table,
) -> Result<(), OperatingError> {
    Err(OperatingError::ShapeMismatch)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let (mut memo, mut scratch) = (Vec::new(), OperatorScratch::default());
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    match scenario {
        Scenario::ScansATable => {
            let (from, mut into) = (files(3), Table::default());
            scan(
                &Rows::Computed(&from),
                &mut Vec::new(),
                &mut context,
                &mut Collect(&mut into),
            )?;
            Ok(into.count.0)
        }
        Scenario::TakerRefuses => {
            let from = files(3);
            let mut taker = Pipe(
                |_: &mut OperatorContext<'_>, _: &[Cell]| -> Result<(), OperatingError> {
                    Err(OperatingError::NotRun)
                },
            );
            scan(
                &Rows::Computed(&from),
                &mut Vec::new(),
                &mut context,
                &mut taker,
            )?;
            Ok(0)
        }
        Scenario::DerivesAContext => {
            let own = derive(&mut context, &mut memo, &mut scratch);
            Ok(own.source.0.len())
        }
        Scenario::ReplaysComputedRows | Scenario::ComputingRefuses => {
            let from = files(2);
            let mut into = Table::default();
            let compute = if matches!(scenario, Scenario::ComputingRefuses) {
                refuse
            } else {
                copy
            };
            replay_computed(
                &operation(),
                &mut Held { table: &from },
                &mut context,
                &mut Room::default(),
                &mut Collect(&mut into),
                compute,
            )?;
            Ok(into.count.0)
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_passes_on_every_row_of_a_table"),
        word: Word::Succeeds,
        input: || Scenario::ScansATable,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_derived_context_reads_the_source_of_the_context_it_comes_from"),
        word: Word::Succeeds,
        input: || Scenario::DerivesAContext,
        expect: Expect::Value(SOURCE.len()),
    },
    Case {
        name: CaseName("a_step_that_holds_its_inputs_passes_on_the_rows_it_computes"),
        word: Word::Succeeds,
        input: || Scenario::ReplaysComputedRows,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_scan_stops_at_the_row_the_taker_refuses"),
        word: Word::Refuses,
        input: || Scenario::TakerRefuses,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("rows_the_computing_refuses_are_not_passed_on"),
        word: Word::Refuses,
        input: || Scenario::ComputingRefuses,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
