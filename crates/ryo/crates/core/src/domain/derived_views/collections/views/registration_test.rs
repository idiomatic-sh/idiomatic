//! Actual registered families prepare their own requests and refuse every other family.
use super::registration::registered;
use crate::domain::{
    derived_views::{service::kind, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    operator::contract::resources::DfsResources,
    stored_selection::types::Selector,
    table_model::types::SelectionIdentity,
};
use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

fn request(family: DerivedKind) -> ViewRequest {
    let column = ColumnIndex(0);
    match family {
        DerivedKind::Equality => ViewRequest::Equality {
            columns: vec![column],
        },
        DerivedKind::PredicateMatches => ViewRequest::PredicateMatches(Arc::new(Selector {
            relation: Relation::Function,
            identity: Arc::new(SelectionIdentity {
                comparisons: vec![],
                bindings: vec![],
            }),
            columns: vec![column],
            distinct: false,
        })),
        DerivedKind::CandidatePostings => ViewRequest::CandidatePostings {
            column,
            literals: vec![],
        },
        DerivedKind::DistinctRepresentatives => ViewRequest::DistinctRepresentatives {
            columns: vec![column],
        },
        DerivedKind::Adjacency => ViewRequest::Adjacency {
            endpoints: [column, ColumnIndex(1)],
            keys: vec![],
        },
        DerivedKind::Membership => ViewRequest::Membership { column },
        DerivedKind::PartitionOrder => ViewRequest::PartitionOrder { keys: vec![column] },
        DerivedKind::ClosureAdjacency => ViewRequest::ClosureAdjacency {
            source: column,
            destination: ColumnIndex(1),
        },
        DerivedKind::CycleAdjacency => ViewRequest::CycleAdjacency {
            source: column,
            destination: ColumnIndex(1),
        },
        DerivedKind::OrderedKeys => ViewRequest::OrderedKeys {
            left_on: vec![column],
            right_on: vec![column],
            left_order: ColumnIndex(1),
            right_order: ColumnIndex(1),
        },
        DerivedKind::OrderedDfs => ViewRequest::OrderedDfs(DfsResources {
            root: column,
            source: column,
            destination: ColumnIndex(1),
            order: ColumnIndex(2),
            edge: ColumnIndex(3),
            root_keys: vec![],
            edge_keys: vec![],
        }),
        DerivedKind::NativeLookup => ViewRequest::NativeLookup {
            left_on: vec![column],
            right_on: vec![column],
            view: vec![column],
        },
    }
}

#[test]
fn every_requested_family_reaches_its_actual_prepared_family() {
    for &family in DerivedKind::ALL {
        let mut kinds = [CellKind::Number; 4];
        if family == DerivedKind::CandidatePostings {
            kinds[0] = CellKind::Symbol;
        }
        let schemas = [
            InputSchema {
                kinds: &kinds,
                tree: None,
            },
            InputSchema {
                kinds: &kinds,
                tree: None,
            },
        ];
        let requested = request(family);
        assert_eq!(kind(&requested), family);
        let prepared = registered(family)
            .prepare(&requested, &schemas)
            .expect("real registered prepare");
        assert_eq!(prepared.kind, family);
        assert_eq!(kind(&prepared.request), family);
        assert_eq!(prepared.schemas, vec![kinds.to_vec(), kinds.to_vec()]);
        assert!(prepared.metadata.is_ok());
        assert!(matches!(
            registered(family).view(&prepared, &DerivedStorage::Unbuilt),
            Err(OperatingError::NotRun)
        ));
        for &other in DerivedKind::ALL {
            if family != other {
                assert!(
                    matches!(
                        registered(other).prepare(&requested, &schemas),
                        Err(OperatingError::WrongOperation)
                    ),
                    "wrong family {other:?} accepted {family:?}"
                );
            }
        }
    }
}

#[test]
fn missing_static_metadata_is_deferred_without_reading_or_building() {
    for &family in DerivedKind::ALL {
        let prepared = registered(family)
            .prepare(&request(family), &[])
            .expect("metadata is deferred");
        assert!(matches!(
            prepared.metadata,
            Err(OperatingError::MissingInput { position: 1 })
        ));
        assert!(matches!(
            registered(family).view(&prepared, &DerivedStorage::Unbuilt),
            Err(OperatingError::MissingInput { position: 1 })
        ));
    }
}

#[test]
fn borrowed_positions_preserve_bag_order_and_cannot_change_family() {
    let kinds = [CellKind::Number];
    let schema = [InputSchema {
        kinds: &kinds,
        tree: None,
    }];
    let predicate = registered(DerivedKind::PredicateMatches)
        .prepare(&request(DerivedKind::PredicateMatches), &schema)
        .unwrap();
    let distinct = registered(DerivedKind::DistinctRepresentatives)
        .prepare(&request(DerivedKind::DistinctRepresentatives), &schema)
        .unwrap();
    let storage = DerivedStorage::RowIds {
        kind: DerivedKind::PredicateMatches,
        rows: Arc::new(vec![2, 2, 0]),
    };
    let DerivedView::RowIds(rows) = registered(DerivedKind::PredicateMatches)
        .view(&predicate, &storage)
        .unwrap()
    else {
        panic!("wrong view");
    };
    assert_eq!(rows, &[2, 2, 0]);
    let DerivedStorage::RowIds { rows: owned, .. } = &storage else {
        unreachable!()
    };
    assert_eq!(
        rows.as_ptr(),
        owned.as_ptr(),
        "view must borrow built storage"
    );
    assert!(matches!(
        registered(DerivedKind::DistinctRepresentatives).view(&distinct, &storage),
        Err(OperatingError::WrongOperation)
    ));
    assert!(matches!(
        registered(DerivedKind::PredicateMatches).view(&distinct, &storage),
        Err(OperatingError::WrongOperation)
    ));
}
