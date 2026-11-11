//! Warm replay closure is observed through the actual prepared facade, not a copied graph algorithm.

use super::fixtures::{fixture, selection};
use crate::application::file_dag::program::prepared_execution::types::ReplayClosure;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::stored_facts::types::Bucket;
#[test]
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
#[test]
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
