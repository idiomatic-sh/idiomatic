
use crate::domain::table_model::types::representation_input::value::RepresentationInput as BorrowedRows;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::push_row;

fn scope() -> std::rc::Rc<crate::application::file_dag::relations::RelationOwner> {
    std::rc::Rc::new(crate::application::file_dag::relations::RelationOwner::new(
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        crate::application::file_dag::testing::program_derived_port(),
    ))
}

fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}

#[test]
fn interleaved_partitions_keep_one_reused_adjacency() {
    let partitions = 2048;
    let mut input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for (from, to) in [(1, 2), (2, 3), (1, 4)] {
        for partition in 0..partitions {
            push_row(&mut input, &[number(from), number(to), number(partition)]);
        }
    }
    let scope = scope();
    let publication = scope.computed(input.clone()).unwrap();
    let rows = scope
        .resolve(publication.as_ref(), &|_| {
            Err(crate::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let mut cursor = super::schemas::prepare(
        &publication,
        BorrowedRows::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    let mut seen = 0;
    let mut maximum_live = 0;
    let mut maximum_reserved = 0;
    while cursor.advance().unwrap() {
        let part = cursor.current().unwrap();
        assert_eq!(part.table().count.0, 3);
        assert_eq!(part.table().cells.len(), 6);
        assert_eq!(part.key().len(), 1);
        assert_eq!(part.sources(), vec![number(1), number(2)]);
        seen += 1;
        maximum_live = maximum_live.max(cursor.working_cells());
        maximum_reserved = maximum_reserved.max(cursor.working_capacity());
    }
    assert_eq!(seen, partitions as usize);
    assert!(
        maximum_live <= 16,
        "all owned Value payload, including key and sources: {maximum_live}"
    );
    assert!(
        maximum_reserved < 64,
        "retained Value capacity: {maximum_reserved}"
    );
    // Ordering metadata scales with row IDs, not the number of partition graphs.
    assert!(cursor.ordering_bytes() < input.count.0 * 64);
    assert!(!cursor.advance().unwrap());
}

#[test]
fn separate_keys_never_share_edges() {
    let mut input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for (from, to, key) in [(1, 2, 5), (9, 10, 7), (2, 3, 5), (10, 11, 7)] {
        push_row(&mut input, &[number(from), number(to), number(key)]);
    }
    let scope = scope();
    let publication = scope.computed(input.clone()).unwrap();
    let rows = scope
        .resolve(publication.as_ref(), &|_| {
            Err(crate::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let mut cursor = super::schemas::prepare(
        &publication,
        BorrowedRows::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    let mut observed = Vec::new();
    while cursor.advance().unwrap() {
        observed.push((
            cursor.current().unwrap().key().to_vec(),
            cursor.current().unwrap().table().cells.clone(),
        ));
    }
    assert_eq!(observed.len(), 2);
    assert!(observed.contains(&(
        vec![number(5)],
        vec![number(1), number(2), number(2), number(3)]
    )));
    assert!(observed.contains(&(
        vec![number(7)],
        vec![number(9), number(10), number(10), number(11)]
    )));
}

#[test]
fn advancing_releases_only_the_previous_partition_lease() {
    let mut input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for (from, to, key) in [(1, 2, 5), (3, 4, 7)] {
        push_row(&mut input, &[number(from), number(to), number(key)]);
    }
    let scope = scope();
    let publication = scope.computed(input.clone()).unwrap();
    let rows = scope
        .resolve(publication.as_ref(), &|_| {
            Err(crate::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let input_charge = scope.derived().publication_ownership_bytes().0;
    assert!(
        input_charge > 0,
        "the actual input publication remains live"
    );
    let mut cursor = super::schemas::prepare(
        &publication,
        BorrowedRows::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    assert!(cursor.advance().unwrap());
    let owner = std::rc::Rc::clone(&scope);
    let first_backing =
        crate::application::file_dag::derived::owner::fixtures::resources::weak_storage(
            cursor.current.as_ref().unwrap(),
        );
    let first_charge = owner.derived().publication_ownership_bytes().0;
    assert!(first_charge > input_charge);
    assert!(cursor.advance().unwrap());
    assert!(
        first_backing.upgrade().is_none(),
        "advance releases the previous physical backing"
    );
    let second_backing =
        crate::application::file_dag::derived::owner::fixtures::resources::weak_storage(
            cursor.current.as_ref().unwrap(),
        );
    assert_eq!(
        owner.derived().publication_ownership_bytes().0,
        first_charge,
        "advancing releases the preceding equal-sized partition charge"
    );
    let transferred = cursor.take_current().unwrap();
    let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(part) =
        transferred.view().unwrap()
    else {
        panic!("closure partition")
    };
    assert_eq!(part.key(), &[number(7)]);
    assert!(cursor.current.is_none());
    drop(cursor);
    assert!(
        second_backing.upgrade().is_some(),
        "transferred view retains the physical backing"
    );
    assert!(
        owner.derived().publication_ownership_bytes().0 > 0,
        "the transferred consumer still owns its mandatory charge"
    );
    drop(transferred);
    assert!(
        second_backing.upgrade().is_none(),
        "final consumer releases the physical backing"
    );
    assert_eq!(owner.derived().publication_ownership_bytes().0, input_charge);
    drop(rows);
    drop(publication);
    assert_eq!(owner.derived().publication_ownership_bytes().0, 0);
}

#[test]
fn empty_order_and_wrong_prepared_order_are_refused_precisely() {
    let input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    let scope = scope();
    let publication = scope.computed(input.clone()).unwrap();
    let rows = scope
        .resolve(publication.as_ref(), &|_| {
            Err(crate::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let cursor = super::schemas::prepare(
        &publication,
        BorrowedRows::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    let mut empty = cursor;
    assert!(matches!(
        empty.current(),
        Err(crate::domain::operating::errors::OperatingError::NotRun)
    ));
    assert!(!empty.advance().unwrap());
    assert!(matches!(
        empty.current(),
        Err(crate::domain::operating::errors::OperatingError::NotRun)
    ));
    assert!(matches!(
        empty.take_current(),
        Err(crate::domain::operating::errors::OperatingError::NotRun)
    ));
    let wrong_plan = std::sync::Arc::new((*empty.partition_plan).clone());
    assert!(matches!(
        super::schemas::from_prepared(
            empty.proof.clone(),
            wrong_plan,
            std::sync::Arc::clone(&empty.closure_plan),
            empty.ordered.clone(),
            std::rc::Rc::clone(&empty.owner),
        ),
        Err(crate::domain::operating::errors::OperatingError::WrongOperation)
    ));
}

use ryo_values::domain::rows::types::Table;
