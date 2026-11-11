//! The counters of the ledger as a value, and the peaks they have reached.
use super::{
    capacity::table,
    types::{BackingKey, ByteCount, LedgerEntry, LedgerSnapshot, SlotCount},
};
use crate::application::file_dag::derived::ledger::service::Ledger;

impl Ledger {
    pub(crate) fn snapshot(&self) -> LedgerSnapshot {
        LedgerSnapshot {
            required_bytes: self.required_bytes,
            optional_bytes: self.optional_bytes,
            view_optional_bytes: self.view_optional_bytes,
            optional_units: self.optional_units,
            overlap_bytes: self.overlap_bytes,
            unique_bytes: self.unique_bytes,
            metadata_bytes: self.metadata_bytes(),
            peak_required_bytes: self.peak_required_bytes,
            peak_optional_bytes: self.peak_optional_bytes,
            peak_view_optional_bytes: self.peak_view_optional_bytes,
            peak_optional_units: self.peak_optional_units,
            peak_overlap_bytes: self.peak_overlap_bytes,
            peak_unique_bytes: self.peak_unique_bytes,
            peak_metadata_bytes: self.peak_metadata_bytes,
        }
    }

    fn metadata_bytes(&self) -> ByteCount {
        // A conservative bucket estimate plus the actual reserved lease key
        // capacities. No iteration over live resources is needed for a snapshot.
        let own = ByteCount(size_of::<Self>());
        let entries = table::<(BackingKey, LedgerEntry)>(SlotCount(self.entries.capacity()));
        own.saturating_add(entries)
            .saturating_add(self.lease_metadata)
    }

    pub(super) fn note_peaks(&mut self) {
        self.peak_required_bytes = self.peak_required_bytes.max(self.required_bytes);
        self.peak_optional_bytes = self.peak_optional_bytes.max(self.optional_bytes);
        self.peak_view_optional_bytes = self.peak_view_optional_bytes.max(self.view_optional_bytes);
        self.peak_optional_units = self.peak_optional_units.max(self.optional_units);
        self.peak_overlap_bytes = self.peak_overlap_bytes.max(self.overlap_bytes);
        self.peak_unique_bytes = self.peak_unique_bytes.max(self.unique_bytes);
        self.peak_metadata_bytes = self.peak_metadata_bytes.max(self.metadata_bytes());
    }
}
