//! The finished worker record cases exercise its original staged receipt and ordered delivery.
use super::value::WorkerRecord;
use ryo_language::domain::files::types::FilePath;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::records::types::{
    FileOutcome, FrontierCoverage, FrontierOrdinal, FrontierProof, Handed, HandedWords, Handing,
    PayloadLength, PayloadOffset, ProducerRule, Ran, Record, RecordWords, StagedWords,
    StagingSession,
};
use ryo_values::domain::stored_facts::types::{Bucket, Digest};
enum Scenario {
    Complete,
    Partial,
    MixedSessions,
}
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Shared,
    Original,
    Changed,
}
type Input = Scenario;
type Value = Outcome;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_completed_file_shares_its_layout_and_restores_every_ordered_range"),
        word: Word::Succeeds,
        input: || Scenario::Complete,
        expect: Expect::Value(Outcome::Shared),
    },
    Case {
        name: CaseName("a_partial_proof_preserves_its_original_record"),
        word: Word::Succeeds,
        input: || Scenario::Partial,
        expect: Expect::Value(Outcome::Original),
    },
    Case {
        name: CaseName("mixed_staging_sessions_preserve_the_original_record"),
        word: Word::Succeeds,
        input: || Scenario::MixedSessions,
        expect: Expect::Value(Outcome::Original),
    },
];
fn subject(given: Input) -> Result<Value, Failure> {
    let row = |offset| HandedWords {
        bucket: Bucket(5),
        proof: FrontierProof {
            rule: ProducerRule(1),
            input: FrontierOrdinal(2),
            rule_digest: Digest([4; 16]),
            coverage: FrontierCoverage::Complete,
        },
        handed: RecordWords::Staged(StagedWords {
            session: StagingSession(2),
            offset: PayloadOffset(offset),
            length: PayloadLength(3),
        }),
    };
    let mut buckets = vec![row(15), row(18), row(88)];
    match given {
        Scenario::Complete => {}
        Scenario::Partial => buckets[1].proof.coverage = FrontierCoverage::Partial(Vec::new()),
        Scenario::MixedSessions => {
            let RecordWords::Staged(words) = &mut buckets[1].handed else {
                return Err(());
            };
            words.session = StagingSession(3);
        }
    }
    let record = Record {
        executable: Default::default(),
        path: FilePath("a.rs".into()),
        size: FileSize(2),
        modified: Nanoseconds(8),
        digest: Digest([3; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Read(Ran {
            rows: Default::default(),
            number: FileNumber(0),
            interface: Digest([0; 16]),
            noted: Digest([7; 16]),
            frontier: Vec::new(),
            handed: Handing::Made(Handed {
                buckets,
                ..Default::default()
            }),
        }),
    };
    let mut worker = WorkerRecord::from(record.clone());
    let mut template = None;
    worker.park(&mut template);
    Ok(if worker.into_record() != record {
        Outcome::Changed
    } else if template.is_some() {
        Outcome::Shared
    } else {
        Outcome::Original
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
