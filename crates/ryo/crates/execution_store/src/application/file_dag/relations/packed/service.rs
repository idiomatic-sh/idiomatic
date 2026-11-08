//! Owned cross-root words use the same required backing ledger as resident publications.
use crate::application::file_dag::derived::ledger::types::{Backing, ResourceLease};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords};
use std::sync::Arc;

/// Coordinator-local lease. Only the immutable words, never this lease, enter a worker closure.
#[derive(Clone)]
pub(crate) struct PackedPublication {
    _words: Arc<KeptWords>,
    _lease: Arc<ResourceLease>,
}

mod issuance;
