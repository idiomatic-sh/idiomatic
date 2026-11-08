//! The values the ledger counts and the records it keeps.
use std::{cell::RefCell, rc::Rc};
use rustc_hash::FxHashMap;

/// Physical bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteCount(pub usize);

/// Immutable row-position capacity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct UnitCount(pub usize);

/// Live leases of one class on one backing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HoldCount(pub usize);

/// Backings of one kind a ledger holds.
#[cfg(any(test, feature = "fixtures"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BackingCount(pub usize);

/// Elements a container has reserved room for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SlotCount(pub usize);

/// The kind of physical payload a backing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BackingTag(pub u8);

/// The address of the allocation a backing is, or the scope id of an unshared one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BackingAddress(pub usize);

/// One physical allocation, identified across every handle that shares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BackingKey {
    pub tag: BackingTag,
    pub address: BackingAddress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Backing {
    pub key: BackingKey,
    pub bytes: ByteCount,
    /// Immutable row-position capacity when this backing is optionally retained.
    pub retention_units: UnitCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeaseClass {
    Required,
    /// Derived-view retention, subject to the view byte and position caps.
    Optional,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LedgerSnapshot {
    pub required_bytes: ByteCount,
    /// Physical bytes retained by derived views, counted once per backing.
    pub optional_bytes: ByteCount,
    pub view_optional_bytes: ByteCount,
    pub optional_units: UnitCount,
    pub overlap_bytes: ByteCount,
    pub unique_bytes: ByteCount,
    pub metadata_bytes: ByteCount,
    pub peak_required_bytes: ByteCount,
    pub peak_optional_bytes: ByteCount,
    pub peak_view_optional_bytes: ByteCount,
    pub peak_optional_units: UnitCount,
    pub peak_overlap_bytes: ByteCount,
    pub peak_unique_bytes: ByteCount,
    pub peak_metadata_bytes: ByteCount,
}

#[derive(Debug)]
pub(crate) struct LedgerEntry {
    pub(super) bytes: ByteCount,
    pub(super) retention_units: UnitCount,
    pub(super) required: HoldCount,
    pub(super) optional: HoldCount,
}

#[derive(Debug)]
pub(crate) struct Ledger {
    pub(super) entries: FxHashMap<BackingKey, LedgerEntry>,
    pub(super) required_bytes: ByteCount,
    pub(super) optional_bytes: ByteCount,
    pub(super) view_optional_bytes: ByteCount,
    pub(super) optional_units: UnitCount,
    pub(super) overlap_bytes: ByteCount,
    pub(super) unique_bytes: ByteCount,
    pub(super) lease_metadata: ByteCount,
    pub(super) peak_required_bytes: ByteCount,
    pub(super) peak_optional_bytes: ByteCount,
    pub(super) peak_view_optional_bytes: ByteCount,
    pub(super) peak_optional_units: UnitCount,
    pub(super) peak_overlap_bytes: ByteCount,
    pub(super) peak_unique_bytes: ByteCount,
    pub(super) peak_metadata_bytes: ByteCount,
}

/// One physical handle, shared by consumers through an outer `Arc`.
#[derive(Debug)]
pub(crate) struct ResourceLease {
    pub(super) ledger: Rc<RefCell<Ledger>>,
    pub(super) keys: Vec<BackingKey>,
    pub(super) class: LeaseClass,
    pub(super) metadata_bytes: ByteCount,
}

/// A backing with its immutable size, once the duplicates of a request are folded.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Measured {
    pub key: BackingKey,
    pub bytes: ByteCount,
    pub units: UnitCount,
}

/// What releasing a set of optional handles would give back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Credit {
    pub bytes: ByteCount,
    pub units: UnitCount,
}

/// Whether the backings of a request are held by optional views alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dormancy {
    Dormant,
    Live,
}
