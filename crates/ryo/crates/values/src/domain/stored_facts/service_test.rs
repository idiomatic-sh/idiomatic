//! The `stored_facts` service's cases, on values alone.

use super::service::frontier_bucket;
use super::types::Bucket;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::InputNumber;

/// What a row of the cases asks of the service: the rule's first bucket and the frontier input.
type Input = (u32, usize);
/// The bucket the input's rows wait in.
type Value = u32;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_first_input_waits_in_the_first_bucket"),
        word: Word::Succeeds,
        input: || (7, 0),
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("each_later_input_waits_one_bucket_further"),
        word: Word::Succeeds,
        input: || (7, 3),
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("a_bucket_past_the_last_stays_at_the_last"),
        word: Word::Succeeds,
        input: || (u32::MAX - 1, 5),
        expect: Expect::Value(u32::MAX),
    },
];

fn subject((first, input): Input) -> Result<Value, Failure> {
    Ok(frontier_bucket(Bucket(first), InputNumber(input)).0)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
