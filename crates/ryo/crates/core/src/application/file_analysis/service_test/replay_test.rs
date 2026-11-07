//! The cases of replayed buckets: selection before any read, and stale proofs refused.
use crate::domain::visit::types::Keeping;
use crate::domain::workers::service::tables_for;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::outbound::facts::fixtures::fake::FactsFake;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Handed;
use ryo_values::domain::records::types::HandedWords;
use ryo_values::domain::records::types::Handing;
use ryo_values::domain::records::types::Ran;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::records::types::RecordWords;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::row_texts::types::FindingRows;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::KeptRows;
use super::program_test::NOTHING_HANDED;
use super::program_test::NOTHING_STORED;
use super::program_test::one_capture_plan;
use super::program_test::prepared_test_program;
use super::program_test::replay_under;
use super::program_test::replay_under_capture;
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::types::RequestVisitSetting;
use ryo_execution::outbound::facts::interface::FactsPort;
use ryo_execution::outbound::matching::interface::MatchingPort;
pub(super) fn a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read() -> Result<usize, FileAnalysisError> {
    use ryo_values::domain::records::types::FrontierCoverage;
    use ryo_values::domain::stored_facts::types::{Bucket, Digest};
    let good = crate::outbound::lint_store::fixtures::proof();
    let selected = BucketsWanted::These(vec![Bucket(0)]);
    replay_under(good.clone(), &selected).expect("the proof of the prepared root is accepted");
    let mut wrong_digest = good.clone();
    wrong_digest.rule_digest = Digest([9; 16]);
    let mut wrong_ordinal = good.clone();
    wrong_ordinal.input = ryo_values::domain::records::types::FrontierOrdinal(1);
    let mut wrong_coverage = good.clone();
    wrong_coverage.coverage = FrontierCoverage::Partial(Vec::new());
    let refused = |result: Result<(), FileAnalysisError>| {
        matches!(
            result,
            Err(FileAnalysisError::Execution(
                ryo_execution::inbound::use_cases::execution::interface::OperatingError::ObservationIncomplete { .. }
            ))
        )
    };
    assert!(
        refused(replay_under_capture(good.clone(), &selected, false)),
        "a bucket whose frontier the prepared root does not capture is refused when selected"
    );
    replay_under_capture(good.clone(), &BucketsWanted::These(Vec::new()), false)
        .expect("the capture flag of a bucket the run does not select is never checked");
    for stale in [wrong_digest, wrong_ordinal, wrong_coverage] {
        assert!(
            matches!(
                replay_under(stale.clone(), &selected),
                Err(FileAnalysisError::Execution(
                    ryo_execution::inbound::use_cases::execution::interface::OperatingError::ObservationIncomplete { .. }
                ))
            ),
            "{stale:?} must be refused for a selected bucket"
        );
        replay_under(stale, &BucketsWanted::These(Vec::new()))
            .expect("the proof of a bucket the run does not select is never checked");
    }
    Ok(1)
}

pub(super) fn staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops() -> Result<usize, FileAnalysisError> {
    use super::super::types::{FileVisit, Worker};

    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::domain::workers::types::FileOrder;
    use crate::outbound::lint_store::interface::LintStorePort as _;
    use crate::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};
    use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value, Words};
    use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
    use ryo_values::domain::records::types::{
        FileOutcome, Handed, HandedWords, Handing, Ran, Record, RecordWords, StagedWords,
        StagingSession,
    };
    use ryo_values::domain::row_texts::types::FindingRows;
    use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, Digest};
    use ryo_values::domain::stores::types::BucketsWanted;
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;

    let directory = tempfile::tempdir().expect("snapshot directory");
    let store = crate::outbound::lint_store::factory::postcard(Store {
        directory: CacheDirectory(directory.path().to_string_lossy().into()),
        build: BuildIdentity("replay-staging".into()),
    });
    let snapshot = store.stored().expect("read snapshot");
    let new_worker = || Worker {
        tables: tables_for(WorkerNumber(0), &[]),
        facts: FactsFake::default().worker().expect("worker"),
        old_rows: None,
        failure: None,
    };
    let producer = new_worker();
    let mut words = Words::default();
    encode_table(
        &Table {
            kinds: vec![CellKind::Number],
            cells: vec![Value::Number(Integer(7))],
            count: RowCount(1),
        },
        &[],
        &mut words,
    );
    let packed = pack(WordSlice(&words.0));
    let ran = Ran {
        rows: FindingRows::default(),
        number: FileNumber(0),
        interface: Digest([1; 16]),
        noted: Digest([2; 16]),
        frontier: vec![],
        handed: Handing::Made(Handed {
            buckets: vec![HandedWords {
                bucket: Bucket(0),
                proof: crate::outbound::lint_store::fixtures::proof(),
                handed: packed.into(),
            }],
            text: ArenaBytes::default(),
            ..Handed::default()
        }),
    };
    let outcome =
        FileAnalysis::stage_outcome(Some(snapshot.as_ref()), Some(FileOutcome::Read(ran)))
            .expect("normalized outcome staged")
            .expect("outcome");
    drop(producer);
    let mut record = Record {
        executable: Default::default(),
        path: FilePath("case.py".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
        digest: Digest([3; 16]),
        number: FileNumber(0),
        outcome,
    };
    let FileOutcome::Read(ran) = &mut record.outcome else {
        panic!("read")
    };
    let Handing::Made(handed) = &mut ran.handed else {
        panic!("fresh")
    };
    handed.buckets.push(HandedWords {
        bucket: Bucket(1),
        proof: crate::outbound::lint_store::fixtures::proof(),
        handed: RecordWords::Staged(StagedWords {
            session: StagingSession(u64::MAX),
            offset: ryo_values::domain::records::types::PayloadOffset(u64::MAX),
            length: ryo_values::domain::records::types::PayloadLength(9),
        }),
    });
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: ryo_execution::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let wanted = BucketsWanted::These(vec![Bucket(0)]);
    // The proofs the replay checks are those of this program: one rule capturing one frontier input, its first bucket 0.
    let plan = one_capture_plan();
    let plans = [plan];
    let ids = [RuleId("capture".into())];
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let visit = FileVisit {
        store: Some(snapshot.as_ref()),
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
                reuse: &ryo_values::domain::stores::types::Reuse::Cross(
                    ryo_values::domain::stores::types::MadeAgain::All,
                ),
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &wanted,
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
    let mut replay = new_worker();
    FileAnalysis::hand_again(&mut replay, &visit, FileOrder(0), FileId(0), &record)
        .expect("invalid unrequested span is not read");
    let all_visit = FileVisit {
        setting: RequestVisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                wanted_buckets: &BucketsWanted::All,
                ..(visit.setting.clone()).domain
            },
            ..visit.setting.clone()
        },
        store: visit.store,
    };
    // Once the bucket is selected its staged descriptor is read, and the store refuses a span of another session as unreadable.
    assert!(matches!(
        FileAnalysis::hand_again(&mut replay, &all_visit, FileOrder(0), FileId(0), &record),
        Err(FileAnalysisError::Store(
            crate::outbound::lint_store::errors::LintStoreError::Unreadable { .. }
        ))
    ));

    // The persistent old-row path uses the same filter-before-resolution contract.
    use ryo_values::domain::stored_findings::types::StoredFileNumber;
    use ryo_values::domain::stores::types::{
        KeptRows, OldWords, ReadWordPayload, ReadWords, StoreSnapshot,
    };
    let mut valid = ryo_values::domain::stored_facts::types::PackedWords::default();
    let FileOutcome::Read(ran) = &record.outcome else {
        panic!("read")
    };
    let Handing::Made(handed) = &ran.handed else {
        panic!("made")
    };
    let RecordWords::Staged(descriptor) = &handed.buckets[0].handed else {
        panic!("staged")
    };
    snapshot
        .read_staged(descriptor, &mut valid)
        .expect("valid packed words");
    let mut persisted = crate::outbound::lint_store::fixtures::tables::stored(1);
    persisted.files[0].path = record.path.clone();
    persisted.handed.frontier[0].bucket = Bucket(0);
    persisted.handed.proofs[0].bucket = Bucket(0);
    persisted.handed.frontier[0].words = valid;
    snapshot
        .keep(persisted.clone(), &KeptRows::default())
        .expect("persist old fixture");
    let old = store.stored().expect("old snapshot");
    let mut old_handed = old.handed(&BucketsWanted::All).expect("headers only");
    let invalid = OldWords {
        snapshot: StoreSnapshot(u64::MAX),
        bucket: Bucket(1),
        file: StoredFileNumber(0),
        header: ryo_values::domain::records::types::HeaderOffset(u64::MAX),
        offset: ryo_values::domain::records::types::PayloadOffset(u64::MAX),
        length: ryo_values::domain::records::types::PayloadLength(9),
    };
    old_handed.frontier.push(ReadWords {
        file: StoredFileNumber(0),
        bucket: Bucket(1),
        proof: crate::outbound::lint_store::fixtures::proof(),
        payload: ReadWordPayload::Old(invalid),
    });
    old_handed.frontier.push(ReadWords {
        file: StoredFileNumber(1),
        bucket: Bucket(0),
        proof: crate::outbound::lint_store::fixtures::proof(),
        payload: ReadWordPayload::Old(invalid),
    });
    let mut old_record = record.clone();
    let FileOutcome::Read(ran) = &mut old_record.outcome else {
        panic!("read")
    };
    ran.handed = Handing::Kept;
    let old_visit = FileVisit {
        store: Some(old.as_ref()),
        setting: RequestVisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                stored: &persisted,
                handed: &old_handed,
                ..(visit.setting.clone()).domain
            },
            ..visit.setting.clone()
        },
    };
    FileAnalysis::hand_again(
        &mut replay,
        &old_visit,
        FileOrder(0),
        FileId(0),
        &old_record,
    )
    .expect("unrequested bucket and other file never resolve their invalid descriptors");
    let old_all = FileVisit {
        store: old_visit.store,
        setting: RequestVisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                wanted_buckets: &BucketsWanted::All,
                ..(old_visit.setting).domain
            },
            ..old_visit.setting
        },
    };
    // Once the bucket is selected its old descriptor is resolved, and the store refuses a row of another snapshot as unreadable.
    assert!(matches!(
        FileAnalysis::hand_again(&mut replay, &old_all, FileOrder(0), FileId(0), &old_record),
        Err(FileAnalysisError::Store(
            crate::outbound::lint_store::errors::LintStoreError::Unreadable { .. }
        ))
    ));
    Ok(1)
}
