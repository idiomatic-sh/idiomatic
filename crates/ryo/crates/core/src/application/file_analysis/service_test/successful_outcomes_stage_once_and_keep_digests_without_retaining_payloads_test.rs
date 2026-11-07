//! The scenario `successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads` of the cases of this file.

use super::*;

/// Only post-normalization outcomes can reach the staging boundary; IO failures remain failures.
pub(super) fn successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads() -> Result<Value, Failure> {
    use successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads_doubles::StagingRead;
    let outcome = FileOutcome::Read(Ran {
        rows: FindingRows::default(),
        number: FileNumber(0),
        interface: Digest([1; 16]),
        noted: Digest([2; 16]),
        frontier: vec![],
        handed: Handing::Made(Handed {
            buckets: (0..64)
                .map(|bucket| HandedWords {
                    bucket: Bucket(bucket),
                    proof: crate::outbound::lint_store::fixtures::proof(),
                    words: PackedWords(vec![7; 4096]).into(),
                })
                .collect(),
            ..Handed::default()
        }),
    });
    let read = StagingRead::default();
    assert!(
        FileAnalysis::stage_outcome(Some(&read), None)
            .expect("no outcome")
            .is_none()
    );
    assert_eq!(read.calls.load(Ordering::Relaxed), 0);
    assert_eq!(
        FileAnalysis::stage_outcome(None, Some(outcome.clone())).expect("no store"),
        Some(outcome.clone())
    );
    let Some(FileOutcome::Read(ran)) =
        FileAnalysis::stage_outcome(Some(&read), Some(outcome.clone())).expect("stage")
    else {
        panic!("read")
    };
    assert_eq!(ran.interface, Digest([1; 16]));
    assert_eq!(ran.noted, Digest([2; 16]));
    let Handing::Made(handed) = ran.handed else {
        panic!("fresh outcome")
    };
    assert_eq!(read.calls.load(Ordering::Relaxed), 64);
    assert!(handed.buckets.iter().all(|row| matches!(
        row.words,
        RecordWords::Staged(StagedWords {
            length: ryo_values::domain::records::types::PayloadLength(4096),
            ..
        })
    )));
    let failing = StagingRead {
        fail: true,
        ..StagingRead::default()
    };
    assert!(matches!(
        FileAnalysis::stage_outcome(Some(&failing), Some(outcome)),
        Err(FileAnalysisError::Store(LintStoreError::NothingRead))
    ));
    assert_eq!(failing.calls.load(Ordering::Relaxed), 1);
    Ok(1)
}

mod successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads_doubles {
    use super::*;
    use crate::outbound::lint_store::errors::LintStoreError;
    use crate::outbound::lint_store::interface::ReadStore;
    use ryo_values::domain::files::types::FileNumber;
    use ryo_values::domain::records::types::{
        FileOutcome, Handed, HandedWords, Handing, Ran, RecordWords, StagedWords, StagingSession,
    };
    use ryo_values::domain::row_texts::types::FindingRows;
    use ryo_values::domain::stored_facts::types::{Bucket, Digest, PackedWords};
    use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored};
    use std::sync::atomic::AtomicUsize;

    #[derive(Debug, Default)]
    pub(super) struct StagingRead {
        pub(super) calls: AtomicUsize,
        pub(super) fail: bool,
    }

    impl ReadStore for StagingRead {
        fn tables(&self) -> Stored {
            Stored::default()
        }
        fn handed(
            &self,
            _: &BucketsWanted,
        ) -> Result<ryo_values::domain::stores::types::ReadHanded, LintStoreError> {
            Ok(ryo_values::domain::stores::types::ReadHanded::default())
        }
        fn read_old(
            &self,
            _words: &ryo_values::domain::stores::types::OldWords,
            _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
        ) -> Result<(), LintStoreError> {
            Err(LintStoreError::NothingRead)
        }

        fn begin_old_rows(
            &self,
            _wanted: &ryo_values::domain::stores::types::BucketsWanted,
        ) -> Result<Option<Box<dyn crate::outbound::lint_store::interface::OldRowBatch>>, LintStoreError> {
            Ok(None)
        }

        fn read_staged(
            &self,
            _words: &ryo_values::domain::records::types::StagedWords,
            _into: &mut ryo_values::domain::stored_facts::types::PackedWords,
        ) -> Result<(), LintStoreError> {
            Err(LintStoreError::NothingRead)
        }

        fn keep(&self, _: Stored, _: &KeptRows) -> Result<(), LintStoreError> {
            Ok(())
        }
        fn stage_words(&self, words: PackedWords) -> Result<RecordWords, LintStoreError> {
            let offset = self.calls.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                return Err(LintStoreError::NothingRead);
            }
            Ok(RecordWords::Staged(StagedWords {
                session: StagingSession(1),
                offset: ryo_values::domain::records::types::PayloadOffset(
                    u64::try_from(offset).expect("small count"),
                ),
                length: ryo_values::domain::records::types::PayloadLength(
                    u64::try_from(words.0.len()).expect("size"),
                ),
            }))
        }
    }
}
