//! The services of the `kept` part: which registers hold the file.

use super::types::RegisterHolds;
use crate::domain::code::constants::{CONSTANT, THE_FILE};
use crate::domain::code::deciding::service::{let_value_of, read_node_of};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::small_codes::service::node_column_of;
use crate::domain::code::types::{Op, ProgramCode};
use crate::domain::scratch::types::AskPart;
use crate::domain::steps::types::Register;
use ryo_relations::domain::relations::types::NodeColumn;

/// Whether a register is marked as holding the file.
#[inline]
pub(crate) fn holds_file(marks: &[RegisterHolds], register: Register) -> RegisterHolds {
    match marks.get(usize::try_from(register.0).unwrap_or(usize::MAX)) {
        Some(RegisterHolds::TheFile) => RegisterHolds::TheFile,
        Some(RegisterHolds::Cells) | None => RegisterHolds::Cells,
    }
}

/// Marks a register as holding the file; answers whether the mark is new.
pub(crate) fn mark_file(marks: &mut Vec<RegisterHolds>, register: Register) -> RegisterHolds {
    let place = usize::try_from(register.0).unwrap_or(usize::MAX);
    if marks.len() <= place && place < usize::MAX {
        marks.resize(place.saturating_add(1), RegisterHolds::Cells);
    }
    match marks.get_mut(place) {
        Some(held) if *held == RegisterHolds::Cells => {
            *held = RegisterHolds::TheFile;
            RegisterHolds::TheFile
        }
        Some(_) | None => RegisterHolds::Cells,
    }
}

/// Marks every register that holds the file, from the code and the marks already there: a register does when some word
/// that writes it writes the file into it: a `Let` of the file; a `ReadNode` of the node table's file column; a `Let`
/// from a register that holds the file; the formal of a pattern in call form whose asker gives the file or such a
/// register. (A `Read` of a made table's own file column is known at the word's first read, and marked then.) A test
/// of a text whose left is such a register reads the file's path, not the text of the word numbered like the file:
/// the rule the fill side's pipelines follow (2121c643a), said of the loop's words. Once a worker, and again when a
/// read marks one more.
pub(crate) fn files_marked(code: &ProgramCode, parts: &[AskPart], marks: &mut Vec<RegisterHolds>) {
    loop {
        let mut new = RegisterHolds::Cells;
        let mut mark = |marks: &mut Vec<RegisterHolds>, register: Register| {
            if mark_file(marks, register) == RegisterHolds::TheFile {
                new = RegisterHolds::TheFile;
            }
        };
        for word in &code.words {
            match word.op {
                Op::Let => {
                    let read = let_value_of(*word);
                    let from_file = read.flag.0 & THE_FILE.0 != 0
                        || (read.flag.0 & CONSTANT.0 == 0
                            && holds_file(marks, Register(read.from.0)) == RegisterHolds::TheFile);
                    if from_file {
                        mark(marks, read.out);
                    }
                }
                Op::ReadNode => {
                    let read = read_node_of(*word);
                    if node_column_of(read.column) == Some(NodeColumn::File) {
                        mark(marks, read.out);
                    }
                }
                _ => {}
            }
        }
        for part in parts {
            let given = match part.value {
                Some(PooledOperand::File) => RegisterHolds::TheFile,
                Some(PooledOperand::Register(from)) => holds_file(marks, from),
                Some(PooledOperand::Constant(_) | PooledOperand::Constants(_)) | None => {
                    RegisterHolds::Cells
                }
            };
            if given == RegisterHolds::TheFile {
                mark(marks, part.formal);
            }
        }
        if new == RegisterHolds::Cells {
            break;
        }
    }
}
