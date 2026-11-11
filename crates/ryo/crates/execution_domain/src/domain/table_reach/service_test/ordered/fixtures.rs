//! The support of the `ordered` cases.
use super::super::super::{
    service::{begin_ordered, next_ordered},
    types::{Edges, OrderedVisit, OrderedWalk},
};
use crate::domain::operating::errors::OperatingError;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::domain::row_cells::types::BulkRows;
use ryo_derived::domain::row_cells::types::CanonicalTexts;
use ryo_derived::domain::row_cells::types::RowCells;
use ryo_derived::domain::row_cells::types::ValueTexts;
use ryo_relations::{
    domain::relations::types::{CellKind, Integer, Name, PlaceNumber, TextHash, Value as Cell},
    domain::terms::types::Symbol,
};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::{
        service::{cell, push_row, row_in},
        types::{RowCount, RowIndex, Table},
    },
};
use std::borrow::Cow;

pub(super) struct Rows<'a>(pub(super) &'a Table);

impl RowCells for Rows<'_> {
    fn bulk(&self) -> Option<BulkRows<'_>> {
        Some(BulkRows::Computed(self.0))
    }
    fn validate(&self) -> Result<(), DerivedFault> {
        if self.0.count.0.checked_mul(self.0.kinds.len()) == Some(self.0.cells.len()) {
            Ok(())
        } else {
            Err(DerivedFault::ShapeMismatch)
        }
    }
    fn kinds(&self) -> &[CellKind] {
        &self.0.kinds
    }
    fn count(&self) -> RowCount {
        self.0.count
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Cell> {
        row_in(self.0, row)?
            .get(usize::try_from(column.0).expect("a column index fits in usize"))
            .copied()
    }
}

#[derive(Default)]
pub(super) struct Texts {
    pub(super) canonical: Vec<String>,
}

impl ValueTexts for Texts {
    fn text(&self, value: Cell) -> Option<ryo_values::domain::row_texts::types::CellText<'_>> {
        match value {
            Cell::Name(name) => ["a", "b", "c", "ctx", "edge"]
                .get(usize::try_from(name.place.0).expect("a place number fits in usize"))
                .copied()
                .map(ryo_values::domain::row_texts::types::CellText),
            Cell::Symbol(symbol) => self
                .canonical
                .get(usize::try_from(symbol.0).expect("a symbol number fits in usize"))
                .map(|text| ryo_values::domain::row_texts::types::CellText(text.as_str())),
            Cell::Node(_) | Cell::File(_) | Cell::Span(_) | Cell::Number(_) | Cell::Flag(_) => None,
        }
    }
}

impl CanonicalTexts for Texts {
    fn intern(
        &mut self,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> Result<Cell, DerivedFault> {
        let at = if let Some(at) = self.canonical.iter().position(|value| value == text.0) {
            at
        } else {
            self.canonical.push(text.0.into());
            self.canonical.len() - 1
        };
        Ok(Cell::Symbol(Symbol(
            u32::try_from(at).map_err(|_| DerivedFault::ShapeMismatch)?,
        )))
    }
}

pub(super) fn request(keys: bool) -> DerivedRequest {
    DerivedRequest::OrderedDfs {
        root: ColumnIndex(0),
        root_keys: if keys { vec![ColumnIndex(1)] } else { vec![] },
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        order: ColumnIndex(2),
        edge: ColumnIndex(3),
        edge_keys: if keys { vec![ColumnIndex(4)] } else { vec![] },
    }
}

pub(super) fn with_graph<T>(
    roots: &Table,
    edges: &Table,
    keys: bool,
    texts: &mut Texts,
    check: impl FnOnce(&PreparedDfs) -> T,
) -> Result<T, DerivedFault> {
    let use_case = crate::domain::derived_views::fixtures::semantics();
    let prepared = use_case.prepare(
        request(keys),
        vec![roots.kinds.clone(), edges.kinds.clone()],
    );
    let BuildOutcome::Built(payload) = use_case.build(
        &prepared,
        BuildInputs::Dfs {
            roots: &Rows(roots),
            edges: &Rows(edges),
            texts,
        },
        &mut BuildWorkspace::default(),
        None,
    )?
    else {
        return Err(DerivedFault::WrongOperation);
    };
    let DerivedView::OrderedDfs(graph) = use_case.view(&prepared, &payload)? else {
        return Err(DerivedFault::WrongOperation);
    };
    Ok(check(graph))
}

pub(super) fn numbers(width: usize, rows: &[&[i64]]) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: rows
            .iter()
            .flat_map(|row| row.iter().map(|n| Cell::Number(Integer(*n))))
            .collect(),
        count: RowCount(rows.len()),
    }
}

/// The graph a row prepares and walks.
#[derive(Clone, Copy, Debug)]
pub(super) enum Graph {
    /// One root with two numbered edges, walked with its next ordinal at the last representable one.
    LastOrdinal,
    /// One edge whose order cell is a flag.
    FlagOrder,
    /// Roots and edges keyed by source symbols and by retained names, two of them with equal hashes over different
    /// texts, walked from `a` and from `b` in context `ctx`.
    RetainedNames,
}

/// What preparing and walking the graph observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// The walk at the last representable ordinal.
    LastOrdinal {
        /// The first visit.
        first: Result<Option<OrderedVisit>, OperatingError>,
        /// Whether the destination was expanded before the first visit was yielded.
        destination_expanded_at_yield: bool,
        /// The next visit.
        next: Result<Option<OrderedVisit>, OperatingError>,
    },
    /// The prepared graph and its walks.
    RetainedNames {
        /// Whether the source symbol and the retained name of the same text share a root key.
        same_text_shares_key: bool,
        /// Whether equal retained hashes of different texts share a root key.
        equal_hashes_share_key: bool,
        /// Whether every prepared root row starts with its original cells.
        roots_preserved: bool,
        /// Whether every prepared edge row starts with its original cells.
        edges_preserved: bool,
        /// Whether every prepared edge keeps its retained edge name.
        edge_names_kept: bool,
        /// The third root's first prepared cell.
        third_root: Cell,
        /// The visits from `a`.
        from_a: Cow<'static, [OrderedVisit]>,
        /// The visits from `b`.
        from_b: Cow<'static, [OrderedVisit]>,
        /// The vertices both walks expanded.
        expanded: usize,
        /// The edges both walks examined.
        examined: usize,
    },
    /// The graph prepared, and nothing was walked.
    Prepared,
}

/// A retained name of hash 99 at a place.
pub(super) const fn retained(place: u32) -> Cell {
    Cell::Name(Name {
        hash: TextHash(99),
        place: PlaceNumber(place),
    })
}

/// Prepares and walks a row's graph.
pub(super) fn walk(graph: Graph) -> Result<Observed, DerivedFault> {
    match graph {
        Graph::LastOrdinal => last_ordinal(),
        Graph::FlagOrder => flag_order(),
        Graph::RetainedNames => retained_names(),
    }
}

fn last_ordinal() -> Result<Observed, DerivedFault> {
    let roots = numbers(1, &[&[1]]);
    let edges = numbers(4, &[&[1, 2, 0, 12], &[1, 3, 1, 13]]);
    with_graph(&roots, &edges, false, &mut Texts::default(), |graph| {
        let columns = &graph.edge_columns[..graph.edge_columns.len() - 1];
        let target = *graph.edge_columns.last().unwrap();
        let edges = Edges {
            table: &graph.edges,
            order: &graph.edge_index,
            from: columns,
            to: target,
        };
        let mut walk = OrderedWalk::default();
        begin_ordered(&edges, graph, &[], Cell::Number(Integer(1)), &mut walk);
        walk.ordinal = Some(Integer(i64::MAX));
        let first = next_ordered(&edges, graph, &[], &mut walk);
        let destination_expanded_at_yield = walk.visited.contains(&Cell::Number(Integer(2)));
        let next = next_ordered(&edges, graph, &[], &mut walk);
        Observed::LastOrdinal {
            first,
            destination_expanded_at_yield,
            next,
        }
    })
}

fn flag_order() -> Result<Observed, DerivedFault> {
    let roots = numbers(1, &[&[1]]);
    let mut edges = numbers(4, &[&[1, 2, 0, 12]]);
    edges.cells[2] = Cell::Flag(ryo_relations::domain::relations::types::Flag::Raised);
    with_graph(&roots, &edges, false, &mut Texts::default(), |_| {
        Observed::Prepared
    })
}

fn retained_names() -> Result<Observed, DerivedFault> {
    let a = Cell::Symbol(Symbol(0));
    let b = Cell::Symbol(Symbol(1));
    let ctx = Cell::Symbol(Symbol(2));
    let zero = Cell::Number(Integer(0));
    let one = Cell::Number(Integer(1));
    let mut roots = Table {
        kinds: vec![CellKind::Symbol; 2],
        ..Table::default()
    };
    for row in [[a, ctx], [retained(0), retained(3)], [retained(1), ctx]] {
        push_row(&mut roots, &row);
    }
    let mut edges = Table {
        kinds: vec![
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number,
            CellKind::Symbol,
            CellKind::Symbol,
        ],
        ..Table::default()
    };
    for row in [
        [retained(0), b, zero, retained(4), retained(3)],
        [b, retained(2), zero, retained(4), ctx],
        [retained(1), retained(0), one, retained(4), ctx],
    ] {
        push_row(&mut edges, &row);
    }
    let mut texts = Texts {
        canonical: vec!["a".into(), "b".into(), "ctx".into()],
    };
    with_graph(&roots, &edges, true, &mut texts, |graph| {
        let canonical = graph.root_columns.clone();
        let canonical_key = |row| {
            canonical
                .iter()
                .map(|column| cell(row_in(&graph.roots, RowIndex(row)).unwrap(), *column).unwrap())
                .collect::<Vec<_>>()
        };
        let roots_preserved = (0..roots.count.0).all(|row| {
            row_in(&graph.roots, RowIndex(row)).map(|cells| &cells[..2])
                == row_in(&roots, RowIndex(row))
        });
        let edges_preserved = (0..edges.count.0).all(|row| {
            row_in(&graph.edges, RowIndex(row)).map(|cells| &cells[..5])
                == row_in(&edges, RowIndex(row))
        });
        let edge_names_kept = (0..edges.count.0).all(|row| {
            row_in(&graph.edges, RowIndex(row)).and_then(|cells| cell(cells, ColumnIndex(3)).ok())
                == Some(retained(4))
        });
        let third_root = cell(row_in(&graph.roots, RowIndex(2)).unwrap(), ColumnIndex(0)).unwrap();
        let columns = &graph.edge_columns[..graph.edge_columns.len() - 1];
        let walked = Edges {
            table: &graph.edges,
            order: &graph.edge_index,
            from: columns,
            to: *graph.edge_columns.last().unwrap(),
        };
        let mut expanded = 0;
        let mut examined = 0;
        let mut visits = [a, b].map(|root| {
            let mut walk = OrderedWalk::default();
            begin_ordered(&walked, graph, &[ctx], root, &mut walk);
            let mut actual = Vec::new();
            while let Ok(Some(visit)) = next_ordered(&walked, graph, &[ctx], &mut walk) {
                actual.push(visit);
            }
            expanded += walk.expanded.0;
            examined += walk.examined.0;
            actual
        });
        let from_b = std::mem::take(&mut visits[1]);
        let from_a = std::mem::take(&mut visits[0]);
        Observed::RetainedNames {
            same_text_shares_key: canonical_key(0) == canonical_key(1),
            equal_hashes_share_key: canonical_key(0) == canonical_key(2),
            roots_preserved,
            edges_preserved,
            edge_names_kept,
            third_root,
            from_a: Cow::Owned(from_a),
            from_b: Cow::Owned(from_b),
            expanded,
            examined,
        }
    })
}
