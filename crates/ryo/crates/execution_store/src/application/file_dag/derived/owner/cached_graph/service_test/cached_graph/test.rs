//! A graph demand reaches the cache only through the owner of a prepared file; before
//! preparation and after retirement it is refused, never traversed by another route.
use super::fixtures::{ENDPOINTS, SOURCE, graph_of};
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use crate::application::file_dag::relations::RelationOwner;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};

#[test]
fn an_unprepared_owner_refuses_graph_demand_with_a_typed_error() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    );
    assert!(matches!(
        owner.derived().graph_cache(),
        Err(OperatingError::NotRun)
    ));
}

#[test]
fn a_retired_owner_refuses_graph_demand_instead_of_traversing() {
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    assert!(matches!(fixture.cache_ready(), Ok(())));
    fixture.retire();
    assert!(matches!(fixture.cache_ready(), Err(OperatingError::NotRun)));
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

#[test]
fn a_project_owner_declines_the_graph_so_its_axis_walks_its_tables() {
    let owner = std::rc::Rc::new(RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    ));
    // Before it serves the project, an owner with no cache refuses the graph, as a file owner before its preparation does.
    assert!(matches!(graph_of(&owner), Err(OperatingError::NotRun)));
    assert!(matches!(owner.derived().serve_project(), Ok(())));
    assert!(matches!(graph_of(&owner), Ok(false)));
    // A retirement of its population leaves it the project's.
    owner.release_population();
    assert!(matches!(graph_of(&owner), Ok(false)));
}

#[test]
fn a_file_owner_cannot_serve_the_project() {
    let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
    assert!(matches!(
        fixture.owner.derived().serve_project(),
        Err(OperatingError::WrongOperation)
    ));
}
