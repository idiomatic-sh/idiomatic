use super::types::{
    Backing, BackingAddress, BackingKey, BackingTag, ByteCount, Credit, LeaseClass, Ledger,
    UnitCount,
};
use crate::domain::operating::errors::OperatingError;
use std::{cell::RefCell, rc::Rc, sync::Arc};

fn backing(key: BackingAddress, bytes: ByteCount) -> Backing {
    Backing {
        key: BackingKey {
            tag: BackingTag(1),
            address: key,
        },
        bytes,
        retention_units: UnitCount(0),
    }
}

fn positions(key: BackingAddress, bytes: ByteCount, capacity: UnitCount) -> Backing {
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
            &[
                backing(BackingAddress(7), ByteCount(100)),
                backing(BackingAddress(7), ByteCount(100)),
            ],
            LeaseClass::Required,
        )
        .unwrap(),
    );
    let other_consumer = Arc::clone(&required);
    assert_eq!(ledger.borrow().snapshot().required_bytes, ByteCount(100));
    assert_eq!(ledger.borrow().snapshot().unique_bytes, ByteCount(100));
    drop(required);
    assert_eq!(ledger.borrow().snapshot().unique_bytes, ByteCount(100));
    drop(other_consumer);
    let after = ledger.borrow().snapshot();
    assert_eq!(after.required_bytes, ByteCount(0));
    assert_eq!(after.unique_bytes, ByteCount(0));
    assert_eq!(after.peak_unique_bytes, ByteCount(100));
}

#[test]
fn optional_retention_survives_required_release_without_double_physical_charge() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Ledger::acquire(
        &ledger,
        &[backing(BackingAddress(7), ByteCount(100))],
        LeaseClass::Required,
    )
    .unwrap();
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_bytes(&[backing(BackingAddress(7), ByteCount(100))])
            .unwrap(),
        ByteCount(100)
    );
    let optional = Ledger::acquire(
        &ledger,
        &[backing(BackingAddress(7), ByteCount(100))],
        LeaseClass::Optional,
    )
    .unwrap();
    let both = ledger.borrow().snapshot();
    assert_eq!(
        (
            both.required_bytes,
            both.optional_bytes,
            both.overlap_bytes,
            both.unique_bytes
        ),
        (
            ByteCount(100),
            ByteCount(100),
            ByteCount(100),
            ByteCount(100)
        )
    );
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_bytes(&[backing(BackingAddress(7), ByteCount(100))])
            .unwrap(),
        ByteCount(0)
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
        (ByteCount(0), ByteCount(100), ByteCount(0), ByteCount(100))
    );
    drop(optional);
    assert_eq!(ledger.borrow().snapshot().unique_bytes, ByteCount(0));
}

#[test]
fn optional_release_keeps_required_backing_and_recharges_the_view_allowance() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(BackingAddress(8), ByteCount(400), UnitCount(100));
    let view = Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap();
    let required = Ledger::acquire(&ledger, &[rows], LeaseClass::Required).unwrap();
    let both = ledger.borrow().snapshot();
    assert_eq!(
        (
            both.required_bytes,
            both.optional_bytes,
            both.overlap_bytes,
            both.unique_bytes
        ),
        (
            ByteCount(400),
            ByteCount(400),
            ByteCount(400),
            ByteCount(400)
        )
    );
    drop(view);
    let required_only = ledger.borrow().snapshot();
    assert_eq!(
        (
            required_only.required_bytes,
            required_only.optional_bytes,
            required_only.optional_units,
            required_only.overlap_bytes,
            required_only.unique_bytes
        ),
        (
            ByteCount(400),
            ByteCount(0),
            UnitCount(0),
            ByteCount(0),
            ByteCount(400)
        )
    );
    assert_eq!(
        ledger.borrow().incremental_optional_bytes(&[rows]).unwrap(),
        ByteCount(400)
    );
    assert_eq!(
        ledger.borrow().incremental_optional_units(&[rows]).unwrap(),
        UnitCount(100)
    );
    drop(required);
    let released = ledger.borrow().snapshot();
    assert_eq!(released.unique_bytes, ByteCount(0));
    assert_eq!(released.peak_required_bytes, ByteCount(400));
    assert_eq!(released.peak_optional_bytes, ByteCount(400));
    assert_eq!(released.peak_view_optional_bytes, ByteCount(400));
    assert_eq!(released.peak_overlap_bytes, ByteCount(400));
    assert_eq!(released.peak_optional_units, UnitCount(100));
}

#[test]
fn position_capacity_is_charged_once_per_optional_backing_and_survives_required_release() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let row_ids = positions(BackingAddress(9), ByteCount(400), UnitCount(100));
    let required = Ledger::acquire(&ledger, &[row_ids, row_ids], LeaseClass::Required).unwrap();
    assert_eq!(ledger.borrow().snapshot().optional_units, UnitCount(0));
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_units(&[row_ids, row_ids])
            .unwrap(),
        UnitCount(100)
    );
    let optional = Ledger::acquire(&ledger, &[row_ids, row_ids], LeaseClass::Optional).unwrap();
    let same_optional = Ledger::acquire(&ledger, &[row_ids], LeaseClass::Optional).unwrap();
    assert_eq!(ledger.borrow().snapshot().optional_units, UnitCount(100));
    assert_eq!(
        ledger
            .borrow()
            .incremental_optional_units(&[row_ids])
            .unwrap(),
        UnitCount(0)
    );
    drop(required);
    assert_eq!(ledger.borrow().snapshot().optional_units, UnitCount(100));
    drop(optional);
    assert_eq!(ledger.borrow().snapshot().optional_units, UnitCount(100));
    drop(same_optional);
    let final_state = ledger.borrow().snapshot();
    assert_eq!(final_state.optional_units, UnitCount(0));
    assert_eq!(final_state.peak_optional_units, UnitCount(100));
}

#[test]
fn optional_release_preview_requires_every_last_handle_and_counts_units_separately() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(BackingAddress(10), ByteCount(400), UnitCount(100));
    let first = Arc::new(Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap());
    let second = Arc::new(Ledger::acquire(&ledger, &[rows], LeaseClass::Optional).unwrap());

    assert_eq!(
        ledger.borrow().optional_release_credit(&[&first]).unwrap(),
        Some(Credit {
            bytes: ByteCount(0),
            units: UnitCount(0),
        })
    );
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some(Credit {
            bytes: ByteCount(400),
            units: UnitCount(100),
        })
    );

    let live_consumer = Arc::clone(&first);
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some(Credit {
            bytes: ByteCount(0),
            units: UnitCount(0),
        })
    );
    drop(live_consumer);

    let required = Ledger::acquire(&ledger, &[rows], LeaseClass::Required).unwrap();
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&first, &second])
            .unwrap(),
        Some(Credit {
            bytes: ByteCount(0),
            units: UnitCount(100),
        })
    );
    drop(required);
    let shared = Arc::new(
        Ledger::acquire(
            &ledger,
            &[positions(BackingAddress(11), ByteCount(250), UnitCount(60))],
            LeaseClass::Optional,
        )
        .unwrap(),
    );
    let other_slot = Arc::clone(&shared);
    assert_eq!(
        ledger.borrow().optional_release_credit(&[&shared]).unwrap(),
        Some(Credit {
            bytes: ByteCount(0),
            units: UnitCount(0),
        })
    );
    assert_eq!(
        ledger
            .borrow()
            .optional_release_credit(&[&shared, &other_slot])
            .unwrap(),
        Some(Credit {
            bytes: ByteCount(250),
            units: UnitCount(60),
        })
    );

    drop(first);
    drop(second);
    drop(shared);
    drop(other_slot);
    assert_eq!(ledger.borrow().snapshot().optional_units, UnitCount(0));
}

#[test]
fn a_reused_key_cannot_change_its_immutable_size() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Ledger::acquire(
        &ledger,
        &[backing(BackingAddress(3), ByteCount(10))],
        LeaseClass::Required,
    )
    .unwrap();
    let before = ledger.borrow().snapshot();
    assert!(matches!(
        ledger
            .borrow()
            .incremental_optional_bytes(&[backing(BackingAddress(3), ByteCount(11))]),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[backing(BackingAddress(3), ByteCount(11))],
            LeaseClass::Optional
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        ledger.borrow().incremental_optional_units(&[positions(
            BackingAddress(3),
            ByteCount(10),
            UnitCount(1)
        )]),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[positions(BackingAddress(3), ByteCount(10), UnitCount(1))],
            LeaseClass::Optional
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[
                positions(BackingAddress(5), ByteCount(10), UnitCount(1)),
                positions(BackingAddress(5), ByteCount(10), UnitCount(2))
            ],
            LeaseClass::Optional
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        Ledger::acquire(
            &ledger,
            &[
                backing(BackingAddress(4), ByteCount(5)),
                backing(BackingAddress(4), ByteCount(6))
            ],
            LeaseClass::Required
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert_eq!(ledger.borrow().snapshot(), before);
    drop(required);
}

mod bounds;
