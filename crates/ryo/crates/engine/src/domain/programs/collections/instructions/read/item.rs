//! `Read`: one cell of a row, by the row's handle, written to a register: a made table's, a filled relation's or the node
//! table's, read through the fill side's seam. Where a made table's or a relation's column lies is asked of the seam at
//! the word's first read in a file and held for that file (`made_tables::reading::service::column_held`); the node
//! table's cells are made when read, one at a time.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::deciding::service::{read_filled_of, read_made_of, read_node_of};
use crate::domain::code::types::{CodeEntry, CodeWord, Op, Short};
use crate::domain::made_tables::columns::types::HeldCells;
use crate::domain::made_tables::reading::service::{cell_held, column_held};
use crate::domain::made_tables::sources::service::cell_of;
use crate::domain::made_tables::types::ReadSource;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, set};
use crate::domain::scratch::kept::types::HeldRead;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, NodeExtent};

/// The `Read` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Read;

impl InstructionsItem for Read {
    #[inline(always)]
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) -> Flow {
        if word.op == Op::ReadNode {
            let read = read_node_of(word);
            let handle = cell(frame, read.row);
            let source = ReadSource::Nodes(NodeExtent::Embedded);
            let value = cell_of(frame.fill, source, read.column, handle);
            set(frame, read.out, value);
            return Flow::Next;
        }
        let (source, column, row, out) = if word.op == Op::ReadFilled {
            let read = read_filled_of(word);
            (
                ReadSource::Filled(read.relation),
                read.place,
                read.row,
                read.out,
            )
        } else {
            let read = read_made_of(word);
            (
                ReadSource::Made(read.table),
                Short(read.column.0),
                read.row,
                read.out,
            )
        };
        let handle = cell(frame, row);
        let slot = frame
            .state
            .decoded
            .lists
            .get(usize::try_from(at.0).unwrap_or(usize::MAX))
            .map(|list| list.start);
        let file = Some(frame.fill.store.stamp);
        let held = slot.and_then(|place| frame.state.held_reads.get(place.0));
        let value = match held {
            Some(read) if read.file == file => cell_held(frame.fill.store, read.column, handle),
            Some(_) | None => first_read(frame, source, column, slot, handle),
        };
        set(frame, out, value);
        Flow::Next
    }
}

/// A word's first read of a source's column in a file: the cell, and where the column lies is held for the word's later
/// reads in the file. A column that lies nowhere yet (a table not made for the file, a relation with no row of the file)
/// is not held: it is asked again at the next read.
#[inline(never)]
fn first_read(
    frame: &mut StepFrame<'_, '_>,
    source: ReadSource,
    column: Short,
    slot: Option<Count>,
    handle: CellWord,
) -> CellWord {
    let lies = column_held(frame.fill.store, source, column);
    if lies.cells != HeldCells::Nothing
        && let Some(place) = slot
    {
        let held = HeldRead {
            file: Some(frame.fill.store.stamp),
            column: lies,
        };
        let reads = &mut frame.state.held_reads;
        if reads.len() <= place.0 {
            reads.resize(place.0.saturating_add(1), HeldRead { file: None, ..held });
        }
        if let Some(read) = reads.get_mut(place.0) {
            *read = held;
        }
    }
    cell_of(frame.fill, source, column, handle)
}
