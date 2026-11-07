//! What the schemas of the frontier proofs accept: a proof reads back as it was written, complete or partial.

use super::{proof_row, proof_schema};
use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage, FrontierMissing};
use ryo_values::domain::stored_facts::types::{Bucket, PartialKind};
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::StoredFrontierProof;

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
        name: CaseName("a_frontier_proof_reads_back_as_it_was_written"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

/// Writes a complete proof and a partial one and reads each back.
fn subject() -> Value {
    let complete = StoredFrontierProof {
        file: StoredFileNumber(0),
        bucket: Bucket(1),
        proof: crate::outbound::lint_store::fixtures::proof(),
    };
    let mut proof = crate::outbound::lint_store::fixtures::proof();
    proof.coverage = FrontierCoverage::Partial(vec![
        FrontierAffected::CurrentFile(FrontierMissing::Kinds {
            column: ryo_values::domain::plan_parts::types::ColumnIndex(2),
            kinds: vec![PartialKind("function".into())],
        }),
        FrontierAffected::Repository(FrontierMissing::Any),
    ]);
    let partial = StoredFrontierProof {
        file: StoredFileNumber(3),
        bucket: Bucket(11),
        proof,
    };
    [complete, partial]
        .into_iter()
        .all(|row| proof_row(&proof_schema(&row)) == Ok(row))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
