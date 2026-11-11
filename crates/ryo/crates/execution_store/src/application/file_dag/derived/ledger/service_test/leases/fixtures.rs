//! The leases of the `leases` cases and what each observed.
use crate::application::file_dag::derived::ledger::fixtures::{backing, positions};
use crate::application::file_dag::derived::ledger::service::Ledger;
use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, ByteCount, Credit, LeaseClass, UnitCount,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::{cell::RefCell, rc::Rc, sync::Arc};

/// Required, optional, overlap and unique bytes of one snapshot.
type Categories = (ByteCount, ByteCount, ByteCount, ByteCount);

/// What one backing leased twice, then released by its last consumer, observed.
#[derive(Debug, PartialEq)]
pub(super) struct SharedAlias {
    pub(super) required: ByteCount,
    pub(super) unique: ByteCount,
    pub(super) unique_after_first_release: ByteCount,
    pub(super) required_after: ByteCount,
    pub(super) unique_after: ByteCount,
    pub(super) peak_unique: ByteCount,
}

/// What an optional lease beside a required one of the same backing observed.
#[derive(Debug, PartialEq)]
pub(super) struct OptionalRetention {
    pub(super) incremental_before: ByteCount,
    pub(super) both: Categories,
    pub(super) incremental_beside: ByteCount,
    pub(super) retained: Categories,
    pub(super) unique_after: ByteCount,
}

/// What releasing a view before its required lease observed.
#[derive(Debug, PartialEq)]
pub(super) struct OptionalRelease {
    pub(super) both: Categories,
    pub(super) required_only: (ByteCount, ByteCount, UnitCount, ByteCount, ByteCount),
    pub(super) incremental_bytes: ByteCount,
    pub(super) incremental_units: UnitCount,
    pub(super) unique_after: ByteCount,
    pub(super) peak_required: ByteCount,
    pub(super) peak_optional: ByteCount,
    pub(super) peak_view_optional: ByteCount,
    pub(super) peak_overlap: ByteCount,
    pub(super) peak_optional_units: UnitCount,
}

/// What position capacity charged as required and optional leases came and went.
#[derive(Debug, PartialEq)]
pub(super) struct PositionCapacity {
    pub(super) units_while_required: UnitCount,
    pub(super) incremental_before: UnitCount,
    pub(super) units_with_optionals: UnitCount,
    pub(super) incremental_beside: UnitCount,
    pub(super) units_after_required: UnitCount,
    pub(super) units_after_one_optional: UnitCount,
    pub(super) units_after: UnitCount,
    pub(super) peak_units: UnitCount,
}

/// What previews of releasing optional handles credited.
#[derive(Debug, PartialEq)]
pub(super) struct ReleasePreview {
    pub(super) first_alone: Option<Credit>,
    pub(super) every_handle: Option<Credit>,
    pub(super) while_consumed: Option<Credit>,
    pub(super) beside_required: Option<Credit>,
    pub(super) shared_alone: Option<Credit>,
    pub(super) shared_with_its_other_slot: Option<Credit>,
    pub(super) units_after: UnitCount,
}

/// What asks that change a live key's size were refused with, and whether the ledger moved.
#[derive(Debug, PartialEq)]
pub(super) struct ReusedKey {
    pub(super) refusals: [Option<OperatingError>; 6],
    pub(super) unchanged: bool,
}

/// What a `leases` case observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    SharedAlias(SharedAlias),
    OptionalRetention(OptionalRetention),
    OptionalRelease(OptionalRelease),
    PositionCapacity(PositionCapacity),
    ReleasePreview(ReleasePreview),
    ReusedKey(ReusedKey),
}

fn categories(ledger: &Rc<RefCell<Ledger>>) -> Categories {
    let snapshot = ledger.borrow().snapshot();
    (
        snapshot.required_bytes,
        snapshot.optional_bytes,
        snapshot.overlap_bytes,
        snapshot.unique_bytes,
    )
}

/// One backing listed twice in a required lease shared by two consumers.
pub(super) fn shared_alias() -> Result<SharedAlias, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Arc::new(Ledger::acquire(
        &ledger,
        &[
            backing(BackingAddress(7), ByteCount(100)),
            backing(BackingAddress(7), ByteCount(100)),
        ],
        LeaseClass::Required,
    )?);
    let other_consumer = Arc::clone(&required);
    let required_bytes = ledger.borrow().snapshot().required_bytes;
    let unique = ledger.borrow().snapshot().unique_bytes;
    drop(required);
    let unique_after_first_release = ledger.borrow().snapshot().unique_bytes;
    drop(other_consumer);
    let after = ledger.borrow().snapshot();
    Ok(SharedAlias {
        required: required_bytes,
        unique,
        unique_after_first_release,
        required_after: after.required_bytes,
        unique_after: after.unique_bytes,
        peak_unique: after.peak_unique_bytes,
    })
}

/// A required lease of 100 bytes, then an optional one of the same backing.
pub(super) fn optional_retention() -> Result<OptionalRetention, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let held = [backing(BackingAddress(7), ByteCount(100))];
    let required = Ledger::acquire(&ledger, &held, LeaseClass::Required)?;
    let incremental_before = ledger.borrow().incremental_optional_bytes(&held)?;
    let optional = Ledger::acquire(&ledger, &held, LeaseClass::Optional)?;
    let both = categories(&ledger);
    let incremental_beside = ledger.borrow().incremental_optional_bytes(&held)?;
    drop(required);
    let retained = categories(&ledger);
    drop(optional);
    let unique_after = ledger.borrow().snapshot().unique_bytes;
    Ok(OptionalRetention {
        incremental_before,
        both,
        incremental_beside,
        retained,
        unique_after,
    })
}

/// An optional view and a required lease of 400 bytes and 100 positions; the view goes first.
pub(super) fn optional_release() -> Result<OptionalRelease, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(BackingAddress(8), ByteCount(400), UnitCount(100));
    let view = Ledger::acquire(&ledger, &[rows], LeaseClass::Optional)?;
    let required = Ledger::acquire(&ledger, &[rows], LeaseClass::Required)?;
    let both = categories(&ledger);
    drop(view);
    let only = ledger.borrow().snapshot();
    let incremental_bytes = ledger.borrow().incremental_optional_bytes(&[rows])?;
    let incremental_units = ledger.borrow().incremental_optional_units(&[rows])?;
    drop(required);
    let released = ledger.borrow().snapshot();
    Ok(OptionalRelease {
        both,
        required_only: (
            only.required_bytes,
            only.optional_bytes,
            only.optional_units,
            only.overlap_bytes,
            only.unique_bytes,
        ),
        incremental_bytes,
        incremental_units,
        unique_after: released.unique_bytes,
        peak_required: released.peak_required_bytes,
        peak_optional: released.peak_optional_bytes,
        peak_view_optional: released.peak_view_optional_bytes,
        peak_overlap: released.peak_overlap_bytes,
        peak_optional_units: released.peak_optional_units,
    })
}

/// A required lease of one position backing listed twice, then two optional leases of it.
pub(super) fn position_capacity() -> Result<PositionCapacity, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let row_ids = positions(BackingAddress(9), ByteCount(400), UnitCount(100));
    let units = |ledger: &Rc<RefCell<Ledger>>| ledger.borrow().snapshot().optional_units;
    let required = Ledger::acquire(&ledger, &[row_ids, row_ids], LeaseClass::Required)?;
    let units_while_required = units(&ledger);
    let incremental_before = ledger
        .borrow()
        .incremental_optional_units(&[row_ids, row_ids])?;
    let optional = Ledger::acquire(&ledger, &[row_ids, row_ids], LeaseClass::Optional)?;
    let same_optional = Ledger::acquire(&ledger, &[row_ids], LeaseClass::Optional)?;
    let units_with_optionals = units(&ledger);
    let incremental_beside = ledger.borrow().incremental_optional_units(&[row_ids])?;
    drop(required);
    let units_after_required = units(&ledger);
    drop(optional);
    let units_after_one_optional = units(&ledger);
    drop(same_optional);
    let after = ledger.borrow().snapshot();
    Ok(PositionCapacity {
        units_while_required,
        incremental_before,
        units_with_optionals,
        incremental_beside,
        units_after_required,
        units_after_one_optional,
        units_after: after.optional_units,
        peak_units: after.peak_optional_units,
    })
}

/// Two optional handles of one backing, a live consumer, a required lease, then one shared handle.
pub(super) fn release_preview() -> Result<ReleasePreview, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let rows = positions(BackingAddress(10), ByteCount(400), UnitCount(100));
    let first = Arc::new(Ledger::acquire(&ledger, &[rows], LeaseClass::Optional)?);
    let second = Arc::new(Ledger::acquire(&ledger, &[rows], LeaseClass::Optional)?);
    let first_alone = ledger.borrow().optional_release_credit(&[&first])?;
    let every_handle = ledger
        .borrow()
        .optional_release_credit(&[&first, &second])?;
    let live_consumer = Arc::clone(&first);
    let while_consumed = ledger
        .borrow()
        .optional_release_credit(&[&first, &second])?;
    drop(live_consumer);
    let required = Ledger::acquire(&ledger, &[rows], LeaseClass::Required)?;
    let beside_required = ledger
        .borrow()
        .optional_release_credit(&[&first, &second])?;
    drop(required);
    let shared = Arc::new(Ledger::acquire(
        &ledger,
        &[positions(BackingAddress(11), ByteCount(250), UnitCount(60))],
        LeaseClass::Optional,
    )?);
    let other_slot = Arc::clone(&shared);
    let shared_alone = ledger.borrow().optional_release_credit(&[&shared])?;
    let shared_with_its_other_slot = ledger
        .borrow()
        .optional_release_credit(&[&shared, &other_slot])?;
    drop(first);
    drop(second);
    drop(shared);
    drop(other_slot);
    let units_after = ledger.borrow().snapshot().optional_units;
    Ok(ReleasePreview {
        first_alone,
        every_handle,
        while_consumed,
        beside_required,
        shared_alone,
        shared_with_its_other_slot,
        units_after,
    })
}

/// A required lease of 10 bytes at key 3, then asks that give a key a second size.
pub(super) fn reused_key() -> Result<ReusedKey, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let required = Ledger::acquire(
        &ledger,
        &[backing(BackingAddress(3), ByteCount(10))],
        LeaseClass::Required,
    )?;
    let before = ledger.borrow().snapshot();
    let first = ledger
        .borrow()
        .incremental_optional_bytes(&[backing(BackingAddress(3), ByteCount(11))])
        .err();
    let second = Ledger::acquire(
        &ledger,
        &[backing(BackingAddress(3), ByteCount(11))],
        LeaseClass::Optional,
    )
    .err();
    let third = ledger
        .borrow()
        .incremental_optional_units(&[positions(BackingAddress(3), ByteCount(10), UnitCount(1))])
        .err();
    let fourth = Ledger::acquire(
        &ledger,
        &[positions(BackingAddress(3), ByteCount(10), UnitCount(1))],
        LeaseClass::Optional,
    )
    .err();
    let fifth = Ledger::acquire(
        &ledger,
        &[
            positions(BackingAddress(5), ByteCount(10), UnitCount(1)),
            positions(BackingAddress(5), ByteCount(10), UnitCount(2)),
        ],
        LeaseClass::Optional,
    )
    .err();
    let sixth = Ledger::acquire(
        &ledger,
        &[
            backing(BackingAddress(4), ByteCount(5)),
            backing(BackingAddress(4), ByteCount(6)),
        ],
        LeaseClass::Required,
    )
    .err();
    let refusals = [first, second, third, fourth, fifth, sixth];
    let unchanged = ledger.borrow().snapshot() == before;
    drop(required);
    Ok(ReusedKey {
        refusals,
        unchanged,
    })
}
