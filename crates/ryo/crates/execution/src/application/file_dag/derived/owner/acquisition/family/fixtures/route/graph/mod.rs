//! The graph entry: adjacency and membership acquired through an admitted graph input.
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::route::exit::{
    Exit, released, released_at_exit,
};
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use crate::domain::derived_views::types::ViewRequest;
use crate::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

pub(super) fn graph(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let mut floor = 0;
    let result = through_graph(kind, exit, &fixture, &mut floor);
    let at_exit = fixture.snapshot();
    released_at_exit(
        kind,
        exit,
        (
            at_exit.required_bytes,
            at_exit.optional_bytes,
            at_exit.unique_bytes,
        ),
        floor,
    );
    fixture.clear();
    let state = fixture.snapshot();
    released(
        kind,
        exit,
        (
            state.required_bytes,
            state.optional_bytes,
            state.unique_bytes,
        ),
    );
    result
}
fn through_graph(
    kind: DerivedKind,
    exit: Exit,
    fixture: &GraphFixture,
    floor: &mut usize,
) -> Result<DerivedKind, OperatingError> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let domain_columns = [FactColumn(vec![3]), FactColumn(vec![0])];
    let domain_rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &domain_columns,
    });
    let source = Input::Relation(Relation::Function);
    let edges = [ColumnIndex(0), ColumnIndex(1)];
    let membership = kind == DerivedKind::Membership;
    *floor = fixture.source_floor(Input::Relation(Relation::Function), &rows)?;
    if membership {
        *floor = fixture.source_floor(Input::Relation(Relation::Identifier), &domain_rows)?;
    }
    let graph = fixture
        .acquire(source, &rows, edges, &[])?
        .ok_or(OperatingError::WrongOperation)?;
    let domain = || {
        fixture
            .domain(
                &graph,
                Input::Relation(Relation::Identifier),
                &domain_rows,
                ColumnIndex(0),
            )?
            .ok_or(OperatingError::WrongOperation)
    };
    match exit {
        Exit::Cancelled => {
            if membership {
                let _withdrawn = domain()?;
            }
            Ok(kind)
        }
        Exit::Succeeds => {
            if membership {
                drop(domain()?);
            }
            Ok(kind)
        }
        Exit::Errors => {
            // The entry refuses a demand for the other family while the generation is live; the
            // leases it holds are released by the exit, not by retiring the generation.
            let _held = if membership { Some(domain()?) } else { None };
            let wrong = fixture.acquire_wrong(
                source,
                &rows,
                ViewRequest::Membership {
                    column: ColumnIndex(0),
                },
            );
            assert!(
                wrong.is_err(),
                "{kind:?}: a demand for another family is refused"
            );
            wrong.map(|_| kind)
        }
    }
}
