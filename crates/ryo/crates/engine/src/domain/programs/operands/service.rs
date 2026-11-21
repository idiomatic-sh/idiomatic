//! The services of the `operands` part: what a word's operands stand for now. A register, a constant, the scanned file and
//! a list of the pool each have one reading here, which every item and the loop use.

use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::types::{CodeEntry, ConstantNumber, PoolAt, WordSlot};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;

/// The word after `at`.
#[inline]
pub(crate) const fn after(at: CodeEntry) -> CodeEntry {
    CodeEntry(at.0.wrapping_add(1))
}

/// The cell a register holds; the zero cell for a register the rule set does not have.
#[inline]
pub(crate) fn cell(frame: &StepFrame<'_, '_>, register: Register) -> CellWord {
    match frame
        .state
        .registers
        .get(usize::try_from(register.0).unwrap_or(usize::MAX))
    {
        Some(held) => *held,
        None => CellWord(0),
    }
}

/// Writes a register.
#[inline]
pub(crate) fn set(frame: &mut StepFrame<'_, '_>, register: Register, value: CellWord) {
    if let Some(slot) = frame
        .state
        .registers
        .get_mut(usize::try_from(register.0).unwrap_or(usize::MAX))
    {
        *slot = value;
    }
}

/// A constant of the code, by number.
#[inline]
pub(crate) fn constant(frame: &StepFrame<'_, '_>, number: ConstantNumber) -> CellWord {
    match frame
        .fill
        .programs
        .code
        .constants
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
    {
        Some(held) => *held,
        None => CellWord(0),
    }
}

/// The scanned file, as a cell.
#[inline]
pub(crate) fn file_cell(frame: &StepFrame<'_, '_>) -> CellWord {
    CellWord(u64::from(frame.fill.file.0))
}

/// A list of the pool: its length, then its items.
#[inline]
pub(crate) fn listed(pool: &[WordSlot], at: PoolAt) -> &[WordSlot] {
    let from = usize::try_from(at.0).unwrap_or(usize::MAX);
    let length = pool
        .get(from)
        .map_or(0, |slot| usize::try_from(slot.0).unwrap_or(usize::MAX));
    pool.get(from.saturating_add(1)..from.saturating_add(1).saturating_add(length))
        .unwrap_or_default()
}

/// The cell an operand of the pool stands for now; the zero cell for a set of constants, which only a test reads.
#[inline]
pub(crate) fn operand(frame: &StepFrame<'_, '_>, value: PooledOperand) -> CellWord {
    match value {
        PooledOperand::Register(register) => cell(frame, register),
        PooledOperand::Constant(number) => constant(frame, number),
        PooledOperand::Constants(_) => CellWord(0),
        PooledOperand::File => file_cell(frame),
    }
}

/// What is or-ed onto an element's place to make its id: the scanned file, in the high half.
#[inline]
pub(crate) fn node_tag(frame: &StepFrame<'_, '_>) -> CellWord {
    CellWord(u64::from(frame.fill.file.0) << u32::BITS)
}
