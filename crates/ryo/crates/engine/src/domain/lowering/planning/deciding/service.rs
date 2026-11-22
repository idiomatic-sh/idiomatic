//! The services of the `deciding` part: a decision as an access. A definition is asked through the pattern of the columns
//! the decision binds, which collects nothing. A relation's own rows are asked in place by the cells given: a program
//! writes the read as the body of an `Exists`, a pipeline as a probe that asks present or absent.

use crate::domain::lowering::definitions::service::{asked_by, fresh_register};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{AtomRows, FlatTerm, RowColumn};
use crate::domain::lowering::planning::deciding::types::AskedCell;
use crate::domain::lowering::planning::operands::service::{operand, read_column, source_of};
use crate::domain::lowering::planning::types::{Emitter, WrittenAccess};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    ColumnMask, MatchColumns, Polarity, ReadColumn, RegisterList, StepOperands,
};
use ryo_relations::domain::relations::types::{Count, NodeColumn};

/// The cells asked of a source, each with its place: the file's own cell is asked of nothing.
fn asked_cells(emitter: &Emitter<'_>, cells: &[Option<FlatTerm>]) -> Vec<AskedCell> {
    let each = cells.iter().enumerate().filter_map(|(place, cell)| {
        let value = cell.and_then(|cell| match cell {
            FlatTerm::File => None,
            FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) => {
                operand(&emitter.environment, cell)
            }
        });
        value.map(|value| AskedCell {
            place: RowColumn(place),
            value,
        })
    });
    each.collect()
}

/// Whether a key of these columns reads a source in place: any columns of a filled relation or a made table are one key;
/// the node table is read by one of its element, its parent or its span.
pub(crate) fn keys_in_place(listed: &[ReadColumn]) -> Option<()> {
    let node = listed
        .iter()
        .any(|column| matches!(column, ReadColumn::Node(_)));
    let one = matches!(
        listed,
        [ReadColumn::Node(
            NodeColumn::Id | NodeColumn::Parent | NodeColumn::Span
        )]
    );
    (!listed.is_empty() && (!node || one)).then_some(())
}

/// A decision: the cells asked are the source's bound columns, and nothing is collected. A relation its key cannot read in
/// place has no form here: the miss is counted.
pub(crate) fn decided(
    emitter: &mut Emitter<'_>,
    polarity: Polarity,
    rows: AtomRows,
    cells: &[Option<FlatTerm>],
) {
    let mode = match polarity {
        Polarity::Present => UseMode::Present,
        Polarity::Absent => UseMode::Absent,
    };
    let asked_of = asked_cells(emitter, cells);
    let values = StepOperands(asked_of.iter().map(|cell| cell.value.clone()).collect());
    match rows {
        AtomRows::Definition(definition) => {
            let columns = asked_of.iter().filter_map(|cell| made_column(cell.place));
            let bound = ColumnMask(columns.collect());
            asked_by(emitter.drafts, definition, &bound, &ColumnMask::default(), emitter.role);
            emitter.written.push(WrittenAccess::Use {
                definition,
                bound,
                collected: ColumnMask::default(),
                values,
                outputs: RegisterList::default(),
                mode,
            });
        }
        AtomRows::Relation(_) | AtomRows::Made(_) => {
            let columns = asked_of.iter().map(|cell| read_column(rows, cell.place));
            let listed: Option<Vec<ReadColumn>> = columns.collect();
            let Some(listed) = listed.filter(|listed| keys_in_place(listed).is_some()) else {
                emitter.missed = Count(emitter.missed.0.saturating_add(1));
                return;
            };
            let out = fresh_register(emitter.drafts);
            emitter.written.push(WrittenAccess::Rows {
                source: source_of(rows),
                columns: MatchColumns(listed.into()),
                values,
                kinds: None,
                mode,
                out,
            });
        }
    }
}
