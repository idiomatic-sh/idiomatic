//! The graph entry: adjacency and membership acquired through an admitted graph input.
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::route::exit::{Exit, released};
use crate::application::file_dag::derived::owner::{fixtures::graph::GraphFixture, index::service::LIMIT};
use crate::domain::{
    operating::errors::OperatingError, stored_graph::types::GraphWalk, table_model::types::Rows,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

pub(super) fn graph(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let result = through_graph(kind, exit, &fixture);
    fixture.clear();
    let state = fixture.snapshot();
    released(
        kind,
        exit,
        (state.required_bytes, state.optional_bytes, state.unique_bytes),
    );
    result
}
fn through_graph(
    kind: DerivedKind,
    exit: Exit,
    fixture: &GraphFixture,
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
    let graph = fixture
        .acquire(source, &rows, edges, &[])?
        .ok_or(OperatingError::WrongOperation)?;
    let membership = kind == DerivedKind::Membership;
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
        Exit::Cancelled => Ok(kind),
        Exit::Succeeds => {
            if membership {
                drop(domain()?);
            }
            Ok(kind)
        }
        Exit::Errors => {
            let held = if membership { Some(domain()?) } else { None };
            fixture.clear();
            let mut walk = GraphWalk::default();
            let refused = match &held {
                Some(domain) => fixture
                    .nearest(
                        &graph,
                        domain,
                        0,
                        crate::domain::stored_graph::types::Orientation::Forward,
                        &mut vec![],
                        &mut walk,
                    )
                    .map(|_| ()),
                None => fixture
                    .reach(&graph, &[], Value::Number(Integer(1)), &mut walk)
                    .map(|_| ()),
            };
            refused.map(|()| kind)
        }
    }
}
