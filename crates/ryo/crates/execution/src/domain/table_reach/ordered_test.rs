//! Boundary values for the actual borrowed graph preparation and traversal APIs.
use super::{
    service::{begin_ordered, next_ordered},
    types::{Edges, OrderedWalk},
};
use crate::domain::operating::errors::OperatingError;
use ryo_derived::{
    domain::contract::types::{
        BuildOutcome, BuildWorkspace, BulkRows, CanonicalTexts, DerivedFault, DerivedInputs,
        DerivedView, PreparedDfs, RowCells, ValueTexts, ViewRequest,
    },
    inbound::use_cases::derived::{interface::DerivedUseCase, use_case::DerivedUseCaseImpl},
};
use ryo_relations::{
    domain::relations::types::{CellKind, Integer, Name, PlaceNumber, TextHash, Value},
    domain::terms::types::Symbol,
};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::{
        service::{cell, push_row, row_in},
        types::{RowCount, RowIndex, Table},
    },
};
struct Rows<'a>(&'a Table);
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
    fn count(&self) -> usize {
        self.0.count.0
    }
    fn cell(&self, row: usize, column: ColumnIndex) -> Option<Value> {
        row_in(self.0, RowIndex(row))?
            .get(column.0 as usize)
            .copied()
    }
}
#[derive(Default)]
struct Texts {
    canonical: Vec<String>,
}
impl ValueTexts for Texts {
    fn text(&self, value: Value) -> Option<&str> {
        match value {
            Value::Name(name) => ["a", "b", "c", "ctx", "edge"]
                .get(name.place.0 as usize)
                .copied(),
            Value::Symbol(symbol) => self.canonical.get(symbol.0 as usize).map(String::as_str),
            _ => None,
        }
    }
}
impl CanonicalTexts for Texts {
    fn intern(&mut self, text: &str) -> Result<Value, DerivedFault> {
        let at = if let Some(at) = self.canonical.iter().position(|value| value == text) {
            at
        } else {
            self.canonical.push(text.into());
            self.canonical.len() - 1
        };
        Ok(Value::Symbol(Symbol(
            u32::try_from(at).map_err(|_| DerivedFault::ShapeMismatch)?,
        )))
    }
}
fn request(keys: bool) -> ViewRequest {
    ViewRequest::OrderedDfs {
        root: ColumnIndex(0),
        root_keys: if keys { vec![ColumnIndex(1)] } else { vec![] },
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        order: ColumnIndex(2),
        edge: ColumnIndex(3),
        edge_keys: if keys { vec![ColumnIndex(4)] } else { vec![] },
    }
}
fn with_graph<T>(
    roots: &Table,
    edges: &Table,
    keys: bool,
    texts: &mut Texts,
    check: impl FnOnce(&PreparedDfs) -> T,
) -> Result<T, DerivedFault> {
    let use_case = DerivedUseCaseImpl::new();
    let prepared = use_case.prepare(
        request(keys),
        vec![roots.kinds.clone(), edges.kinds.clone()],
    );
    let BuildOutcome::Built(payload) = use_case.build(
        &prepared,
        DerivedInputs::Dfs {
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
fn numbers(width: usize, rows: &[&[i64]]) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: rows
            .iter()
            .flat_map(|row| row.iter().map(|n| Value::Number(Integer(*n))))
            .collect(),
        count: RowCount(rows.len()),
    }
}
#[test]
fn ordinal_overflow_follows_the_last_representable_visit() {
    let roots = numbers(1, &[&[1]]);
    let edges = numbers(4, &[&[1, 2, 0, 12], &[1, 3, 1, 13]]);
    with_graph(&roots, &edges, false, &mut Texts::default(), |graph| {
        let columns = &graph.edge_columns()[..graph.edge_columns().len() - 1];
        let target = *graph.edge_columns().last().unwrap();
        let edges = Edges {
            table: graph.edges(),
            order: graph.edge_index(),
            from: columns,
            to: target,
        };
        let mut walk = OrderedWalk::default();
        begin_ordered(&edges, graph, &[], Value::Number(Integer(1)), &mut walk);
        walk.ordinal = Some(i64::MAX);
        let first = next_ordered(&edges, graph, &[], &mut walk)
            .unwrap()
            .unwrap();
        assert_eq!(first.1, Integer(i64::MAX));
        assert!(
            !walk.visited.contains(&Value::Number(Integer(2))),
            "yield before destination expansion"
        );
        assert_eq!(
            next_ordered(&edges, graph, &[], &mut walk),
            Err(OperatingError::DfsOrdinalOverflow)
        );
    })
    .unwrap();
}
#[test]
fn nonnumeric_order_cell_is_refused_by_actual_derived_preparation() {
    let roots = numbers(1, &[&[1]]);
    let mut edges = numbers(4, &[&[1, 2, 0, 12]]);
    edges.cells[2] = Value::Flag(ryo_relations::domain::relations::types::Flag::Raised);
    assert_eq!(
        with_graph(&roots, &edges, false, &mut Texts::default(), |_| ()),
        Err(DerivedFault::ShapeMismatch)
    );
}
#[test]
fn equal_retained_hashes_use_exact_text_and_preserve_original_cells() {
    let retained = |place| {
        Value::Name(Name {
            hash: TextHash(99),
            place: PlaceNumber(place),
        })
    };
    let a = Value::Symbol(Symbol(0));
    let b = Value::Symbol(Symbol(1));
    let ctx = Value::Symbol(Symbol(2));
    let zero = Value::Number(Integer(0));
    let one = Value::Number(Integer(1));
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
        let canonical = graph.root_columns();
        let canonical_key = |row| {
            canonical
                .iter()
                .map(|column| cell(row_in(graph.roots(), RowIndex(row)).unwrap(), *column).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            canonical_key(0),
            canonical_key(1),
            "source Symbol and retained Name of the same text share an exact root key"
        );
        assert_ne!(
            canonical_key(0),
            canonical_key(2),
            "equal retained hashes do not merge different text"
        );
        for row in 0..roots.count.0 {
            assert_eq!(
                &row_in(graph.roots(), RowIndex(row)).unwrap()[..2],
                row_in(&roots, RowIndex(row)).unwrap()
            );
        }
        for row in 0..edges.count.0 {
            assert_eq!(
                &row_in(graph.edges(), RowIndex(row)).unwrap()[..5],
                row_in(&edges, RowIndex(row)).unwrap()
            );
            assert_eq!(
                cell(
                    row_in(graph.edges(), RowIndex(row)).unwrap(),
                    ColumnIndex(3)
                )
                .unwrap(),
                retained(4)
            );
        }
        assert_eq!(
            cell(row_in(graph.roots(), RowIndex(2)).unwrap(), ColumnIndex(0)).unwrap(),
            retained(1)
        );
        let columns = &graph.edge_columns()[..graph.edge_columns().len() - 1];
        let walked = Edges {
            table: graph.edges(),
            order: graph.edge_index(),
            from: columns,
            to: *graph.edge_columns().last().unwrap(),
        };
        let mut total_expanded = 0;
        let mut total_examined = 0;
        for (root, expected) in [
            (
                a,
                vec![
                    (RowIndex(0), Integer(0)),
                    (RowIndex(1), Integer(1)),
                    (RowIndex(2), Integer(2)),
                ],
            ),
            (
                b,
                vec![
                    (RowIndex(1), Integer(0)),
                    (RowIndex(2), Integer(1)),
                    (RowIndex(0), Integer(2)),
                ],
            ),
        ] {
            let mut walk = OrderedWalk::default();
            begin_ordered(&walked, graph, &[ctx], root, &mut walk);
            let mut actual = Vec::new();
            while let Some(visit) = next_ordered(&walked, graph, &[ctx], &mut walk).unwrap() {
                actual.push(visit);
            }
            assert_eq!(actual, expected);
            total_expanded += walk.expanded;
            total_examined += walk.examined;
        }
        assert_eq!(total_expanded, 6);
        assert_eq!(total_examined, 6);
    })
    .unwrap();
}
