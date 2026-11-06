//! The values the ledger counts and the records it keeps.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Physical bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag) struct ByteCount(pub usize);

/// Immutable row-position capacity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag) struct UnitCount(pub usize);

/// Live leases of one class on one backing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag::derived) struct HoldCount(pub usize);

/// Backings of one kind a ledger holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::application::file_dag::derived) struct BackingCount(pub usize);

/// Elements a container has reserved room for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag) struct SlotCount(pub usize);

/// The kind of physical payload a backing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag) struct BackingTag(pub u8);

/// The address of the allocation a backing is, or the scope id of an unshared one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag) struct BackingAddress(pub usize);

/// One physical allocation, identified across every handle that shares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::application::file_dag) struct BackingKey {
    pub tag: BackingTag,
    pub address: BackingAddress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct Backing {
    pub key: BackingKey,
    pub bytes: ByteCount,
    /// Immutable row-position capacity when this backing is optionally retained.
    pub retention_units: UnitCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived) enum LeaseClass {
    Required,
    /// Derived-view retention, subject to the view byte and position caps.
    Optional,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::application::file_dag::derived) struct Snapshot {
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
pub(in crate::application::file_dag::derived) struct LedgerEntry {
    pub bytes: ByteCount,
    pub retention_units: UnitCount,
    pub required: HoldCount,
    pub optional: HoldCount,
}

#[derive(Debug)]
pub(in crate::application::file_dag::derived) struct Ledger {
    pub entries: HashMap<BackingKey, LedgerEntry>,
    pub required_bytes: ByteCount,
    pub optional_bytes: ByteCount,
    pub view_optional_bytes: ByteCount,
    pub optional_units: UnitCount,
    pub overlap_bytes: ByteCount,
    pub unique_bytes: ByteCount,
    pub lease_metadata: ByteCount,
    pub peak_required_bytes: ByteCount,
    pub peak_optional_bytes: ByteCount,
    pub peak_view_optional_bytes: ByteCount,
    pub peak_optional_units: UnitCount,
    pub peak_overlap_bytes: ByteCount,
    pub peak_unique_bytes: ByteCount,
    pub peak_metadata_bytes: ByteCount,
}

/// One physical handle, shared by consumers through an outer `Arc`.
#[derive(Debug)]
pub(in crate::application::file_dag) struct ResourceLease {
    pub ledger: Rc<RefCell<Ledger>>,
    pub keys: Vec<BackingKey>,
    pub class: LeaseClass,
    pub metadata_bytes: ByteCount,
}

/// A backing with its immutable size, once the duplicates of a request are folded.
#[derive(Clone, Copy, Debug)]
pub(in crate::application::file_dag::derived) struct Measured {
    pub key: BackingKey,
    pub bytes: ByteCount,
    pub units: UnitCount,
}

/// What releasing a set of optional handles would give back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived) struct Credit {
    pub bytes: ByteCount,
    pub units: UnitCount,
}

/// Whether the backings of a request are held by optional views alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived) enum Dormancy {
    Dormant,
    Live,
}
