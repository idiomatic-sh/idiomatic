//! The leases of the `bounds` cases and what each observed.
use crate::application::file_dag::accounting::ledger::fixtures::{backing, positions};
use crate::application::file_dag::accounting::ledger::service::Ledger;
use crate::application::file_dag::accounting::ledger::types::{
    BackingAddress, ByteCount, LeaseClass, UnitCount,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::{borrow::Cow, cell::RefCell, rc::Rc};

/// What one turn of an optional lease taken and released observed.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Turn {
    /// Optional and unique bytes while the lease is live.
    pub(super) live: (ByteCount, ByteCount),
    /// Whether the live lease charged metadata.
    pub(super) metadata_charged: bool,
    /// Optional and unique bytes once it is released.
    pub(super) released: (ByteCount, ByteCount),
    /// Whether the peak metadata is the first turn's.
    pub(super) peak_metadata_as_first: bool,
}

/// What repeated leases observed: how many turns ran and each distinct turn, in order.
#[derive(Debug, PartialEq)]
pub(super) struct Repeated {
    pub(super) turns: usize,
    pub(super) distinct: Cow<'static, [Turn]>,
}

/// What overflowing asks were refused with, and whether the ledger moved.
#[derive(Debug, PartialEq)]
pub(super) struct Overflow {
    pub(super) bytes: Option<OperatingError>,
    pub(super) incremental_units: Option<OperatingError>,
    pub(super) units: Option<OperatingError>,
    pub(super) unchanged: bool,
}

/// What a `bounds` case observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    Repeated(Repeated),
    Overflow(Overflow),
}

/// Sixty-four turns of one optional lease of 20 and 30 bytes, taken and released.
pub(super) fn repeated() -> Result<Repeated, OperatingError> {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let mut first_peak = ByteCount(0);
    let mut distinct: Vec<Turn> = Vec::new();
    let turns = 64;
    for turn in 0..turns {
        let lease = Ledger::acquire(
            &ledger,
            &[
                backing(BackingAddress(1), ByteCount(20)),
                backing(BackingAddress(2), ByteCount(30)),
            ],
            LeaseClass::Optional,
        )?;
        let live = ledger.borrow().snapshot();
        drop(lease);
        let released = ledger.borrow().snapshot();
        if turn == 0 {
            first_peak = released.peak_metadata_bytes;
        }
        let observed = Turn {
            live: (live.optional_bytes, live.unique_bytes),
            metadata_charged: live.metadata_bytes > ByteCount(0),
            released: (released.optional_bytes, released.unique_bytes),
            peak_metadata_as_first: released.peak_metadata_bytes == first_peak,
        };
        if !distinct.contains(&observed) {
            distinct.push(observed);
        }
    }
    Ok(Repeated {
        turns,
        distinct: Cow::Owned(distinct),
    })
}

/// Asks whose bytes or position units overflow a count.
pub(super) fn overflow() -> Overflow {
    let ledger = Rc::new(RefCell::new(Ledger::default()));
    let before = ledger.borrow().snapshot();
    let bytes = Ledger::acquire(
        &ledger,
        &[
            backing(BackingAddress(1), ByteCount(usize::MAX)),
            backing(BackingAddress(2), ByteCount(1)),
        ],
        LeaseClass::Required,
    )
    .err();
    let too_many_positions = [
        positions(BackingAddress(3), ByteCount(1), UnitCount(usize::MAX)),
        positions(BackingAddress(4), ByteCount(1), UnitCount(1)),
    ];
    let incremental_units = ledger
        .borrow()
        .incremental_optional_units(&too_many_positions)
        .err();
    let units = Ledger::acquire(&ledger, &too_many_positions, LeaseClass::Optional).err();
    Overflow {
        bytes,
        incremental_units,
        units,
        unchanged: ledger.borrow().snapshot() == before,
    }
}
