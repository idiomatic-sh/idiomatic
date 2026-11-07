//! A store keeps the proofs of the buckets it copied from the store before, under the number each file has now.
use super::service::carry_proofs;
use super::types::{FileAgain, KeptRows, StoredFrontierProof, StoredHanded};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::records::types::{FrontierCoverage, FrontierProof};
use crate::domain::stored_facts::types::{Bucket, Digest};
use crate::domain::stored_findings::types::StoredFileNumber;
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    UnchangedBucket,
    BucketAlreadyMade,
    FileUsedAgain,
    FileLeft,
}
type Input = Scenario;
type Value = Cow<'static, [StoredFrontierProof]>;
type Failure = std::convert::Infallible;

const fn proof(file: u32, bucket: u32) -> StoredFrontierProof {
    StoredFrontierProof {
        file: StoredFileNumber(file),
        bucket: Bucket(bucket),
        proof: FrontierProof {
            rule: crate::domain::records::types::ProducerRule(0),
            input: crate::domain::records::types::FrontierOrdinal(0),
            rule_digest: Digest([0; 16]),
            coverage: FrontierCoverage::Complete,
        },
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_proof_of_an_unchanged_bucket_is_carried"),
        word: Word::Succeeds,
        input: || Scenario::UnchangedBucket,
        expect: Expect::Value(Cow::Borrowed(&[proof(0, 1)])),
    },
    Case {
        name: CaseName("a_bucket_made_this_run_keeps_its_one_proof"),
        word: Word::Succeeds,
        input: || Scenario::BucketAlreadyMade,
        expect: Expect::Value(Cow::Borrowed(&[proof(0, 1)])),
    },
    Case {
        name: CaseName("the_proof_of_a_file_used_again_moves_to_its_new_number"),
        word: Word::Succeeds,
        input: || Scenario::FileUsedAgain,
        expect: Expect::Value(Cow::Borrowed(&[proof(7, 2)])),
    },
    Case {
        name: CaseName("the_proof_of_a_file_no_longer_used_is_dropped"),
        word: Word::Succeeds,
        input: || Scenario::FileLeft,
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let mut old = StoredHanded::default();
    let mut current = StoredHanded::default();
    let mut kept = KeptRows::default();
    match input {
        Scenario::UnchangedBucket => {
            old.proofs.push(proof(0, 1));
            kept.unchanged.push(Bucket(1));
        }
        Scenario::BucketAlreadyMade => {
            old.proofs.push(proof(0, 1));
            current.proofs.push(proof(0, 1));
            kept.unchanged.push(Bucket(1));
        }
        Scenario::FileUsedAgain => {
            old.proofs.push(proof(4, 2));
            kept.file_again.push(FileAgain {
                was: StoredFileNumber(4),
                now: StoredFileNumber(7),
            });
        }
        Scenario::FileLeft => {
            old.proofs.push(proof(4, 2));
        }
    }
    carry_proofs(&mut current, &old, &kept);
    Ok(Cow::Owned(std::mem::take(&mut current.proofs)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod retained_test;
