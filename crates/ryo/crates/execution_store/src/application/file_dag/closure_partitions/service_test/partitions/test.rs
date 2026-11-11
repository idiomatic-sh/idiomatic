//! The proofs of the partitioned closure cursor.
use super::super::super::fixtures::working_capacity;
use super::fixtures::{number, ordering_bytes, scope, working_cells};
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::push_row;

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
            Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let mut cursor = super::super::super::service::prepare(
        &crate::application::file_dag::relations::handle::service::Publication::of(
            publication.clone(),
        ),
        RepresentationInput::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    let mut seen = 0;
    let mut maximum_live = 0;
    let mut maximum_reserved = 0;
    while cursor.advance().unwrap()
        == crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced
    {
        let part = cursor.current().unwrap();
        assert_eq!(part.table.count.0, 3);
        assert_eq!(part.table.cells.len(), 6);
        assert_eq!(part.key.len(), 1);
        assert_eq!(part.sources, vec![number(1), number(2)]);
        seen += 1;
        maximum_live = maximum_live.max(working_cells(&cursor));
        maximum_reserved = maximum_reserved.max(working_capacity(&cursor));
    }
    assert_eq!(
        seen,
        usize::try_from(partitions).expect("the partition count fits in usize")
    );
    assert!(
        maximum_live <= 16,
        "all owned Value payload, including key and sources: {maximum_live}"
    );
    assert!(
        maximum_reserved < 64,
        "retained Value capacity: {maximum_reserved}"
    );
    // Ordering metadata scales with row IDs, not the number of partition graphs.
    assert!(ordering_bytes(&cursor) < input.count.0 * 64);
    assert_eq!(
        cursor.advance().unwrap(),
        crate::application::file_dag::closure_partitions::types::PartitionAdvance::Exhausted
    );
}

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
            Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let mut cursor = super::super::super::service::prepare(
        &crate::application::file_dag::relations::handle::service::Publication::of(
            publication.clone(),
        ),
        RepresentationInput::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    let mut observed = Vec::new();
    while cursor.advance().unwrap()
        == crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced
    {
        observed.push((
            cursor.current().unwrap().key.to_vec(),
            cursor.current().unwrap().table.cells.clone(),
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
            Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let input_charge = scope.derived().publication_ownership_bytes().0;
    assert!(
        input_charge > 0,
        "the actual input publication remains live"
    );
    let mut cursor = super::super::super::service::prepare(
        &crate::application::file_dag::relations::handle::service::Publication::of(
            publication.clone(),
        ),
        RepresentationInput::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    assert_eq!(
        cursor.advance().unwrap(),
        crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced
    );
    let owner = std::rc::Rc::clone(&scope);
    let first_backing =
        crate::application::file_dag::derived::owner::fixtures::resources::weak_storage(
            cursor.current.as_ref().unwrap(),
        );
    let first_charge = owner.derived().publication_ownership_bytes().0;
    assert!(first_charge > input_charge);
    assert_eq!(
        cursor.advance().unwrap(),
        crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced
    );
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
    assert_eq!(part.key, &[number(7)]);
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
    assert_eq!(
        owner.derived().publication_ownership_bytes().0,
        input_charge
    );
    drop(rows);
    drop(publication);
    assert_eq!(owner.derived().publication_ownership_bytes().0, 0);
}

fn empty_order_and_wrong_prepared_order_are_refused_precisely() {
    let input = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    let scope = scope();
    let publication = scope.computed(input.clone()).unwrap();
    let rows = scope
        .resolve(publication.as_ref(), &|_| {
            Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
        })
        .unwrap();
    let cursor = super::super::super::service::prepare(
        &crate::application::file_dag::relations::handle::service::Publication::of(
            publication.clone(),
        ),
        RepresentationInput::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )
    .unwrap();
    let mut empty = cursor;
    assert!(matches!(
        empty.current(),
        Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRun)
    ));
    assert_eq!(
        empty.advance().unwrap(),
        crate::application::file_dag::closure_partitions::types::PartitionAdvance::Exhausted
    );
    assert!(matches!(
        empty.current(),
        Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRun)
    ));
    assert!(matches!(
        empty.take_current(),
        Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRun)
    ));
    let wrong_plan = std::sync::Arc::new((*empty.partition_plan).clone());
    assert!(matches!(
        super::super::super::service::from_prepared(
            empty.proof.clone(),
            wrong_plan,
            std::sync::Arc::clone(&empty.closure_plan),
            empty.ordered.clone(),
            std::rc::Rc::clone(&empty.owner),
        ),
        Err(ryo_execution_domain::domain::operating::errors::OperatingError::WrongOperation)
    ));
}

use ryo_values::domain::rows::types::Table;

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    InterleavedPartitionsKeepOneReusedAdjacency,
    SeparateKeysNeverShareEdges,
    AdvancingReleasesOnlyThePreviousPartitionLease,
    EmptyOrderAndWrongPreparedOrderAreRefusedPrecisely,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("interleaved_partitions_keep_one_reused_adjacency"),
        word: Word::Succeeds,
        input: || Scenario::InterleavedPartitionsKeepOneReusedAdjacency,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("separate_keys_never_share_edges"),
        word: Word::Succeeds,
        input: || Scenario::SeparateKeysNeverShareEdges,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("advancing_releases_only_the_previous_partition_lease"),
        word: Word::Succeeds,
        input: || Scenario::AdvancingReleasesOnlyThePreviousPartitionLease,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_order_and_wrong_prepared_order_are_refused_precisely"),
        word: Word::Succeeds,
        input: || Scenario::EmptyOrderAndWrongPreparedOrderAreRefusedPrecisely,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::InterleavedPartitionsKeepOneReusedAdjacency => {
            interleaved_partitions_keep_one_reused_adjacency();
            Ok(())
        }
        Scenario::SeparateKeysNeverShareEdges => {
            separate_keys_never_share_edges();
            Ok(())
        }
        Scenario::AdvancingReleasesOnlyThePreviousPartitionLease => {
            advancing_releases_only_the_previous_partition_lease();
            Ok(())
        }
        Scenario::EmptyOrderAndWrongPreparedOrderAreRefusedPrecisely => {
            empty_order_and_wrong_prepared_order_are_refused_precisely();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
