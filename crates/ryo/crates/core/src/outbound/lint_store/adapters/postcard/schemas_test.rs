//! What the schemas of this adapter accept and refuse, one row of `CASES` for each input.

use super::super::types::TableOfContents;
use super::{
    bucket_bytes, bucket_from, contents, contents_bytes, rest_bytes, rest_from, tables_bytes,
    tables_of,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::lint_store::fixtures::fake::stored;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::runs::types::{RuleId, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};
use ryo_values::domain::stored_findings::types::StoredFinding;
use ryo_values::domain::stores::types::{Stored, StoredHanded, StoredWords};

/// What a row offers the schemas.
#[derive(Clone, Copy, Debug)]
enum Offered {
    /// Each section written and read back.
    TheSameBuild,
    /// A table of contents read back by another build.
    AnotherBuild,
    /// Tables cut short.
    CutShort,
    /// A thousand findings in one file: how many times its path is written.
    APathOfManyFindings,
}

type Input = Offered;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_section_reads_back_as_it_was_written"),
        word: Word::Succeeds,
        input: || Offered::TheSameBuild,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("another_builds_table_of_contents_reads_as_nothing"),
        word: Word::Succeeds,
        input: || Offered::AnotherBuild,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("tables_cut_short_read_as_nothing"),
        word: Word::Succeeds,
        input: || Offered::CutShort,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_text_is_written_once_however_many_rows_name_it"),
        word: Word::Succeeds,
        input: || Offered::APathOfManyFindings,
        expect: Expect::Value(true),
    },
];

/// The bytes of the small tables of `stored`.
fn written(stored: &Stored) -> Vec<u8> {
    tables_bytes(stored).expect("written")
}

/// `stored` without what its files handed: what the small tables hold.
fn tables(mut stored: Stored) -> Stored {
    stored.handed = StoredHanded::default();
    stored
}

/// Whether what the files of `stored` handed reads back from its sections: the rest, and the words of each bucket.
fn handed_reads_back(stored: &Stored) -> bool {
    let rest = rest_bytes(&stored.handed)
        .ok()
        .and_then(|bytes| rest_from(&bytes));
    let mut buckets: Vec<Bucket> = stored
        .handed
        .frontier
        .iter()
        .map(|each| each.bucket)
        .collect();
    buckets.sort_unstable();
    buckets.dedup();
    let mut words = Vec::new();
    for bucket in buckets {
        let rows: Vec<&StoredWords> = stored
            .handed
            .frontier
            .iter()
            .filter(|each| each.bucket == bucket)
            .collect();
        let Some(read) = bucket_bytes(&rows)
            .ok()
            .and_then(|bytes| bucket_from(bucket, &bytes, &stored.handed.proofs))
        else {
            return false;
        };
        words.extend(read);
    }
    rest.is_some_and(|rest| {
        StoredHanded(ryo_values::domain::stores::types::HandedRows {
            frontier: words,
            ..rest.0
        }) == stored.handed
    })
}

fn subject(offered: Input) -> Value {
    match offered {
        Offered::TheSameBuild => {
            let mut stored = stored(1);
            stored.skipped = vec![Skipped {
                rule: RuleId("syntax-only".into()),
                path: FilePath("src/1/rejected.py".into()),
                reason: SkipReason::SyntaxRejected,
            }];
            tables_of(&written(&stored)) == tables(stored.clone()) && handed_reads_back(&stored)
        }
        Offered::AnotherBuild => {
            let bytes = contents_bytes("b", &TableOfContents::default()).expect("written");
            contents("b", &bytes).is_some() && contents("c", &bytes).is_none()
        }
        Offered::CutShort => {
            let mut bytes = written(&stored(1));
            bytes.truncate(bytes.len() - 3);
            tables_of(&bytes) == Stored::default()
        }
        Offered::APathOfManyFindings => {
            let mut many = tables(stored(4));
            let path = FilePath("src/a/very/long/path/of/the/file.rs".into());
            let template = many.findings.first().cloned();
            many.findings = (0..1000)
                .filter_map(|_| template.clone())
                .map(|each| StoredFinding {
                    subject: Some(Cell::File(path.0.clone())).into(),
                    ..each
                })
                .collect();
            let bytes = written(&many);
            let needle = path.0.as_bytes();
            bytes
                .windows(needle.len())
                .filter(|each| *each == needle)
                .count()
                == 1
                && tables_of(&bytes) == many
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |offered| Ok(subject(offered)));
}

#[test]
fn named_frontier_readiness_roundtrips_independently_of_row_bytes() {
    use ryo_values::domain::records::types::ObservationReadinessCount;
    use ryo_values::domain::records::types::ObservationReadinessLayout;
    use ryo_values::domain::records::types::ObservationReadinessSlot;
    use ryo_values::domain::records::types::ObservationReason;
    use ryo_values::domain::records::types::ProducerReadiness;
    use ryo_values::domain::{
        observations::service::{new_frontiers, set_frontier},
        stored_facts::types::Digest,
        stored_findings::types::StoredFileNumber,
        stores::types::StoredFrontierReadiness,
    };
    let mut handed = StoredHanded::default();
    let mut root = new_frontiers(
        ObservationReadinessLayout(Digest([7; 16])),
        ObservationReadinessCount(5),
    );
    for (index, readiness) in [
        ProducerReadiness::Complete,
        ProducerReadiness::Unknown(ObservationReason("exact reason\tλ\0text".into())),
        ProducerReadiness::Skipped,
        ProducerReadiness::NoQuery,
    ]
    .into_iter()
    .enumerate()
    {
        set_frontier(
            &mut root,
            ObservationReadinessSlot(u32::try_from(index).unwrap()),
            readiness,
        )
        .unwrap();
    }
    handed.observations.push(StoredFrontierReadiness {
        file: StoredFileNumber(0),
        root: std::sync::Arc::new(root),
    });
    let bytes = rest_bytes(&handed).expect("current rest schema");
    assert_eq!(rest_from(&bytes), Some(handed));
    assert!(
        rest_from(&bytes[..bytes.len() - 1]).is_none(),
        "truncated readiness cannot become a complete empty source"
    );
}

#[test]
fn frontier_proof_roundtrip_preserves_partial_and_rejects_old_layout() {
    use ryo_values::domain::{
        records::types::{FrontierAffected, FrontierCoverage},
        stored_facts::types::{Bucket, Digest},
        stored_findings::types::StoredFileNumber,
        stores::types::StoredFrontierProof,
    };
    let mut handed = StoredHanded::default();
    let mut partial = crate::outbound::lint_store::fixtures::proof();
    partial.rule = 7;
    partial.input = 2;
    partial.rule_digest = Digest([9; 16]);
    partial.coverage = FrontierCoverage::Partial(vec![
        FrontierAffected::CurrentFile,
        FrontierAffected::Repository,
    ]);
    handed.proofs.push(StoredFrontierProof {
        file: StoredFileNumber(3),
        bucket: Bucket(11),
        proof: partial.clone(),
    });
    handed.frontier.push(StoredWords {
        file: StoredFileNumber(3),
        bucket: Bucket(11),
        proof: partial.clone(),
        words: PackedWords(vec![2, 0, 0]),
    });
    let bytes = rest_bytes(&handed).expect("typed proof");
    let mut restored = rest_from(&bytes).expect("versioned proof table");
    let payload = bucket_bytes(&[&handed.frontier[0]]).expect("bucket bytes");
    restored.frontier = bucket_from(Bucket(11), &payload, &restored.proofs)
        .expect("exact source proof accompanies bucket bytes");
    assert_eq!(restored, handed);
    let read = ryo_values::domain::stores::service::read_handed(restored);
    assert_eq!(read.frontier[0].proof, partial);
    assert_eq!(read.frontier[0].file, StoredFileNumber(3));
    assert_eq!(read.frontier[0].bucket, Bucket(11));

    let mut wrong_root = handed.clone();
    wrong_root.proofs[0].bucket = Bucket(12);
    let wrong = rest_from(&rest_bytes(&wrong_root).unwrap()).unwrap();
    assert!(bucket_from(Bucket(11), &payload, &wrong.proofs).is_none());

    let mut complete = StoredHanded::default();
    complete.proofs.push(StoredFrontierProof {
        file: StoredFileNumber(0),
        bucket: Bucket(1),
        proof: crate::outbound::lint_store::fixtures::proof(),
    });
    assert_eq!(rest_from(&rest_bytes(&complete).unwrap()), Some(complete));
}
