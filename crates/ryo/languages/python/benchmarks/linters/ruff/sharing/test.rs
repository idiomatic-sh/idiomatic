//! Actual configured classes and explicit barriers, through the production SDK.
use super::fixtures::{Observation, observe};
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
    verify(observed.clone())?;
    let mut omitted = observed;
    assert!(
        omitted.dispatches.len() > 1,
        "negative control retains other actual dispatches"
    );
    omitted.dispatches.remove(0);
    assert!(
        std::panic::catch_unwind(|| verify(omitted)).is_err(),
        "removing an actual admitted cohort must fail coverage"
    );
    Ok(())
}
fn verify(observed: Observation) -> Result<Value, Failure> {
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
        census.instructions, census.physical_instructions,
        "route census covers the independent compiled instruction total, including observation unions"
    );
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
    assert_eq!(
        observed.heads.len() as u64,
        census.shared,
        "all prepared shared instructions retain their separate physical IDs"
    );
    let mut expected = BTreeSet::new();
    for head in observed.ready {
        let class = *observed
            .heads
            .get(&head.logical)
            .expect("runtime-admitted eligible physical head has no compiled shared mapping");
        assert_eq!(observed.classes.get(&class), Some(&head.relation));
        // Equivalent physical heads share only after each ID has been accounted for.
        expected.insert((head.generation, head.population, class));
    }
    assert!(
        !expected.is_empty(),
        "authoritative runtime admission precedes any callback"
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
                dispatched.insert((event.generation.clone(), event.population.clone(), class)),
                "class reevaluated in same admitted population"
            );
        }
        passes += event.passes;
        rows += event.source_rows;
        assert!(event.decoded_rows <= event.source_rows);
    }
    assert!(
        dispatched.is_disjoint(&observed.refused),
        "an admitted class is either served or explicitly refused by representation admission"
    );
    let covered = dispatched
        .union(&observed.refused)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        covered, expected,
        "every runtime-admitted physical head is served or explicitly representation-refused, independently of submission"
    );
    assert!(dispatched.len() > 1 && passes > 0 && rows > 0);
    assert!(
        shared_cohort,
        "actual configured rules must share a source traversal across distinct classes"
    );
    // The expected inventory comes from runtime admission before the overridable
    // ready callback. The private runtime omission control skips an actual
    // submission while retaining other dispatches; dispatch deletion above is
    // an additional transport/accounting falsifier.
    // This corpus proves reached eligible heads under all413 selection.
    // Independently proving every statically eligible head is classified correctly
    // remains a separate contract; dormant classes need not execute.
    // Replay/native/column/ordered proofs remain execution's actual route tests.
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
