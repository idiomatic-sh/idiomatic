//! Boundary values for the actual borrowed graph preparation and traversal APIs.
use super::{
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
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
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
    fn count(&self) -> ryo_values::domain::rows::types::RowCount {
        self.0.count
    }
    fn cell(
        &self,
        row: ryo_values::domain::rows::types::RowIndex,
        column: ColumnIndex,
    ) -> Option<Cell> {
        row_in(self.0, row)?
            .get(usize::try_from(column.0).expect("a column index fits in usize"))
            .copied()
    }
}
#[derive(Default)]
struct Texts {
    canonical: Vec<String>,
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
fn request(keys: bool) -> DerivedRequest {
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
fn with_graph<T>(
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
fn numbers(width: usize, rows: &[&[i64]]) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: rows
            .iter()
            .flat_map(|row| row.iter().map(|n| Cell::Number(Integer(*n))))
            .collect(),
        count: RowCount(rows.len()),
    }
}
fn ordinal_overflow_follows_the_last_representable_visit() -> Value {
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
        let first = next_ordered(&edges, graph, &[], &mut walk)
            .unwrap()
            .unwrap();
        assert_eq!(first.ordinal, Integer(i64::MAX));
        assert!(
            !walk.visited.contains(&Cell::Number(Integer(2))),
            "yield before destination expansion"
        );
        assert_eq!(
            next_ordered(&edges, graph, &[], &mut walk),
            Err(OperatingError::DfsOrdinalOverflow)
        );
    })
    .unwrap();
    1
}
fn nonnumeric_order_cell_is_refused_by_actual_derived_preparation() -> Value {
    let roots = numbers(1, &[&[1]]);
    let mut edges = numbers(4, &[&[1, 2, 0, 12]]);
    edges.cells[2] = Cell::Flag(ryo_relations::domain::relations::types::Flag::Raised);
    assert_eq!(
        with_graph(&roots, &edges, false, &mut Texts::default(), |_| ()),
        Err(DerivedFault::ShapeMismatch)
    );
    1
}
fn equal_retained_hashes_use_exact_text_and_preserve_original_cells() -> Value {
    let retained = |place| {
        Cell::Name(Name {
            hash: TextHash(99),
            place: PlaceNumber(place),
        })
    };
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
                &row_in(&graph.roots, RowIndex(row)).unwrap()[..2],
                row_in(&roots, RowIndex(row)).unwrap()
            );
        }
        for row in 0..edges.count.0 {
            assert_eq!(
                &row_in(&graph.edges, RowIndex(row)).unwrap()[..5],
                row_in(&edges, RowIndex(row)).unwrap()
            );
            assert_eq!(
                cell(row_in(&graph.edges, RowIndex(row)).unwrap(), ColumnIndex(3)).unwrap(),
                retained(4)
            );
        }
        assert_eq!(
            cell(row_in(&graph.roots, RowIndex(2)).unwrap(), ColumnIndex(0)).unwrap(),
            retained(1)
        );
        let columns = &graph.edge_columns[..graph.edge_columns.len() - 1];
        let walked = Edges {
            table: &graph.edges,
            order: &graph.edge_index,
            from: columns,
            to: *graph.edge_columns.last().unwrap(),
        };
        let mut total_expanded = 0;
        let mut total_examined = 0;
        for (root, expected) in [
            (
                a,
                vec![
                    OrderedVisit {
                        edge: RowIndex(0),
                        ordinal: Integer(0),
                    },
                    OrderedVisit {
                        edge: RowIndex(1),
                        ordinal: Integer(1),
                    },
                    OrderedVisit {
                        edge: RowIndex(2),
                        ordinal: Integer(2),
                    },
                ],
            ),
            (
                b,
                vec![
                    OrderedVisit {
                        edge: RowIndex(1),
                        ordinal: Integer(0),
                    },
                    OrderedVisit {
                        edge: RowIndex(2),
                        ordinal: Integer(1),
                    },
                    OrderedVisit {
                        edge: RowIndex(0),
                        ordinal: Integer(2),
                    },
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
            total_expanded += walk.expanded.0;
            total_examined += walk.examined.0;
        }
        assert_eq!(total_expanded, 6);
        assert_eq!(total_examined, 6);
    })
    .unwrap();
    1
}

/// A scenario: what a row asks of the part.
type Input = fn() -> Value;
/// What a scenario answers: a yes as one.
type Value = usize;
/// A scenario that checks its own claims never fails as a whole.
type Failure = std::convert::Infallible;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordinal_overflow_follows_the_last_representable_visit"),
        word: Word::Succeeds,
        input: || ordinal_overflow_follows_the_last_representable_visit,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("nonnumeric_order_cell_is_refused_by_actual_derived_preparation"),
        word: Word::Succeeds,
        input: || nonnumeric_order_cell_is_refused_by_actual_derived_preparation,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("equal_retained_hashes_use_exact_text_and_preserve_original_cells"),
        word: Word::Succeeds,
        input: || equal_retained_hashes_use_exact_text_and_preserve_original_cells,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
