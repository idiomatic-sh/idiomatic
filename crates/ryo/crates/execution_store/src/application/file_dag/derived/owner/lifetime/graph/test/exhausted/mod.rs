//! An exhausted domain identity keeps local membership without retained duplicates.
use super::*;
#[test]
fn exhausted_domain_identity_uses_local_membership_without_retained_duplicates() {
    let fixture = GraphFixture::with_identity(
        &[Relation::Function, Relation::Identifier],
        &[],
        crate::application::file_dag::derived::owner::index::service::LIMIT,
        true,
        1,
    );
    let edges = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let graph = fixture
        .acquire(SOURCE, &rows(&edges), ENDPOINTS, &[])
        .unwrap()
        .unwrap();
    assert_eq!(graph.sharing(), GraphSharing::Shared);
    let domain_columns = [FactColumn(vec![2, 3]), FactColumn(vec![0, 0])];
    let input = rows(&domain_columns);
    let mut walk = GraphWalk::default();
    let mut local = Vec::new();
    for _ in 0..2 {
        let domain = fixture
            .domain(
                &graph,
                Input::Relation(Relation::Identifier),
                &input,
                ColumnIndex(0),
            )
            .unwrap()
            .unwrap();
        assert!(
            domain.raw_domain().identity.is_none(),
            "an exhausted source publication cannot grant stable membership identity"
        );
        let nearest = fixture
            .nearest(
                &graph,
                &domain,
                graph
                    .graph()
                    .vertex(
                        ryo_derived::domain::vertices::types::PartitionId(0),
                        number(Integer(1)),
                    )
                    .unwrap()
                    .0,
                Orientation::Forward,
                &mut local,
                &mut walk,
            )
            .unwrap();
        assert_eq!(
            nearest.map(|at| graph.graph().vertices()[at as usize]),
            Some(number(Integer(2)))
        );
    }
    let observed = fixture.snapshot();
    assert_eq!(observed.graphs, 1);
    assert_eq!(observed.domains, 0);
    assert_eq!(observed.nearest, 0);
    assert_eq!(observed.counts.builds, 1);
}
