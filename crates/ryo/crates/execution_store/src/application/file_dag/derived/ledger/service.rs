//! The ledger and the leases it issues, read and changed only by the ledger part.
use super::types::{BackingKey, ByteCount, LeaseClass, LedgerEntry, UnitCount};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

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
