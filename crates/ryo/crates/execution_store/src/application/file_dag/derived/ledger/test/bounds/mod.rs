//! Category totals, metadata and overflow stay bounded across repeated leases.
use super::*;
#[test]
fn category_totals_and_metadata_remain_bounded_across_repeated_leases() {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let mut first_peak = ByteCount(0);
    for turn in 0..64 {
        let lease = Ledger::acquire(
            &ledger,
            &[
                backing(BackingAddress(1), ByteCount(20)),
                backing(BackingAddress(2), ByteCount(30)),
            ],
            LeaseClass::Optional,
        )
        .unwrap();
        let live = ledger.borrow().snapshot();
        assert_eq!(
            (live.optional_bytes, live.unique_bytes),
            (ByteCount(50), ByteCount(50))
        );
        assert!(live.metadata_bytes > ByteCount(0));
        drop(lease);
        let released = ledger.borrow().snapshot();
        assert_eq!(
            (released.optional_bytes, released.unique_bytes),
            (ByteCount(0), ByteCount(0))
        );
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
            &[
                backing(BackingAddress(1), ByteCount(usize::MAX)),
                backing(BackingAddress(2), ByteCount(1))
            ],
            LeaseClass::Required
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    let too_many_positions = [
        positions(BackingAddress(3), ByteCount(1), UnitCount(usize::MAX)),
        positions(BackingAddress(4), ByteCount(1), UnitCount(1)),
    ];
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
