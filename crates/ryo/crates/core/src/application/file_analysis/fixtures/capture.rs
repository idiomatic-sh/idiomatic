//! The cases of captured frontier buckets and the staging of fresh outcomes.
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::types::RequestVisitSetting;
use super::NOTHING_HANDED;
use super::NOTHING_STORED;
use super::NOTHING_WANTED;
use super::prepared_test_program;
use crate::domain::visit::types::Keeping;
use crate::domain::workers::service::tables_for;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::ReadStore;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Handed;
use ryo_values::domain::records::types::HandedWords;
use ryo_values::domain::records::types::Handing;
use ryo_values::domain::records::types::Ran;
use ryo_values::domain::records::types::RecordWords;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::row_texts::types::FindingRows;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::KeptRows;
use ryo_values::domain::stores::types::Stored;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
pub(crate) fn a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket()
-> Result<usize, FileAnalysisError> {
    use super::super::types::{FileVisit, Worker};

    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestCounting;
    use ryo_relations::domain::relations::types::{
        CellKind, FileId, Integer, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash, Value,
        Words,
    };
    use ryo_values::domain::records::types::Kept;
    use ryo_values::domain::stored_facts::types::{ArenaBase, Bucket, KeptWords};
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: ryo_values::domain::plans::types::Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
        }],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![
            ryo_values::domain::plan_parts::types::Input::Step(StepId(0)),
            ryo_values::domain::plan_parts::types::Input::Step(StepId(0)),
        ],
    };
    let plans = [plan];
    let ids = [RuleId("capture".into())];
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: ryo_execution::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let visit = FileVisit {
        store: None,
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                metadata: &metadata,
                wanted: &[],
                paths: &[],
                first_bucket: &[Bucket(0)],
                ids: &ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &[],
                reuse: &ryo_values::domain::stores::types::Reuse::Whole,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &NOTHING_WANTED,
                budget: ryo_values::domain::runs::types::MemoryBytes(u64::MAX),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs: &[],
                answering: &crate::domain::render::types::Answering::Findings,
                keeping: Keeping::Keeps,
                unknown: &[],
            },
        },
    };
    let digest = Arc::new(DigestCounting::default());
    let analysis = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake::default()),
        digest.clone(),
        Arc::new(MemoryFake::default()),
    );
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0)),
        facts: FactsFake::default().worker().expect("facts worker"),
        old_rows: None,
        failure: None,
    };
    let packed = |value, places: &[NamePlace]| {
        let mut words = Words::default();
        let kind = if matches!(value, Value::File(_)) {
            CellKind::File
        } else if matches!(value, Value::Name(_)) {
            CellKind::Symbol
        } else {
            CellKind::Number
        };
        encode_table(
            &Table {
                kinds: vec![kind],
                cells: vec![value],
                count: RowCount(1),
            },
            places,
            &mut words,
        );
        KeptWords(pack(WordSlice(&words.0)))
    };
    let proof = |input| ryo_values::domain::records::types::FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(0),
        input: ryo_values::domain::records::types::FrontierOrdinal(input),
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    };
    let good = Kept {
        bucket: Bucket(0),
        proof: proof(0),
        words: packed(Value::Number(Integer(7)), &[]),
    };
    for (value, places) in [
        (Value::File(FileId(99)), vec![]),
        (
            Value::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(99)),
                file: FileId(0),
                start: Offset(9),
                end: Offset(12),
            }],
        ),
        (
            Value::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(0)),
                file: FileId(0),
                start: Offset(2),
                end: Offset(4),
            }],
        ),
    ] {
        worker.tables.captured = vec![
            good.clone(),
            Kept {
                bucket: Bucket(1),
                proof: proof(1),
                words: packed(value, &places),
            },
        ];
        assert!(
            analysis
                .recorded(&worker, &visit, FileId(0), ArenaBase(7))
                .is_none()
        );
        assert_eq!(
            digest.digests.load(Ordering::Relaxed),
            0,
            "no digest before every captured remap succeeds"
        );
    }
    worker.tables.captured = vec![good];
    assert!(
        analysis
            .recorded(&worker, &visit, FileId(0), ArenaBase(7))
            .is_some()
    );
    assert_eq!(
        digest.digests.load(Ordering::Relaxed),
        2,
        "nonvacuous rule-flat then noted digest control"
    );
    Ok(1)
}

/// Only post-normalization outcomes can reach the staging boundary; IO failures remain failures.
pub(crate) fn successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads()
-> Result<usize, FileAnalysisError> {
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
                    handed: PackedWords(vec![7; 4096]).into(),
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
        row.handed,
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

#[derive(Debug, Default)]
pub(crate) struct StagingRead {
    calls: AtomicUsize,
    fail: bool,
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
    ) -> Result<Option<Box<dyn crate::outbound::lint_store::interface::OldRowBatch>>, LintStoreError>
    {
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
