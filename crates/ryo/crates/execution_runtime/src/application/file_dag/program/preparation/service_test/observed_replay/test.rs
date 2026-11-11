//! Warm replay closure is observed through the actual prepared facade, not a copied graph algorithm.
use crate::cases::{Case, CaseName, Expect, Word, run};

use super::fixtures::{fixture, selection};
use crate::application::file_dag::program::prepared_execution::types::ReplayClosure;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::stored_facts::types::Bucket;
fn warm_consumer_selects_transitive_cross_producers_and_changed_producer_selects_consumers() {
    let prepared = fixture(false);
    let first = [Bucket(10), Bucket(20), Bucket(30), Bucket(40)];
    assert_eq!(
        prepared.replay_closure(selection(&[2]), &first).unwrap(),
        ReplayClosure {
            reuse: selection(&[0, 1, 2]),
            buckets: vec![]
        }
    );
    assert_eq!(
        prepared.replay_closure(selection(&[0]), &first).unwrap(),
        ReplayClosure {
            reuse: selection(&[0, 1, 2]),
            buckets: vec![]
        }
    );
    assert_eq!(
        prepared.replay_closure(selection(&[3]), &first).unwrap(),
        ReplayClosure {
            reuse: selection(&[3]),
            buckets: vec![]
        }
    );
    assert_eq!(
        prepared.replay_closure(selection(&[]), &first).unwrap(),
        ReplayClosure {
            reuse: selection(&[]),
            buckets: vec![]
        }
    );
    assert!(matches!(
        prepared.replay_closure(selection(&[4]), &first),
        Err(OperatingError::NotRun)
    ));
}
fn per_file_producer_is_not_reexecuted_in_the_consumer_cross_phase() {
    let prepared = fixture(true);
    assert_eq!(
        prepared
            .replay_closure(
                selection(&[2]),
                &[Bucket(10), Bucket(20), Bucket(30), Bucket(40)]
            )
            .unwrap(),
        ReplayClosure {
            reuse: selection(&[1, 2]),
            buckets: vec![Bucket(10)]
        },
        "actual captured file frontier is read instead of rerunning its producer"
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    WarmConsumerSelectsTransitiveCrossProducersAndChangedProducerSelectsConsumers,
    PerFileProducerIsNotReexecutedInTheConsumerCrossPhase,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "warm_consumer_selects_transitive_cross_producers_and_changed_producer_selects_consumers",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::WarmConsumerSelectsTransitiveCrossProducersAndChangedProducerSelectsConsumers
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("per_file_producer_is_not_reexecuted_in_the_consumer_cross_phase"),
        word: Word::Succeeds,
        input: || Scenario::PerFileProducerIsNotReexecutedInTheConsumerCrossPhase,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::WarmConsumerSelectsTransitiveCrossProducersAndChangedProducerSelectsConsumers => {
            warm_consumer_selects_transitive_cross_producers_and_changed_producer_selects_consumers(
            );
            Ok(())
        }
        Scenario::PerFileProducerIsNotReexecutedInTheConsumerCrossPhase => {
            per_file_producer_is_not_reexecuted_in_the_consumer_cross_phase();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
