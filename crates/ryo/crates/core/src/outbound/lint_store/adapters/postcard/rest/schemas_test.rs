//! What the schemas of what the files handed accept: readiness and proofs read back as they were written.

use super::{rest_bytes, rest_from};
use crate::outbound::lint_store::adapters::postcard::buckets::schemas::{bucket_bytes, bucket_from};
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stores::types::{StoredHanded, StoredWords};

use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the schemas.
type Input = ();
/// Whether the schemas answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("named_frontier_readiness_roundtrips_independently_of_row_bytes"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("frontier_proof_roundtrip_preserves_partial_and_rejects_old_layout"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

fn named_frontier_readiness_roundtrips_independently_of_row_bytes() -> Value {
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
    true
}

fn frontier_proof_roundtrip_preserves_partial_and_rejects_old_layout() -> Value {
    use ryo_values::domain::{
        records::types::{FrontierAffected, FrontierCoverage, FrontierMissing},
        stored_facts::types::{Bucket, Digest},
        stored_findings::types::StoredFileNumber,
        stores::types::StoredFrontierProof,
    };
    let mut handed = StoredHanded::default();
    let mut partial = crate::outbound::lint_store::fixtures::proof();
    partial.rule = ryo_values::domain::records::types::ProducerRule(7);
    partial.input = ryo_values::domain::records::types::FrontierOrdinal(2);
    partial.rule_digest = Digest([9; 16]);
    partial.coverage = FrontierCoverage::Partial(vec![
        FrontierAffected::CurrentFile(FrontierMissing::Kinds {
            column: ryo_values::domain::plan_parts::types::ColumnIndex(2),
            kinds: vec![
                ryo_values::domain::stored_facts::types::PartialKind("function".into()),
                ryo_values::domain::stored_facts::types::PartialKind("variable".into()),
            ],
        }),
        FrontierAffected::Repository(FrontierMissing::Any),
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
    true
}

/// Runs both of the rows' scenarios.
fn subject() -> Value {
    named_frontier_readiness_roundtrips_independently_of_row_bytes()
        && frontier_proof_roundtrip_preserves_partial_and_rejects_old_layout()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
