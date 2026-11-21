//! The services of the `emitting` part: each atom of a planned branch as accesses over registers. A source's bound cells are
//! its key; a cell that is not part of the key is read from the row found and, when it was given already, tested; a variable
//! nothing else reads is never read. Every use of a definition asks the pattern of its bound and collected columns.

use crate::domain::lowering::costing::service::keyed;
use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{asked, fresh_register};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatTerm, RowColumn, RowsPerFile,
};
use crate::domain::lowering::planning::closures::service::closed;
use crate::domain::lowering::planning::operands::service::{
    anchor_kinds, cell_register, file_noted, file_register, key_value, operand, read_column,
    source_of, state_of, test_operand,
};
use crate::domain::lowering::planning::types::NearestTreeWalk;
use crate::domain::lowering::planning::types::WalkEnds;
use crate::domain::lowering::planning::types::{
    Emitter, Enumerated, FileCell, UnitRole, WrittenAccess,
};
use crate::domain::lowering::planning::walks::service::{nearest, walked};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    ColumnList, ColumnMask, CostClass, DefinitionId, MadeColumn, MadeTableId, MatchColumns,
    Polarity, ReadColumn, Register, RegisterList, StepOperand, StepOperands, TestOperation,
};
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::relations::types::NodeColumn;

/// Reads the cells of a row that its key did not hold: a variable nothing gave yet takes the cell when something else reads
/// it; a cell that was given, or is a constant, is tested against the row's. The file's cell is the file: never read.
fn cells_read(
    emitter: &mut Emitter<'_>,
    source: AtomRows,
    cells: &[FlatTerm],
    key: &[RowColumn],
    row: Register,
) {
    let rows = source_of(source);
    for (place, cell) in cells.iter().enumerate() {
        let Some(column) =
            read_column(source, RowColumn(place)).filter(|_| !key.contains(&RowColumn(place)))
        else {
            continue;
        };
        match (*cell, state_of(&emitter.environment, *cell)) {
            (_, CellState::File) => {}
            // A variable nothing gave takes the row's cell, when something else reads it: a pin of it is tested after.
            (FlatTerm::Variable(variable), CellState::Open | CellState::Constant) => {
                if emitter.uses.get(&variable).is_some_and(|uses| uses.0 >= 2) {
                    let out = cell_register(emitter, rows, column, row);
                    let _new = emitter.environment.registers.insert(variable, out);
                }
            }
            (given, CellState::Register | CellState::Constant | CellState::Open) => {
                if let Some(right) = operand(&emitter.environment, given) {
                    let out = cell_register(emitter, rows, column, row);
                    emitter.written.push(WrittenAccess::Test {
                        test: TestOperation::Equal,
                        left: StepOperand::Register(out),
                        right,
                    });
                }
            }
        }
    }
}

/// A keyed use of a definition: its bound cells are its values, and the open variables something else reads are collected.
fn used(
    emitter: &mut Emitter<'_>,
    definition: DefinitionId,
    cells: &[FlatTerm],
    key: &[RowColumn],
) {
    let (mut bound, mut values) = (Vec::new(), Vec::new());
    let (mut collected, mut outputs, mut tests) = (Vec::new(), Vec::new(), Vec::new());
    for (place, cell) in cells.iter().enumerate() {
        let Some(column) = made_column(RowColumn(place)) else {
            continue;
        };
        let state = state_of(&emitter.environment, *cell);
        match (*cell, state, key.contains(&RowColumn(place))) {
            (_, CellState::File, _) => {}
            // A bound cell: a given variable's value, or the constant a pin holds the cell to.
            (given, CellState::Register | CellState::Constant, true) => {
                if let Some(value) = key_value(emitter, given) {
                    bound.push(column);
                    values.push(value);
                    if let (FlatTerm::Variable(variable), CellState::Constant) = (given, state) {
                        let _new = emitter.environment.keyed.insert(variable, None);
                    }
                }
            }
            (FlatTerm::Variable(variable), CellState::Open | CellState::Constant, _) => {
                if emitter.uses.get(&variable).is_some_and(|uses| uses.0 >= 2) {
                    let out = fresh_register(emitter.drafts);
                    collected.push(column);
                    outputs.push(out);
                    let _new = emitter.environment.registers.insert(variable, out);
                }
            }
            (given, CellState::Register | CellState::Constant | CellState::Open, _) => {
                // A variable the use itself gave in an earlier cell: this cell is collected and tested.
                if let Some(value) = operand(&emitter.environment, given) {
                    let out = fresh_register(emitter.drafts);
                    collected.push(column);
                    outputs.push(out);
                    tests.push(WrittenAccess::Test {
                        test: TestOperation::Equal,
                        left: StepOperand::Register(out),
                        right: value,
                    });
                }
            }
        }
    }
    let (bound, collected) = (ColumnMask(bound.into()), ColumnMask(collected.into()));
    asked(emitter.drafts, definition, &bound, &collected);
    let mode = if collected.0.is_empty() {
        UseMode::Present
    } else {
        UseMode::Rows
    };
    emitter.written.push(WrittenAccess::Use {
        definition,
        bound,
        collected,
        values: StepOperands(values.into()),
        outputs: RegisterList(outputs.into()),
        mode,
    });
    emitter.written.extend(tests);
}

/// Records the columns a made table is read by: each set is one index of it.
pub(crate) fn indexed(drafts: &mut Drafts, table: MadeTableId, listed: &[ReadColumn]) {
    let made: Vec<MadeColumn> = listed
        .iter()
        .filter_map(|column| match column {
            ReadColumn::Made(made) => Some(*made),
            ReadColumn::Node(_) | ReadColumn::Filled(_) => None,
        })
        .collect();
    let list = ColumnList(made.into());
    let place = usize::try_from(table.0).unwrap_or(usize::MAX);
    if let Some(table) = drafts.tables.get_mut(place)
        && !table.indexes.contains(&list)
    {
        table.indexes.push(list);
    }
}

/// The rows of a source, by its bound cells or whole, then the cells its key did not hold.
fn rows_read(
    emitter: &mut Emitter<'_>,
    source: AtomRows,
    cells: &[FlatTerm],
    per_file: RowsPerFile,
) {
    let states: Vec<CellState> = cells
        .iter()
        .map(|cell| state_of(&emitter.environment, *cell))
        .collect();
    let first = emitter.environment.enumerated == Enumerated::NotYet;
    let mut choice = keyed(emitter.drafts, source, per_file, &states);
    // A bucket of the file is a unit's first enumeration; after it, an access with no register is every row: a product.
    if choice.class == CostClass::File && !first {
        choice.cells.clear();
    }
    emitter.environment.enumerated = Enumerated::Already;
    // A definition read by a key is a use of it. A bucket of its rows is a use too where a program starts from it; a
    // pipeline scans the bucket of its table.
    let called = choice.class == CostClass::Fanout || emitter.role != UnitRole::Pipeline;
    if let (AtomRows::Definition(definition), false, true) =
        (source, choice.cells.is_empty(), called)
    {
        used(emitter, definition, cells, &choice.cells);
        return;
    }
    let (mut listed, mut values, mut kinds) = (Vec::new(), Vec::new(), None);
    if per_file == RowsPerFile::One && !first {
        // One row a file: read by the file's own cell, the table's first key.
        if let Some(file) = file_register(emitter) {
            listed.push(ReadColumn::Made(MadeColumn(0)));
            values.push(StepOperand::Register(file));
        }
    }
    let mut pinned = Vec::new();
    for place in &choice.cells {
        let (Some(column), Some(cell)) = (read_column(source, *place), cells.get(place.0)) else {
            continue;
        };
        let state = states.get(place.0).copied();
        let Some(value) = key_value(emitter, *cell) else {
            continue;
        };
        if let (FlatTerm::Variable(variable), Some(CellState::Constant)) = (*cell, state) {
            pinned.push((variable, column));
            if let (ReadColumn::Node(NodeColumn::Kind), AtomRows::Relation(relation)) =
                (column, source)
            {
                kinds = emitter
                    .environment
                    .pins
                    .get(&variable)
                    .and_then(|pin| anchor_kinds(emitter, relation, *pin));
            }
        }
        listed.push(column);
        values.push(value);
    }
    if let AtomRows::Definition(definition) = source {
        let nothing = ColumnMask::default();
        asked(emitter.drafts, definition, &nothing, &nothing);
    }
    if let (AtomRows::Made(table), false) = (source, listed.is_empty()) {
        indexed(emitter.drafts, table, &listed);
    }
    let out = fresh_register(emitter.drafts);
    emitter.written.push(WrittenAccess::Rows {
        source: source_of(source),
        columns: MatchColumns(listed.into()),
        values: StepOperands(values.into()),
        kinds,
        out,
    });
    // A pinned variable the access was keyed by is given from here on: as its constant, and by the row's own cell.
    for (variable, column) in pinned {
        let cell = FileCell {
            source: source_of(source),
            column,
            row: out,
        };
        let _new = emitter.environment.keyed.insert(variable, Some(cell));
    }
    file_noted(emitter, source, out);
    cells_read(emitter, source, cells, &choice.cells, out);
}

/// A decision of a definition: the cells asked are its bound columns, and it collects nothing.
fn decided(
    emitter: &mut Emitter<'_>,
    polarity: Polarity,
    definition: DefinitionId,
    cells: &[Option<FlatTerm>],
) {
    let (mut bound, mut values) = (Vec::new(), Vec::new());
    for (place, cell) in cells.iter().enumerate() {
        let value = cell.and_then(|cell| match cell {
            FlatTerm::File => None,
            FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) => {
                operand(&emitter.environment, cell)
            }
        });
        if let (Some(value), Some(column)) = (value, made_column(RowColumn(place))) {
            bound.push(column);
            values.push(value);
        }
    }
    let bound = ColumnMask(bound.into());
    asked(emitter.drafts, definition, &bound, &ColumnMask::default());
    emitter.written.push(WrittenAccess::Use {
        definition,
        bound,
        collected: ColumnMask::default(),
        values: StepOperands(values.into()),
        outputs: RegisterList::default(),
        mode: match polarity {
            Polarity::Present => UseMode::Present,
            Polarity::Absent => UseMode::Absent,
        },
    });
}

/// Writes one atom of a branch.
pub(crate) fn atom_written(emitter: &mut Emitter<'_>, atom: &FlatAtom) {
    match atom {
        FlatAtom::Rows {
            rows,
            cells,
            per_file,
        } => rows_read(emitter, *rows, cells, *per_file),
        FlatAtom::Latest {
            table,
            key,
            bound,
            side,
            cells,
        } => {
            let operands: Option<Vec<StepOperand>> = key
                .iter()
                .map(|cell| operand(&emitter.environment, *cell))
                .collect();
            let (Some(key), Some(bound)) = (operands, operand(&emitter.environment, *bound)) else {
                emitter.missed = Count(emitter.missed.0.saturating_add(1));
                return;
            };
            let out = fresh_register(emitter.drafts);
            emitter.environment.enumerated = Enumerated::Already;
            emitter.written.push(WrittenAccess::Latest {
                table: *table,
                key: StepOperands(key.into()),
                bound,
                side: *side,
                out,
            });
            cells_read(emitter, AtomRows::Made(*table), cells, &[], out);
        }
        FlatAtom::Decided {
            polarity,
            definition,
            cells,
        } => decided(emitter, *polarity, *definition, cells),
        FlatAtom::Walk {
            relation,
            direction,
            start,
            reached,
        } => {
            let ends = WalkEnds {
                start: *start,
                reached: *reached,
            };
            walked(emitter, *relation, *direction, ends);
        }
        FlatAtom::Nearest {
            relation,
            domain,
            column,
            start,
            reached,
        } => {
            let walk = NearestTreeWalk {
                relation: *relation,
                domain: *domain,
                column: *column,
            };
            nearest(emitter, walk, *start, *reached);
        }
        FlatAtom::Closure {
            edges,
            keys,
            source,
            reached,
        } => {
            let ends = WalkEnds {
                start: *source,
                reached: *reached,
            };
            closed(emitter, edges, keys, ends);
        }
        FlatAtom::Test { test, left, right } => {
            // A pin an access was keyed by holds already.
            if let (TestOperation::Equal, FlatTerm::Variable(variable)) = (test, left)
                && !emitter.environment.registers.contains_key(variable)
                && emitter.environment.keyed.contains_key(variable)
            {
                return;
            }
            let (Some(left), Some(right)) =
                (test_operand(emitter, *left), test_operand(emitter, *right))
            else {
                emitter.missed = Count(emitter.missed.0.saturating_add(1));
                return;
            };
            emitter.written.push(WrittenAccess::Test {
                test: *test,
                left,
                right,
            });
        }
    }
}
