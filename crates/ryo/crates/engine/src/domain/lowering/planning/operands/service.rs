//! The services of the `operands` part: what a term is as an operand, how a cell stands, and the registers a branch's accesses read from.

use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{draft, fresh_register, kind_bits, text_word};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::service::variables;
use crate::domain::lowering::flattening::types::RowColumn;
use crate::domain::lowering::flattening::types::{AtomRows, FlatBranch, FlatTerm, FlatVariable};
use crate::domain::lowering::planning::types::KindAnchor;
use crate::domain::lowering::planning::types::{
    Emitter, Enumerated, Environment, FileCell, UnitRole, WrittenAccess,
};
use crate::domain::steps::calls::service::call_shape;
use crate::domain::steps::calls::types::CallRows;
use crate::domain::steps::types::CallMeaning;
use crate::domain::steps::types::RegisterList;
use crate::domain::steps::types::StepOperands;
use crate::domain::steps::types::{
    AnchorKinds, ColumnMask, ReadColumn, Register, StepOperand, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::relations::types::{CellKind, FactForm, NodeColumn, Relation};
use std::collections::HashMap;

/// How often each variable is written in a branch: in its atoms, and in the columns of its row its unit gives (every
/// column, or the ones named). A variable written once is read by nothing: its cell is never read.
#[must_use]
pub(crate) fn uses_of(
    branch: &FlatBranch,
    given: Option<&ColumnMask>,
) -> HashMap<FlatVariable, Count> {
    let mut uses: HashMap<FlatVariable, Count> = HashMap::new();
    let wanted =
        |place| given.is_none_or(|mask| mask.0.iter().any(|column| usize::from(column.0) == place));
    let row = branch
        .row
        .iter()
        .enumerate()
        .filter_map(|(place, term)| match term {
            FlatTerm::Variable(variable) if wanted(place) => Some(*variable),
            FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => {
                None
            }
        });
    for variable in branch.atoms.iter().flat_map(variables).chain(row) {
        let count = uses.entry(variable).or_insert(Count(0));
        *count = Count(count.0.saturating_add(1));
    }
    uses
}

/// The word of a constant term; none for a variable and for the file.
pub(crate) fn constant_word(term: FlatTerm) -> Option<CellWord> {
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
            None if environment.keyed.contains_key(&variable)
                || environment.held.contains(&variable) =>
            {
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
pub(crate) fn state_of(environment: &Environment, term: FlatTerm) -> CellState {
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
pub(crate) fn key_value(emitter: &mut Emitter<'_>, term: FlatTerm) -> Option<StepOperand> {
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
            emitter.written.push(WrittenAccess::Let {
                from: StepOperand::Word(word),
                out,
            });
            Some(out)
        }
        (UnitRole::Pipeline, Some(cell)) => {
            let out = fresh_register(emitter.drafts);
            emitter.written.push(WrittenAccess::Read {
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
pub(crate) fn read_column(rows: AtomRows, place: RowColumn) -> Option<ReadColumn> {
    match rows {
        AtomRows::Relation(relation) => {
            let column = *columns(relation).get(place.0)?;
            Some(match fact_form(relation) {
                FactForm::NodeTable => ReadColumn::Node(node_column(column)?),
                FactForm::Filled(_) => ReadColumn::Filled(column),
            })
        }
        AtomRows::Definition(_) | AtomRows::Made(_) => made_column(place).map(ReadColumn::Made),
    }
}

/// The source a rows atom's rows are read from with a plain read: a use of a definition by key is no such read.
pub(crate) fn source_of(rows: AtomRows) -> StepRows {
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
pub(crate) fn anchor_kinds(
    emitter: &Emitter<'_>,
    relation: Relation,
    term: FlatTerm,
) -> Option<KindAnchor> {
    let FlatTerm::Text(number) = term else {
        return None;
    };
    let written = emitter.drafts.written.get(&number)?;
    let bits = kind_bits(emitter.kinds, written)?;
    Some(KindAnchor {
        relation,
        kinds: AnchorKinds([bits].into()),
    })
}

/// One cell of a row read into a register: over the node table the handle is the element itself, so its own column is the
/// handle and nothing is read.
pub(crate) fn cell_register(
    emitter: &mut Emitter<'_>,
    rows: StepRows,
    column: ReadColumn,
    row: Register,
) -> Register {
    if column == ReadColumn::Node(NodeColumn::Id) {
        return row;
    }
    let out = fresh_register(emitter.drafts);
    emitter.written.push(WrittenAccess::Read {
        source: rows,
        column,
        row,
        out,
    });
    out
}

/// Remembers a file column of a row at hand, for a pipeline that later needs the file's cell in a register: a relation's
/// file column, or a definition's.
pub(crate) fn file_noted(emitter: &mut Emitter<'_>, source: AtomRows, row: Register) {
    let place = match source {
        AtomRows::Relation(relation) => {
            let listed = columns(relation);
            listed
                .iter()
                .position(|column| kind(*column) == CellKind::File)
        }
        AtomRows::Definition(definition) => draft(emitter.drafts, definition).and_then(|draft| {
            let kinds = draft.shape.0.iter();
            kinds.clone().position(|kind| *kind == CellKind::File)
        }),
        AtomRows::Made(_) => None,
    };
    if let Some(column) = place.and_then(|place| read_column(source, RowColumn(place))) {
        emitter.environment.file = Some(FileCell {
            source: source_of(source),
            column,
            row,
        });
    }
}

/// A register holding the file's own cell: a `Let` in a program; in a pipeline, a read of a file column already at hand.
pub(crate) fn file_register(emitter: &mut Emitter<'_>) -> Option<Register> {
    match (emitter.role, emitter.environment.file) {
        (UnitRole::Branch | UnitRole::Pattern, _) => {
            let out = fresh_register(emitter.drafts);
            emitter.written.push(WrittenAccess::Let {
                from: StepOperand::File,
                out,
            });
            Some(out)
        }
        (UnitRole::Pipeline, Some(cell)) => {
            let out = fresh_register(emitter.drafts);
            emitter.written.push(WrittenAccess::Read {
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

/// The register of a given term, for a walk: a walk starts from a register.
pub(crate) fn register_of(emitter: &mut Emitter<'_>, term: FlatTerm) -> Option<Register> {
    match key_value(emitter, term)? {
        StepOperand::Register(register) => Some(register),
        StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => None,
    }
}

/// Gives a term the cell in `out`: a variable nothing gave takes it; one that was given is tested against it.
pub(crate) fn given_or_tested(emitter: &mut Emitter<'_>, term: FlatTerm, out: Register) {
    match (term, operand(&emitter.environment, term)) {
        (FlatTerm::Variable(variable), None) => {
            let _new = emitter.environment.registers.insert(variable, out);
        }
        (_, Some(right)) => emitter.written.push(WrittenAccess::Test {
            test: TestOperation::Equal,
            left: StepOperand::Register(out),
            right,
        }),
        (FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File, None) => {}
    }
}

/// The operand of a test's term. The file is no operand of a test: it is put in a register first.
pub(crate) fn test_operand(emitter: &mut Emitter<'_>, term: FlatTerm) -> Option<StepOperand> {
    match term {
        FlatTerm::File => file_register(emitter).map(StepOperand::Register),
        FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) => {
            operand(&emitter.environment, term)
        }
    }
}

/// A per-row meaning as an access: it reads the operands of its arguments, and each cell it gives goes to a variable
/// nothing gave yet, or is tested against one that was given.
pub(crate) fn called(
    emitter: &mut Emitter<'_>,
    meaning: CallMeaning,
    arguments: &[FlatTerm],
    outputs: &[FlatTerm],
) {
    let read: Option<Vec<StepOperand>> = arguments
        .iter()
        .map(|term| operand(&emitter.environment, *term))
        .collect();
    let Some(read) = read else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return;
    };
    let gives: Vec<Register> = outputs
        .iter()
        .map(|_| fresh_register(emitter.drafts))
        .collect();
    // A meaning that gives several rows is an iteration: nothing after it is the unit's first enumeration.
    if call_shape(meaning).rows == CallRows::Several {
        emitter.environment.enumerated = Enumerated::Already;
    }
    emitter.written.push(WrittenAccess::Call {
        meaning,
        arguments: StepOperands(read.into()),
        outputs: RegisterList(gives.clone().into()),
    });
    for (term, out) in outputs.iter().zip(gives) {
        given_or_tested(emitter, *term, out);
    }
}
