//! The cases of syntax applicability, recovery partials and required project dependencies.
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::CorpusInventory;
use crate::outbound::corpus::types::PathStat;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;

pub(crate) struct Refuses {
    pub(crate) stat: bool,
}

impl CorpusPort for Refuses {
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
        Ok(CorpusInventory(vec![]))
    }
    fn read(&self, _: &CorpusEntry, _: &mut FileBytes) -> Result<(), CorpusError> {
        Err(CorpusError::Unreadable {
            path: "required".into(),
            reason: "read denied".into(),
        })
    }
    fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError> {
        if self.stat {
            Err(CorpusError::Unreadable {
                path: path.0.to_string(),
                reason: "stat denied".into(),
            })
        } else {
            Ok(PathStat::Found(CorpusEntry {
                executable: Default::default(),
                path: path.clone(),
                size: FileSize(1),
                modified: Nanoseconds(1),
            }))
        }
    }
}
