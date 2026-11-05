use super::{Backing, LeaseClass, Ledger};
use crate::domain::operating::errors::OperatingError;
use std::{cell::RefCell, rc::Rc, sync::Arc};

fn backing(key: usize, bytes: usize) -> Backing {
    Backing {
        key: (1, key),
        bytes,
        retention_units: 0,
    }
}

fn positions(key: usize, bytes: usize, capacity: usize) -> Backing {
    Backing {
        retention_units: capacity,
        ..backing(key, bytes)
    }
}

#[test]
fn shared_alias_is_charged_once_and_last_consumer_releases_it() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Arc::new(
        Ledger::acquire(
            &ledger,
            &[backing(7, 100), backing(7, 100)],
            LeaseClass::Required,
        )
        .unwrap(),
    );
    let other_consumer = Arc::clone(&required);
    assert_eq!(ledger.borrow().snapshot().required_bytes, 100);
    assert_eq!(ledger.borrow().snapshot().unique_bytes, 100);
    drop(required);
    assert_eq!(ledger.borrow().snapshot().unique_bytes, 100);
    drop(other_consumer);
    let after = ledger.borrow().snapshot();
    assert_eq!(after.required_bytes, 0);
    assert_eq!(after.unique_bytes, 0);
    assert_eq!(after.peak_unique_bytes, 100);
}

#[test]
fn optional_retention_survives_required_release_without_double_physical_charge() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Ledger::acquire(&ledger, &[backing(7, 100)], LeaseClass::Required).unwrap();
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_bytes(&[backing(7, 100)])
            .unwrap(),
        100
    );
    let optional = Ledger::acquire(&ledger, &[backing(7, 100)], LeaseClass::Optional).unwrap();
    let both = ledger.borrow().snapshot();
    assert_eq!(
        (
            both.required_bytes,
            both.optional_bytes,
            both.overlap_bytes,
            both.unique_bytes
        ),
        (100, 100, 100, 100)
    );
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_bytes(&[backing(7, 100)])
            .unwrap(),
        0
    );
    drop(required);
    let retained = ledger.borrow().snapshot();
    assert_eq!(
        (
            retained.required_bytes,
            retained.optional_bytes,
            retained.overlap_bytes,
            retained.unique_bytes
        ),
        (0, 100, 0, 100)
    );
    drop(optional);
    assert_eq!(ledger.borrow().snapshot().unique_bytes, 0);
}

#[test]
fn node_and_view_optional_caps_charge_the_same_backing_independently() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(8, 400, 100);
    let required = Ledger::acquire(&ledger, &[rows], LeaseClass::Required).unwrap();
    let node = Ledger::acquire(&ledger, &[rows], LeaseClass::NodeOptional).unwrap();
    let before_view = ledger.borrow().snapshot();
    assert_eq!(
        (
            before_view.required_bytes,
            before_view.optional_bytes,
            before_view.node_optional_bytes,
            before_view.view_optional_bytes,
            before_view.optional_units,
            before_view.overlap_bytes,
            before_view.unique_bytes,
        ),
        (400, 400, 400, 0, 0, 400, 400)
    );
    assert_eq!(
        ledger.borrow().incremental_optional_bytes(&[rows]).unwrap(),
        400
    );
    assert_eq!(
        ledger
            .borrow()
            .incremental_node_optional_bytes(&[rows])
            .unwrap(),
        0
    );
    let view = Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap();
    let all = ledger.borrow().snapshot();
    assert_eq!(
        (
            all.optional_bytes,
            all.node_optional_bytes,
            all.view_optional_bytes,
            all.optional_units,
            all.overlap_bytes,
            all.unique_bytes,
        ),
        (400, 400, 400, 100, 400, 400)
    );
    assert_eq!(
        ledger.borrow().incremental_optional_bytes(&[rows]).unwrap(),
        0
    );
    drop(node);
    let view_only = ledger.borrow().snapshot();
    assert_eq!(
        (
            view_only.optional_bytes,
            view_only.node_optional_bytes,
            view_only.view_optional_bytes,
            view_only.overlap_bytes,
        ),
        (400, 0, 400, 400)
    );
    assert_eq!(
        ledger
            .borrow()
            .incremental_node_optional_bytes(&[rows])
            .unwrap(),
        400
    );
    drop(required);
    let retained = ledger.borrow().snapshot();
    assert_eq!(
        (
            retained.required_bytes,
            retained.optional_bytes,
            retained.overlap_bytes,
            retained.unique_bytes,
        ),
        (0, 400, 0, 400)
    );
    drop(view);
    let released = ledger.borrow().snapshot();
    assert_eq!((released.optional_bytes, released.unique_bytes), (0, 0));
    assert_eq!(released.peak_optional_bytes, 400);
    assert_eq!(released.peak_node_optional_bytes, 400);
    assert_eq!(released.peak_view_optional_bytes, 400);
}

#[test]
fn dropping_view_retention_preserves_node_retention_and_recharges_the_view_cap() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(8, 400, 100);
    let view = Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap();
    assert_eq!(
        ledger
            .borrow()
            .incremental_node_optional_bytes(&[rows])
            .unwrap(),
        400
    );
    let node = Ledger::acquire(&ledger, &[rows], LeaseClass::NodeOptional).unwrap();
    drop(view);
    let node_only = ledger.borrow().snapshot();
    assert_eq!(
        (
            node_only.optional_bytes,
            node_only.view_optional_bytes,
            node_only.node_optional_bytes,
            node_only.optional_units,
            node_only.unique_bytes,
        ),
        (400, 0, 400, 0, 400)
    );
    assert_eq!(
        ledger.borrow().incremental_optional_bytes(&[rows]).unwrap(),
        400
    );
    drop(node);
    assert_eq!(ledger.borrow().snapshot().unique_bytes, 0);
}

#[test]
fn position_capacity_is_charged_once_per_optional_backing_and_survives_required_release() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let row_ids = positions(9, 400, 100);
    let required = Ledger::acquire(&ledger, &[row_ids, row_ids], LeaseClass::Required).unwrap();
    assert_eq!(ledger.borrow().snapshot().optional_units, 0);
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_units(&[row_ids, row_ids])
            .unwrap(),
        100
    );
    let optional = Ledger::acquire(&ledger, &[row_ids, row_ids], LeaseClass::Optional).unwrap();
    let same_optional = Ledger::acquire(&ledger, &[row_ids], LeaseClass::Optional).unwrap();
    assert_eq!(ledger.borrow().snapshot().optional_units, 100);
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_units(&[row_ids])
            .unwrap(),
        0
    );
    drop(required);
    assert_eq!(ledger.borrow().snapshot().optional_units, 100);
    drop(optional);
    assert_eq!(ledger.borrow().snapshot().optional_units, 100);
    drop(same_optional);
    let final_state = ledger.borrow().snapshot();
    assert_eq!(final_state.optional_units, 0);
    assert_eq!(final_state.peak_optional_units, 100);
}

#[test]
fn optional_release_preview_requires_every_last_handle_and_counts_units_separately() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(10, 400, 100);
    let first = Arc::new(Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap());
    let second = Arc::new(Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap());

    assert_eq!(
        ledger.borrow().optional_release_credit(&[&first]).unwrap(),
        Some((0, 0))
    );
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some((400, 100))
    );

    let live_consumer = Arc::clone(&first);
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some((0, 0))
    );
    drop(live_consumer);

    let required = Ledger::acquire(&ledger, &[rows], LeaseClass::Required).unwrap();
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some((0, 100))
    );
    drop(required);
    let node = Ledger::acquire(&ledger, &[rows], LeaseClass::NodeOptional).unwrap();
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some((0, 100))
    );
    drop(node);

    let shared = Arc::new(
        Ledger::acquire(&ledger, &[positions(11, 250, 60)], LeaseClass::Optional).unwrap(),
    );
    let other_slot = Arc::clone(&shared);
    assert_eq!(
        ledger.borrow().optional_release_credit(&[&shared]).unwrap(),
        Some((0, 0))
    );
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&shared, &other_slot])
            .unwrap(),
        Some((250, 60))
    );

    drop(first);
    drop(second);
    drop(shared);
    drop(other_slot);
    assert_eq!(ledger.borrow().snapshot().optional_units, 0);
}

#[test]
fn a_reused_key_cannot_change_its_immutable_size() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Ledger::acquire(&ledger, &[backing(3, 10)], LeaseClass::Required).unwrap();
    let before = ledger.borrow().snapshot();
    assert!(matches!(
        ledger
            .borrow()
            .incremental_optional_bytes(&[backing(3, 11)]),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(&ledger, &[backing(3, 11)], LeaseClass::Optional),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        ledger
            .borrow()
            .incremental_optional_units(&[positions(3, 10, 1)]),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        ledger
            .borrow()
            .incremental_node_optional_bytes(&[positions(3, 10, 1)]),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(&ledger, &[positions(3, 10, 1)], LeaseClass::Optional),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[positions(5, 10, 1), positions(5, 10, 2)],
            LeaseClass::Optional
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[backing(4, 5), backing(4, 6)],
            LeaseClass::Required
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert_eq!(ledger.borrow().snapshot(), before);
    drop(required);
}

#[test]
fn category_totals_and_metadata_remain_bounded_across_repeated_leases() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let mut first_peak = 0;
    for turn in 0..64 {
        let lease = Ledger::acquire(
            &ledger,
            &[backing(1, 20), backing(2, 30)],
            LeaseClass::Optional,
        )
        .unwrap();
        let live = ledger.borrow().snapshot();
        assert_eq!((live.optional_bytes, live.unique_bytes), (50, 50));
        assert!(live.metadata_bytes > 0);
        drop(lease);
        let released = ledger.borrow().snapshot();
        assert_eq!((released.optional_bytes, released.unique_bytes), (0, 0));
        if turn == 0 {
            first_peak = released.peak_metadata_bytes;
        }
        assert_eq!(released.peak_metadata_bytes, first_peak);
    }
}

#[test]
fn overflow_refuses_without_mutating_live_accounting() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let before = ledger.borrow().snapshot();
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[backing(1, usize::MAX), backing(2, 1)],
            LeaseClass::Required
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    let too_many_positions = [positions(3, 1, usize::MAX), positions(4, 1, 1)];
    assert!(matches!(
        ledger
            .borrow()
            .incremental_optional_units(&too_many_positions),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(&ledger, &too_many_positions, LeaseClass::Optional),
        Err(OperatingError::ShapeMismatch)
    ));
    assert_eq!(ledger.borrow().snapshot(), before);
}
