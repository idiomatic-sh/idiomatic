//! `Read`: one cell of a row, by the row's handle, written to a register: a made table's, a filled relation's or the node
//! table's, read through the fill side's seam.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::deciding::service::{read_filled_of, read_made_of, read_node_of};
use crate::domain::code::types::{CodeEntry, CodeWord, Op, Short};
use crate::domain::made_tables::sources::service::cell_of;
use crate::domain::made_tables::types::ReadSource;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, set};
use ryo_relations::domain::relations::types::NodeExtent;

/// The `Read` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Read;

impl InstructionsItem for Read {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let (source, column, row, out) = if word.op == Op::ReadNode {
            let read = read_node_of(word);
            (
                ReadSource::Nodes(NodeExtent::Embedded),
                read.column,
                read.row,
                read.out,
            )
        } else if word.op == Op::ReadFilled {
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
        let value = cell_of(frame.fill, source, column, handle);
        set(frame, out, value);
        Flow::Next
    }
}
