//! The services of the `operands` part: what a stage's word names, read through the code's readers into what a loop takes.
//! A register is a column of the batch, a constant one cell for every row, the file its number.

use crate::domain::code::records::service::{OPERAND_SLOTS, operand_at};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::service::list;
use crate::domain::code::types::{PoolAt, ProgramCode, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::made_tables::pipelines::types::OperandRun;
use crate::domain::stages::types::{BatchValue, KeyPart};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{Count, FileId, Value};

/// The file as the cell a row holds of it.
#[inline]
pub(crate) fn file_cell(file: FileId) -> CellWord {
    CellWord(to_bits(Value::File(file)).0)
}

/// The cell of a constant, by its number.
///
/// # Errors
///
/// The code holds no constant of that number.
#[inline]
pub(crate) fn constant_at(code: &ProgramCode, number: WordSlot) -> Result<CellWord, FilledError> {
    code.constants
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(FilledError::Code {
            named: MissingCode::Constant,
        })
}

/// The column of a register in a batch; no rows for a register the batch has no column of.
#[inline]
pub(crate) fn column_of(columns: &[Vec<CellWord>], register: Register) -> &[CellWord] {
    columns
        .get(usize::try_from(register.0).unwrap_or(usize::MAX))
        .map_or(&[], Vec::as_slice)
}

/// The `at`-th operand of a run of operands that starts at `first`.
///
/// # Errors
///
/// The pool does not reach it, or holds no operand there.
#[inline]
pub(crate) fn operand(
    code: &ProgramCode,
    first: PoolAt,
    at: Count,
) -> Result<PooledOperand, FilledError> {
    let missing = FilledError::Code {
        named: MissingCode::Pool,
    };
    let ahead = u32::try_from(at.0)
        .unwrap_or(u32::MAX)
        .checked_mul(OPERAND_SLOTS.0)
        .and_then(|slots| first.0.checked_add(slots));
    match ahead {
        Some(place) => operand_at(&code.pool, PoolAt(place)).ok_or(missing),
        None => Err(missing),
    }
}

/// The value of the `at`-th bound formal of a body record in call form, whose pairs of a formal and its operand start at
/// `formals`: each pair is the formal's register, then the operand.
///
/// # Errors
///
/// The pool does not reach it, or holds no operand there.
#[inline]
pub(crate) fn formal_value(
    code: &ProgramCode,
    formals: PoolAt,
    at: Count,
) -> Result<PooledOperand, FilledError> {
    let missing = FilledError::Code {
        named: MissingCode::Pool,
    };
    let place = u32::try_from(at.0)
        .unwrap_or(u32::MAX)
        .checked_mul(OPERAND_SLOTS.0.saturating_add(1))
        .and_then(|slots| slots.checked_add(1))
        .and_then(|ahead| formals.0.checked_add(ahead));
    match place {
        Some(place) => operand_at(&code.pool, PoolAt(place)).ok_or(missing),
        None => Err(missing),
    }
}

/// The arguments of a call: its pool record is how many, then that many operands.
///
/// # Errors
///
/// The pool does not reach the record.
pub(crate) fn call_arguments(
    code: &ProgramCode,
    record: PoolAt,
) -> Result<OperandRun, FilledError> {
    let count = code
        .pool
        .get(usize::try_from(record.0).unwrap_or(usize::MAX))
        .ok_or(FilledError::Code {
            named: MissingCode::Pool,
        })?;
    Ok(OperandRun {
        first: PoolAt(record.0.saturating_add(1)),
        count: Count(usize::try_from(count.0).unwrap_or(usize::MAX)),
    })
}

/// What a loop reads for an operand: a column of the batch, or one cell for every row.
///
/// # Errors
///
/// The operand is a set, which only a membership test reads; or a constant the code does not hold.
#[inline]
pub(crate) fn value_of<'l>(
    columns: &'l [Vec<CellWord>],
    code: &ProgramCode,
    file: FileId,
    operand: PooledOperand,
) -> Result<BatchValue<'l>, FilledError> {
    match operand {
        PooledOperand::Register(register) => Ok(BatchValue::Column(column_of(columns, register))),
        PooledOperand::Constant(number) => {
            constant_at(code, WordSlot(number.0)).map(BatchValue::Constant)
        }
        PooledOperand::File => Ok(BatchValue::Constant(file_cell(file))),
        PooledOperand::Constants(_) => Err(FilledError::Code {
            named: MissingCode::Pool,
        }),
    }
}

/// The key a loop asks an index: `count` operands from `first`, a constant's cell written into `cells` at its place, a
/// register's column added to `parts` with its place.
///
/// # Errors
///
/// An operand the pool does not hold.
pub(crate) fn key_of<'l>(
    columns: &'l [Vec<CellWord>],
    code: &ProgramCode,
    file: FileId,
    operands: OperandRun,
    cells: &mut Vec<CellWord>,
    parts: &mut Vec<KeyPart<'l>>,
) -> Result<(), FilledError> {
    let OperandRun { first, count } = operands;
    cells.clear();
    cells.resize(count.0, CellWord(0));
    parts.clear();
    for place in 0..count.0 {
        match value_of(columns, code, file, operand(code, first, Count(place))?)? {
            BatchValue::Column(column) => parts.push(KeyPart {
                place: Count(place),
                column,
            }),
            BatchValue::Constant(cell) => {
                if let Some(slot) = cells.get_mut(place) {
                    *slot = cell;
                }
            }
        }
    }
    Ok(())
}

/// The cells of a sorted list of constants, in the list's order, into `set`.
///
/// # Errors
///
/// A constant the code does not hold.
pub(crate) fn set_of(
    code: &ProgramCode,
    at: WordSlot,
    set: &mut Vec<CellWord>,
) -> Result<(), FilledError> {
    set.clear();
    for number in list(&code.pool, at) {
        set.push(constant_at(code, *number)?);
    }
    Ok(())
}

/// The registers a list in the pool names.
pub(crate) fn registers_at(code: &ProgramCode, at: PoolAt) -> impl Iterator<Item = Register> + '_ {
    list(&code.pool, WordSlot(at.0))
        .iter()
        .map(|slot| Register(slot.0))
}
