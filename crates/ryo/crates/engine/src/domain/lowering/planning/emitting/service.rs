//! The services of the `emitting` part: each atom of a planned branch as accesses over registers. A source's bound cells are
//! its key; a cell that is not part of the key is read from the row found and, when it was given already, tested; a variable
//! nothing else reads is never read. Every use of a definition asks the pattern of its bound and collected columns.

use crate::domain::lowering::costing::service::keyed;
use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{asked, fresh_register, text_word};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::variables;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile,
};
use crate::domain::lowering::planning::types::{
    Emitted, Emitter, Enumerated, Environment, FileCell, UnitRole,
};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    AnchorKinds, ColumnList, ColumnMask, CostClass, DefinitionId, MadeColumn, MadeTableId,
    MatchColumns, Polarity, ReadColumn, Register, RegisterList, StepOperand, StepOperands,
    StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{CellKind, FactForm, KindBits, NodeColumn, Relation};
use std::collections::HashMap;

/// How often each variable is written in a branch: in its atoms and in its row.
#[must_use]
pub(crate) fn uses_of(branch: &FlatBranch) -> HashMap<FlatVariable, usize> {
    let mut uses: HashMap<FlatVariable, usize> = HashMap::new();
    let row = branch.row.iter().filter_map(|term| match term {
        FlatTerm::Variable(variable) => Some(*variable),
        FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => None,
    });
    for variable in branch.atoms.iter().flat_map(variables).chain(row) {
        let count = uses.entry(variable).or_default();
        *count = count.saturating_add(1);
    }
    uses
}

/// The operand of a term: a given variable's register, a constant's word, the file. None for a variable nothing gave.
#[must_use]
pub(crate) fn operand(environment: &Environment, term: FlatTerm) -> Option<StepOperand> {
    Some(match term {
        FlatTerm::Variable(variable) => {
            StepOperand::Register(*environment.registers.get(&variable)?)
        }
        FlatTerm::Text(number) => StepOperand::Word(text_word(number)),
        FlatTerm::Number(number) => StepOperand::Word(CellWord(number.0.cast_unsigned())),
        FlatTerm::File => StepOperand::File,
    })
}

/// How a cell stands in an environment.
fn state_of(environment: &Environment, term: FlatTerm) -> CellState {
    match term {
        FlatTerm::Variable(variable) if environment.registers.contains_key(&variable) => {
            CellState::Register
        }
        FlatTerm::Variable(_) => CellState::Open,
        FlatTerm::Text(_) | FlatTerm::Number(_) => CellState::Constant,
        FlatTerm::File => CellState::File,
    }
}

/// The column of a source a cell is read from, by the cell's place.
fn read_column(rows: AtomRows, place: usize) -> Option<ReadColumn> {
    match rows {
        AtomRows::Relation(relation) => {
            let column = *columns(relation).get(place)?;
            Some(match fact_form(relation) {
                FactForm::NodeTable => ReadColumn::Node(node_column(column)?),
                FactForm::Filled(_) => ReadColumn::Filled(column),
            })
        }
        AtomRows::Definition(_) | AtomRows::Made(_) => {
            Some(ReadColumn::Made(MadeColumn(u16::try_from(place).ok()?)))
        }
    }
}

/// The source a rows atom's rows are read from with a plain read: a use of a definition by key is no such read.
fn source_of(rows: AtomRows) -> StepRows {
    match rows {
        AtomRows::Relation(relation) => match fact_form(relation) {
            FactForm::NodeTable => StepRows::Node(relation),
            FactForm::Filled(_) => StepRows::Filled(relation),
        },
        AtomRows::Definition(definition) => StepRows::Definition(definition),
        AtomRows::Made(table) => StepRows::Made(table),
    }
}

/// The kinds a constant kind names in the vocabulary: its place there; none when the vocabulary does not hold it.
fn anchor_kinds(
    emitter: &Emitter<'_>,
    relation: Relation,
    term: FlatTerm,
) -> Option<(Relation, AnchorKinds)> {
    let FlatTerm::Text(number) = term else {
        return None;
    };
    let written = emitter.drafts.texts.get(usize::try_from(number.0).ok()?)?;
    let named = emitter.kinds.kinds.iter();
    let place = named.clone().position(|kind| kind.0 == written.0)?;
    let bits = KindBits(u16::try_from(place).ok()?);
    Some((relation, AnchorKinds([bits].into())))
}

/// One cell of a row read into a register: over the node table the handle is the element itself, so its own column is the
/// handle and nothing is read.
fn cell_register(
    emitter: &mut Emitter<'_>,
    rows: StepRows,
    column: ReadColumn,
    row: Register,
) -> Register {
    if column == ReadColumn::Node(NodeColumn::Id) {
        return row;
    }
    let out = fresh_register(emitter.drafts);
    emitter.written.push(Emitted::Read {
        source: rows,
        column,
        row,
        out,
    });
    out
}

/// Reads the cells of a row that its key did not hold: a variable nothing gave yet takes the cell when something else reads
/// it; a cell that was given, or is a constant, is tested against the row's. The file's cell is the file: never read.
fn cells_read(
    emitter: &mut Emitter<'_>,
    source: AtomRows,
    cells: &[FlatTerm],
    key: &[usize],
    row: Register,
) {
    let rows = source_of(source);
    for (place, cell) in cells.iter().enumerate() {
        let Some(column) = read_column(source, place).filter(|_| !key.contains(&place)) else {
            continue;
        };
        match (*cell, state_of(&emitter.environment, *cell)) {
            (_, CellState::File) => {}
            (FlatTerm::Variable(variable), CellState::Open) => {
                if emitter.uses.get(&variable).copied().unwrap_or(0) >= 2 {
                    let out = cell_register(emitter, rows, column, row);
                    let _new = emitter.environment.registers.insert(variable, out);
                }
            }
            (given, CellState::Register | CellState::Constant | CellState::Open) => {
                if let Some(right) = operand(&emitter.environment, given) {
                    let out = cell_register(emitter, rows, column, row);
                    emitter.written.push(Emitted::Test {
                        test: TestOperation::Equal,
                        left: StepOperand::Register(out),
                        right,
                    });
                }
            }
        }
    }
}

/// Remembers a file column of a row at hand, for a pipeline that later needs the file's cell in a register.
fn file_noted(emitter: &mut Emitter<'_>, source: AtomRows, row: Register) {
    let AtomRows::Relation(relation) = source else {
        return;
    };
    let listed = columns(relation);
    let place = listed
        .iter()
        .position(|column| kind(*column) == CellKind::File);
    if let Some(column) = place.and_then(|place| read_column(source, place)) {
        emitter.environment.file = Some(FileCell {
            source: source_of(source),
            column,
            row,
        });
    }
}

/// A register holding the file's own cell: a `Let` in a program; in a pipeline, a read of a file column already at hand.
fn file_register(emitter: &mut Emitter<'_>) -> Option<Register> {
    match (emitter.role, emitter.environment.file) {
        (UnitRole::Branch | UnitRole::Pattern, _) => {
            let out = fresh_register(emitter.drafts);
            emitter.written.push(Emitted::Let {
                from: StepOperand::File,
                out,
            });
            Some(out)
        }
        (UnitRole::Pipeline, Some(cell)) => {
            let out = fresh_register(emitter.drafts);
            emitter.written.push(Emitted::Read {
                source: cell.source,
                column: cell.column,
                row: cell.row,
                out,
            });
            Some(out)
        }
        (UnitRole::Pipeline, None) => None,
    }
}

/// A keyed use of a definition: its bound cells are its values, and the open variables something else reads are collected.
fn used(emitter: &mut Emitter<'_>, definition: DefinitionId, cells: &[FlatTerm], key: &[usize]) {
    let (mut bound, mut values) = (Vec::new(), Vec::new());
    let (mut collected, mut outputs, mut tests) = (Vec::new(), Vec::new(), Vec::new());
    for (place, cell) in cells.iter().enumerate() {
        let Ok(column) = u16::try_from(place).map(MadeColumn) else {
            continue;
        };
        match (*cell, state_of(&emitter.environment, *cell)) {
            (_, CellState::File) => {}
            (FlatTerm::Variable(variable), CellState::Open) => {
                if emitter.uses.get(&variable).copied().unwrap_or(0) >= 2 {
                    let out = fresh_register(emitter.drafts);
                    collected.push(column);
                    outputs.push(out);
                    let _new = emitter.environment.registers.insert(variable, out);
                }
            }
            (given, CellState::Register | CellState::Constant | CellState::Open) => {
                let Some(value) = operand(&emitter.environment, given) else {
                    continue;
                };
                if key.contains(&place) {
                    bound.push(column);
                    values.push(value);
                } else {
                    // A variable the use itself gave in an earlier cell: this cell is collected and tested.
                    let out = fresh_register(emitter.drafts);
                    collected.push(column);
                    outputs.push(out);
                    tests.push(Emitted::Test {
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
    emitter.written.push(Emitted::Use {
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
    for place in &choice.cells {
        let (Some(column), Some(cell)) = (read_column(source, *place), cells.get(*place)) else {
            continue;
        };
        let Some(value) = operand(&emitter.environment, *cell) else {
            continue;
        };
        if let (ReadColumn::Node(NodeColumn::Kind), AtomRows::Relation(relation)) = (column, source)
        {
            kinds = anchor_kinds(emitter, relation, *cell);
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
    emitter.written.push(Emitted::Rows {
        source: source_of(source),
        columns: MatchColumns(listed.into()),
        values: StepOperands(values.into()),
        kinds,
        out,
    });
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
        if let (Some(value), Ok(column)) = (value, u16::try_from(place)) {
            bound.push(MadeColumn(column));
            values.push(value);
        }
    }
    let bound = ColumnMask(bound.into());
    asked(emitter.drafts, definition, &bound, &ColumnMask::default());
    emitter.written.push(Emitted::Use {
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
            let (Some(key), Some(bound)) = (operands, operand(&emitter.environment, *bound))
            else {
                return;
            };
            let out = fresh_register(emitter.drafts);
            emitter.environment.enumerated = Enumerated::Already;
            emitter.written.push(Emitted::Latest {
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
        FlatAtom::Test { test, left, right } => {
            let (Some(left), Some(right)) = (
                operand(&emitter.environment, *left),
                operand(&emitter.environment, *right),
            ) else {
                return;
            };
            emitter.written.push(Emitted::Test {
                test: *test,
                left,
                right,
            });
        }
    }
}
