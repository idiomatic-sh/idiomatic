//! The scenario `required_dependency_stat_and_read_errors_cannot_disappear` of the cases of this file.

use super::*;

pub(super) fn required_dependency_stat_and_read_errors_cannot_disappear() -> Result<Value, Failure> {
    use required_dependency_stat_and_read_errors_cannot_disappear_doubles::Refuses;
    for stat in [false, true] {
        let service = FileAnalysis::new(
            Arc::new(Refuses { stat }),
            Arc::new(LanguageFake::default()),
            Arc::new(DigestFake),
            Arc::new(MemoryFake::default()),
        );
        let mut result = crate::domain::project::types::Projected::default();
        assert!(matches!(
            service.consulted(&[FilePath("required".into())], &mut result),
            Err(FileAnalysisError::Corpus(_))
        ));
        assert!(result.consulted.is_empty() && result.consulted_absent.is_empty());
    }
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake::default()),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let mut result = crate::domain::project::types::Projected::default();
    service
        .consulted(&[FilePath("absent".into())], &mut result)
        .expect("a genuine missing dependency is recordable");
    assert_eq!(result.consulted_absent, vec![FilePath("absent".into())]);
    Ok(1)
}

mod required_dependency_stat_and_read_errors_cannot_disappear_doubles {
    use super::*;
    use crate::outbound::corpus::{
        errors::CorpusError,
        interface::CorpusPort,
        types::{CorpusInventory, PathStat},
    };
    use ryo_values::domain::files::types::{CorpusEntry, FileBytes, FileSize, Nanoseconds};

    pub(super) struct Refuses {
        pub(super) stat: bool,
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
}
