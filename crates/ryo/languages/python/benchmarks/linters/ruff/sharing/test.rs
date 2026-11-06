//! Actual configured classes and explicit barriers, through the production SDK.
use super::fixtures::observe;
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use std::collections::BTreeSet;
type Input = ();
type Value = ();
type Failure = String;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("canonical_413_inventory_uses_bound_shared_classes_and_accounts_for_barriers"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    let observed = observe().map_err(|error| error.to_string())?;
    let census = observed.census;
    let mut expected = observed.selected;
    expected.insert("invalid-syntax".into());
    assert_eq!(
        census.rules.len(),
        expected.len(),
        "no duplicate or extra prepared rules"
    );
    assert_eq!(census.rules.into_iter().collect::<BTreeSet<_>>(), expected);
    assert_eq!(
        census.shared + census.barriers.into_iter().sum::<u64>(),
        census.instructions,
        "every bound physical instruction has an explicit route"
    );
    assert!(
        census.shared > 1 && census.barriers.into_iter().sum::<u64>() > 0,
        "real shared heads and barriers"
    );
    assert_eq!(observed.classes.len() as u64, census.classes);
    assert!(observed.classes.len() > 1);
    assert!(
        !observed.dispatches.is_empty(),
        "actual eligible heads dispatch through their file owner"
    );
    let mut dispatched = BTreeSet::new();
    let (mut passes, mut rows) = (0, 0);
    let mut shared_cohort = false;
    for event in observed.dispatches {
        assert_eq!(event.classes.len() as u64, event.result_slots);
        if !event.classes.is_empty() {
            assert_eq!(
                event.passes, 1,
                "one actual source traversal for this admitted cohort"
            );
        }
        shared_cohort |= event.classes.len() > 1;
        for class in event.classes {
            assert_eq!(
                observed.classes.get(&class),
                Some(&event.relation),
                "actual bound source class"
            );
            assert!(
                dispatched.insert((event.population.clone(), class)),
                "class reevaluated in same admitted population"
            );
        }
        passes += event.passes;
        rows += event.source_rows;
        assert!(event.decoded_rows <= event.source_rows);
    }
    assert!(dispatched.len() > 1 && passes > 0 && rows > 0);
    assert!(
        shared_cohort,
        "actual configured rules must share a source traversal across distinct classes"
    );
    // This corpus proves reached classes under all413 selection. Other classes
    // remain candidates, not a claim that every operation runs in one scan.
    // Replay/native/column/ordered proofs remain execution's actual route tests.
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
