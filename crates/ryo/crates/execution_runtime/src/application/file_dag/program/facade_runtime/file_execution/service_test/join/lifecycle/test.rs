//! Reentrant delivery and retry use one genuine prepared program and file owner.
use super::fixtures::exercise_nested;
use crate::cases::{Case, CaseName, Expect, Word, run};

fn same_owner_nested_join_preserves_shared_index_bag_order_and_delivery_retry() {
    exercise_nested(false).expect("one compiled owner, nested Join and delivery retry");
}
fn nested_output_keeps_consumer_memo_scratch_prefix_and_resume() {
    exercise_nested(true).expect("actual nested output scope");
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    SameOwnerNestedJoinPreservesSharedIndexBagOrderAndDeliveryRetry,
    NestedOutputKeepsConsumerMemoScratchPrefixAndResume,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "same_owner_nested_join_preserves_shared_index_bag_order_and_delivery_retry",
        ),
        word: Word::Succeeds,
        input: || Scenario::SameOwnerNestedJoinPreservesSharedIndexBagOrderAndDeliveryRetry,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("nested_output_keeps_consumer_memo_scratch_prefix_and_resume"),
        word: Word::Succeeds,
        input: || Scenario::NestedOutputKeepsConsumerMemoScratchPrefixAndResume,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::SameOwnerNestedJoinPreservesSharedIndexBagOrderAndDeliveryRetry => {
            same_owner_nested_join_preserves_shared_index_bag_order_and_delivery_retry();
            Ok(())
        }
        Scenario::NestedOutputKeepsConsumerMemoScratchPrefixAndResume => {
            nested_output_keeps_consumer_memo_scratch_prefix_and_resume();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
