//! The services of the `visiting` part: the `Graph(OrderedDfs)` fill. The roots and the edges are gathered through their
//! pipelines. The edges are sorted by their keys, their source and their order; each distinct keys and root is walked
//! once over the edges of its keys by the operators crate's meaning (`ordered_dfs::service::walked`), and a row of the
//! table is the root's keys, the root, then the visited edge's source, destination and identity, and the ordinal of the
//! visit among its root's, from 0.

use crate::domain::code::records::types::VisitColumns;
use crate::domain::code::types::WordSlot;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::made_tables::service::column;
use crate::domain::made_tables::types::TableRows;
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::ordered_dfs::service::{unrepeated, walked};
use ryo_operators::domain::ordered_dfs::types::{WalkEdge, WalkEntity};
use ryo_relations::domain::relations::types::{Count, Integer};

/// What the fill is told of its two inputs: the walk's columns, each side's key columns, and the edges' order column.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VisitTold<'c> {
    /// The root of a root row; the source, destination and identity of an edge row.
    pub(crate) columns: VisitColumns,
    /// The roots' key columns.
    pub(crate) root_keys: &'c [WordSlot],
    /// The edges' key columns, matched to the roots' place by place.
    pub(crate) edge_keys: &'c [WordSlot],
    /// The edges' order column.
    pub(crate) order: Count,
}

/// The cells of the columns `keys` and then of `last`, of every row of a table: `keys.len() + 1` cells a row.
fn keyed(table: &TableRows, keys: &[WordSlot], last: Count) -> Vec<CellWord> {
    let columns: Vec<_> = keys
        .iter()
        .map(|key| Count(usize::try_from(key.0).unwrap_or(usize::MAX)))
        .chain([last])
        .map(|place| column(table, place))
        .collect();
    let mut cells = Vec::with_capacity(table.count.0.saturating_mul(columns.len()));
    for row in 0..table.count.0 {
        cells.extend(columns.iter().map(|view| cell_at(*view, Count(row))));
    }
    cells
}

/// The `Graph(OrderedDfs)` fill: for each distinct keys and root of `roots`, the edges of `edges` in its keys visited
/// from it, each with its ordinal.
///
/// # Errors
///
/// `DuplicateOrder` when two edges of one keys and source have one order, as today's kernel.
pub(crate) fn visit_groups(
    roots: &TableRows,
    edges: &TableRows,
    told: VisitTold<'_>,
    out: &mut TableRows,
) -> Result<(), PortableOperatorFault> {
    let place = |column: u16| Count(usize::from(column));
    let width = told.root_keys.len().saturating_add(1);
    // Each side's keys and then its entity (a root's root, an edge's source), row by row.
    let root_cells = keyed(roots, told.root_keys, place(told.columns.root.0));
    let edge_cells = keyed(edges, told.edge_keys, place(told.columns.source.0));
    let of = |cells: &'_ [CellWord], row: usize| -> Vec<CellWord> {
        let from = row.saturating_mul(width);
        cells
            .get(from..from.saturating_add(width))
            .unwrap_or_default()
            .to_vec()
    };
    let orders = column(edges, told.order);
    let destinations = column(edges, place(told.columns.destination.0));
    let identities = column(edges, place(told.columns.edge.0));
    let order_of = |row: usize| Integer(cell_at(orders, Count(row)).0.cast_signed());
    // The edges by keys, source and order: a keys' edges are one stretch, sorted as a walk reads them.
    let mut sorted: Vec<usize> = (0..edges.count.0).collect();
    sorted.sort_by(|one, other| {
        of(&edge_cells, *one)
            .cmp(&of(&edge_cells, *other))
            .then_with(|| order_of(*one).cmp(&order_of(*other)))
    });
    let keys_of = |cells: &[CellWord], row: usize| -> Vec<CellWord> {
        let mut all = of(cells, row);
        let _entity = all.pop();
        all
    };
    let walk_edges: Vec<WalkEdge> = sorted
        .iter()
        .map(|row| WalkEdge {
            source: WalkEntity(of(&edge_cells, *row).last().map_or(0, |cell| cell.0)),
            destination: WalkEntity(cell_at(destinations, Count(*row)).0),
            order: order_of(*row),
        })
        .collect();
    // Two edges of one keys and source with one order have no order to be visited in.
    let mut first = 0_usize;
    while first < sorted.len() {
        let keys = sorted.get(first).map(|row| keys_of(&edge_cells, *row));
        let mut end = first.saturating_add(1);
        while sorted
            .get(end)
            .is_some_and(|row| Some(keys_of(&edge_cells, *row)) == keys)
        {
            end = end.saturating_add(1);
        }
        unrepeated(walk_edges.get(first..end).unwrap_or_default())?;
        first = end;
    }
    // Each distinct keys and root, once.
    let mut starts: Vec<usize> = (0..roots.count.0).collect();
    starts.sort_by_key(|row| of(&root_cells, *row));
    starts.dedup_by_key(|row| of(&root_cells, *row));
    let mut visits = Vec::new();
    for root in starts {
        let keys = keys_of(&root_cells, root);
        let low = sorted.partition_point(|row| keys_of(&edge_cells, *row) < keys);
        let high = sorted.partition_point(|row| keys_of(&edge_cells, *row) <= keys);
        let own = walk_edges.get(low..high).unwrap_or_default();
        let entity = of(&root_cells, root).last().map_or(0, |cell| cell.0);
        visits.clear();
        walked(own, WalkEntity(entity), &mut visits);
        for (ordinal, visit) in visits.iter().enumerate() {
            let Some(edge) = sorted.get(low.saturating_add(visit.0)).copied() else {
                continue;
            };
            out.cells.extend(of(&root_cells, root));
            out.cells
                .push(of(&edge_cells, edge).last().copied().unwrap_or(CellWord(0)));
            out.cells.push(cell_at(destinations, Count(edge)));
            out.cells.push(cell_at(identities, Count(edge)));
            out.cells
                .push(CellWord(u64::try_from(ordinal).unwrap_or(u64::MAX)));
            out.count = Count(out.count.0.saturating_add(1));
        }
    }
    Ok(())
}
