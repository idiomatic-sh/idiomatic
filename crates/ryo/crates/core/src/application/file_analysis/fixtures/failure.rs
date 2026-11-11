//! The cases of read-failure demand across runs and across rules.
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::CorpusInventory;
use crate::outbound::corpus::types::PathStat;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::FileBytes;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub(crate) struct ChangingRead {
    pub(crate) reads: AtomicUsize,
    pub(crate) fail_at: usize,
    /// How many reads started with bytes already in the buffer: earlier source or a partial error's bytes.
    pub(crate) dirty_starts: AtomicUsize,
}

impl CorpusPort for ChangingRead {
    fn observe_entry(
        &self,
        path: &FilePath,
    ) -> ryo_language::domain::project_context::types::EntryObservation {
        ryo_language::domain::project_context::types::EntryObservation {
            path: path.clone(),
            state: ryo_language::domain::project_context::types::ObservedEntryState::Unknown,
        }
    }
    fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
        unreachable!("explicit test inventory")
    }
    fn stat(&self, _: &FilePath) -> Result<PathStat, CorpusError> {
        unreachable!("no dependency lookup")
    }
    fn read(&self, entry: &CorpusEntry, bytes: &mut FileBytes) -> Result<(), CorpusError> {
        if !bytes.0.is_empty() {
            let _previous = self.dirty_starts.fetch_add(1, Ordering::Relaxed);
        }
        let at = self.reads.fetch_add(1, Ordering::Relaxed);
        if at == self.fail_at {
            bytes.0.extend_from_slice(b"partial failed bytes");
            Err(CorpusError::Unreadable {
                path: entry.path.0.to_string(),
                reason: "read denied".into(),
            })
        } else {
            bytes.0.extend_from_slice(b"fn main() {}");
            Ok(())
        }
    }
}
