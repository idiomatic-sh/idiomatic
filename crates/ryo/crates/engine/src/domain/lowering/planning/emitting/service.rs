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
    AnchorKinds, AxisDirection, ColumnList, ColumnMask, CostClass, DefinitionId, MadeColumn,
    MadeTableId,
    MatchColumns, Polarity, ReadColumn, Register, RegisterList, StepOperand, StepOperands,
    StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{CellKind, FactForm, KindBits, NodeColumn, Relation};
use std::collections::HashMap;

/// How often each variable is written in a branch: in its atoms, and in the columns of its row its unit gives (every
/// column, or the ones named). A variable written once is read by nothing: its cell is never read.
#[must_use]
pub(crate) fn uses_of(
    branch: &FlatBranch,
    given: Option<&ColumnMask>,
) -> HashMap<FlatVariable, usize> {
    let mut uses: HashMap<FlatVariable, usize> = HashMap::new();
    let wanted = |place: usize| {
        given.is_none_or(|mask| mask.0.iter().any(|column| usize::from(column.0) == place))
    };
    let row = branch.row.iter().enumerate().filter_map(|(place, term)| match term {
        FlatTerm::Variable(variable) if wanted(place) => Some(*variable),
        FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => None,
    });
    for variable in branch.atoms.iter().flat_map(variables).chain(row) {
        let count = uses.entry(variable).or_default();
        *count = count.saturating_add(1);
    }
    uses
}

/// The word of a constant term; none for a variable and for the file.
fn constant_word(term: FlatTerm) -> Option<CellWord> {
    match term {
        FlatTerm::Text(number) => Some(text_word(number)),
        FlatTerm::Number(number) => Some(CellWord(number.0.cast_unsigned())),
        FlatTerm::Variable(_) | FlatTerm::File => None,
    }
}

/// The operand of a term: a given variable's register, or its constant when an access was keyed by its pin; a constant's
/// word; the file. None for a variable nothing gave.
#[must_use]
pub(crate) fn operand(environment: &Environment, term: FlatTerm) -> Option<StepOperand> {
    match term {
        FlatTerm::Variable(variable) => match environment.registers.get(&variable) {
            Some(register) => Some(StepOperand::Register(*register)),
            None if environment.keyed.contains_key(&variable) => {
                let pin = *environment.pins.get(&variable)?;
                constant_word(pin).map(StepOperand::Word)
            }
            None => None,
        },
        FlatTerm::Text(_) | FlatTerm::Number(_) => constant_word(term).map(StepOperand::Word),
        FlatTerm::File => Some(StepOperand::File),
    }
}

/// How a cell stands in an environment: a variable something gave is a register's value; a variable nothing gave is a
/// constant when an equality pins it, and open otherwise.
fn state_of(environment: &Environment, term: FlatTerm) -> CellState {
    match term {
        FlatTerm::Variable(variable)
            if environment.registers.contains_key(&variable)
                || environment.keyed.contains_key(&variable) =>
        {
            CellState::Register
        }
        FlatTerm::Variable(variable) if environment.pins.contains_key(&variable) => {
            CellState::Constant
        }
        FlatTerm::Variable(_) => CellState::Open,
        FlatTerm::Text(_) | FlatTerm::Number(_) => CellState::Constant,
        FlatTerm::File => CellState::File,
    }
}

/// The value a key takes for a cell. A given variable is a register's value: one an access was keyed by as a constant is
/// put in a register first (a `Let` in a program, a read of the row it was keyed in for a pipeline), so that a key of the
/// plan is a key of the access. A pinned variable nothing gave is its constant: the bucket a unit starts from.
fn key_value(emitter: &mut Emitter<'_>, term: FlatTerm) -> Option<StepOperand> {
    let FlatTerm::Variable(variable) = term else {
        return operand(&emitter.environment, term);
    };
    if let Some(register) = emitter.environment.registers.get(&variable) {
        return Some(StepOperand::Register(*register));
    }
    let word = constant_word(*emitter.environment.pins.get(&variable)?)?;
    let Some(place) = emitter.environment.keyed.get(&variable).copied() else {
        return Some(StepOperand::Word(word));
    };
    let written = match (emitter.role, place) {
        (UnitRole::Branch | UnitRole::Pattern, _) => {
            let out = fresh_register(emitter.drafts);
            emitter.written.push(Emitted::Let {
                from: StepOperand::Word(word),
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
    };
    match written {
        Some(out) => {
            let _new = emitter.environment.registers.insert(variable, out);
            Some(StepOperand::Register(out))
        }
        None => Some(StepOperand::Word(word)),
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
            // A variable nothing gave takes the row's cell, when something else reads it: a pin of it is tested after.
            (FlatTerm::Variable(variable), CellState::Open | CellState::Constant) => {
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
        let state = state_of(&emitter.environment, *cell);
        match (*cell, state, key.contains(&place)) {
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
                if emitter.uses.get(&variable).copied().unwrap_or(0) >= 2 {
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
    let mut pinned = Vec::new();
    for place in &choice.cells {
        let (Some(column), Some(cell)) = (read_column(source, *place), cells.get(*place)) else {
            continue;
        };
        let state = states.get(*place).copied();
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
    emitter.written.push(Emitted::Rows {
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

/// The register of a given term, for a walk: a walk starts from a register.
fn register_of(emitter: &mut Emitter<'_>, term: FlatTerm) -> Option<Register> {
    match key_value(emitter, term)? {
        StepOperand::Register(register) => Some(register),
        StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => None,
    }
}

/// Gives a term the cell in `out`: a variable nothing gave takes it; one that was given is tested against it.
fn given_or_tested(emitter: &mut Emitter<'_>, term: FlatTerm, out: Register) {
    match (term, operand(&emitter.environment, term)) {
        (FlatTerm::Variable(variable), None) => {
            let _new = emitter.environment.registers.insert(variable, out);
        }
        (_, Some(right)) => emitter.written.push(Emitted::Test {
            test: TestOperation::Equal,
            left: StepOperand::Register(out),
            right,
        }),
        (FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File, None) => {}
    }
}

/// The axis that walks an axis back: from what it reaches to what it starts from.
const fn reversed(direction: AxisDirection) -> Option<AxisDirection> {
    match direction {
        AxisDirection::Ancestor => Some(AxisDirection::Descendant),
        AxisDirection::Descendant => Some(AxisDirection::Ancestor),
        AxisDirection::NextSibling => Some(AxisDirection::PreviousSibling),
        AxisDirection::PreviousSibling => Some(AxisDirection::NextSibling),
        // A parent's children are a walk of their own (`Children`): no axis walks to them.
        AxisDirection::Parent => None,
    }
}

/// A walk along a tree axis. From the end that is given it enumerates the other; between two given elements a program
/// tests the containment in place, and a pipeline walks from the start and keeps the rows that reach the other.
fn walked(
    emitter: &mut Emitter<'_>,
    relation: Relation,
    direction: AxisDirection,
    ends: (FlatTerm, FlatTerm),
) {
    let given = |emitter: &Emitter<'_>, term| state_of(&emitter.environment, term) == CellState::Register;
    let (from_start, from_reached) = (given(emitter, ends.0), given(emitter, ends.1));
    emitter.environment.enumerated = Enumerated::Already;
    let contained = match direction {
        AxisDirection::Ancestor => Some((ends.1, ends.0)),
        AxisDirection::Descendant => Some((ends.0, ends.1)),
        AxisDirection::Parent | AxisDirection::NextSibling | AxisDirection::PreviousSibling => None,
    };
    if let (true, true, Some((ancestor, descendant)), UnitRole::Branch | UnitRole::Pattern) =
        (from_start, from_reached, contained, emitter.role)
    {
        let registers = (register_of(emitter, ancestor), register_of(emitter, descendant));
        match registers {
            (Some(ancestor), Some(descendant)) => emitter.written.push(Emitted::Within {
                relation,
                ancestor,
                descendant,
            }),
            (None, _) | (_, None) => emitter.missed = emitter.missed.saturating_add(1),
        }
        return;
    }
    // From the start, along the axis; from what is reached, back along it: a parent's children are a walk of their own.
    let (walk, from, to) = if from_start {
        (Some(direction), ends.0, ends.1)
    } else {
        (reversed(direction), ends.1, ends.0)
    };
    let Some(start) = register_of(emitter, from) else {
        emitter.missed = emitter.missed.saturating_add(1);
        return;
    };
    let out = fresh_register(emitter.drafts);
    emitter.written.push(match walk {
        Some(direction) => Emitted::Axis {
            relation,
            direction,
            start,
            out,
        },
        None => Emitted::Children {
            relation,
            parent: start,
            out,
        },
    });
    given_or_tested(emitter, to, out);
}

/// A nearest walk. A program runs it in place, the domain's definition asked whether it holds the element under test. A
/// pipeline holds no walk with a condition: it uses the pattern of its own definition that binds the start and collects
/// what is reached, whose program is this walk, run once per distinct start.
fn nearest(
    emitter: &mut Emitter<'_>,
    walk: (Relation, DefinitionId, MadeColumn),
    start: FlatTerm,
    reached: FlatTerm,
) {
    let Some(from) = register_of(emitter, start) else {
        emitter.missed = emitter.missed.saturating_add(1);
        return;
    };
    emitter.environment.enumerated = Enumerated::Already;
    let out = fresh_register(emitter.drafts);
    let column_of = |emitter: &Emitter<'_>, term: FlatTerm| {
        let place = emitter.row.iter().position(|each| *each == term)?;
        u16::try_from(place).ok().map(MadeColumn)
    };
    match (emitter.role, emitter.unit) {
        (UnitRole::Pipeline, Some(unit)) => {
            let (Some(bound), Some(collected)) =
                (column_of(emitter, start), column_of(emitter, reached))
            else {
                emitter.missed = emitter.missed.saturating_add(1);
                return;
            };
            let (bound, collected) = (ColumnMask([bound].into()), ColumnMask([collected].into()));
            asked(emitter.drafts, unit, &bound, &collected);
            emitter.written.push(Emitted::Use {
                definition: unit,
                bound,
                collected,
                values: StepOperands([StepOperand::Register(from)].into()),
                outputs: RegisterList([out].into()),
                mode: UseMode::Rows,
            });
        }
        (UnitRole::Branch | UnitRole::Pattern | UnitRole::Pipeline, _) => {
            let bound = ColumnMask([walk.2].into());
            asked(emitter.drafts, walk.1, &bound, &ColumnMask::default());
            let element = fresh_register(emitter.drafts);
            emitter.written.push(Emitted::Nearest {
                relation: walk.0,
                domain: walk.1,
                column: walk.2,
                element,
                start: from,
                out,
            });
        }
    }
    given_or_tested(emitter, reached, out);
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
                emitter.missed = emitter.missed.saturating_add(1);
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
        FlatAtom::Walk {
            relation,
            direction,
            start,
            reached,
        } => walked(emitter, *relation, *direction, (*start, *reached)),
        FlatAtom::Nearest {
            relation,
            domain,
            column,
            start,
            reached,
        } => nearest(emitter, (*relation, *domain, *column), *start, *reached),
        FlatAtom::Test { test, left, right } => {
            // A pin an access was keyed by holds already.
            if let (TestOperation::Equal, FlatTerm::Variable(variable)) = (test, left)
                && !emitter.environment.registers.contains_key(variable)
                && emitter.environment.keyed.contains_key(variable)
            {
                return;
            }
            let (Some(left), Some(right)) = (
                operand(&emitter.environment, *left),
                operand(&emitter.environment, *right),
            ) else {
                emitter.missed = emitter.missed.saturating_add(1);
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
