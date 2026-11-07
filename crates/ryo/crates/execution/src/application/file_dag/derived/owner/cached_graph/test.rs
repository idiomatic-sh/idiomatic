//! A graph demand reaches the cache only through the owner of a prepared file; before
//! preparation and after retirement it is refused, never traversed by another route.
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

const SOURCE: Input = Input::Relation(Relation::Function);
const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];

#[test]
fn an_unprepared_owner_refuses_graph_demand_with_a_typed_error() {
    let owner = RelationOwner::new(
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        crate::application::file_dag::testing::program_derived_port(),
    );
    assert!(matches!(
        owner.derived.graph_cache(),
        Err(OperatingError::NotRun)
    ));
}

#[test]
fn a_retired_owner_refuses_graph_demand_instead_of_traversing() {
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    assert!(matches!(fixture.cache_ready(), Ok(())));
    fixture.retire();
    assert!(matches!(
        fixture.cache_ready(),
        Err(OperatingError::NotRun)
    ));
}

#[test]
fn a_prepared_owner_answers_graph_demand_through_its_cache() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    assert!(matches!(fixture.cache_ready(), Ok(())));
    let graph = fixture
        .acquire(SOURCE, &rows, ENDPOINTS, &[])
        .unwrap()
        .expect("the owner route answers");
    assert_eq!(graph.graph().vertices().len(), 3);
    assert_eq!(fixture.snapshot().counts.builds, 1);
}
