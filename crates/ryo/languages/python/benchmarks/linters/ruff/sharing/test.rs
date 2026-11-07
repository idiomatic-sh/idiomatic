//! Actual configured classes and explicit barriers, through the production SDK.
use super::fixtures::{Observation, expected_routes, observe};
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
    // The trace dispatcher installs once per process, so both controls reuse this observation.
    let mut silent = observed.clone();
    let mut single = observed.clone();
    let mut relabelled = observed.clone();
    let mut unshared = observed.clone();
    let mut unwalked = observed.clone();
    let mut omitted = observed;
    assert!(
        omitted.dispatches.len() > 1,
        "negative control retains other actual dispatches"
    );
    let generation = omitted.dispatches[0].generation.clone();
    let before = omitted.dispatches.len();
    omitted.dispatches.remove(0);
    assert!(
        std::panic::catch_unwind(|| verify(omitted)).is_err(),
        "removing an actual admitted cohort must fail coverage"
    );
    // Dropping every dispatch of one file generation must fail too: no generation is
    // covered by another's evidence.
    silent
        .dispatches
        .retain(|dispatch| dispatch.generation != generation);
    assert!(silent.dispatches.len() < before);
    assert!(
        std::panic::catch_unwind(|| verify(silent)).is_err(),
        "a file generation with no dispatch must fail coverage"
    );
    // Cross-rule sharing comes from the compiled program's class membership: with every
    // class reduced to one rule, verification must fail.
    for rules in single.class_rules.values_mut() {
        *rules = rules.iter().take(1).copied().collect();
    }
    assert!(
        std::panic::catch_unwind(|| verify(single)).is_err(),
        "a program in which no class is shared by two rules must fail coverage"
    );
    // Relabelling one class's route must fail: a bound shared head claimed as a derived barrier.
    let shared = relabelled
        .routes
        .iter_mut()
        .find(|route| route.route == "Shared")
        .expect("a shared route event");
    shared.route = "derived".into();
    assert!(
        std::panic::catch_unwind(|| verify(relabelled)).is_err(),
        "a class whose expected route is changed must fail coverage"
    );
    // A project-stage head relabelled as the old refusal must fail: its route is shared.
    let project = unshared
        .routes
        .iter_mut()
        .find(|route| route.phase == "CrossFile" && route.route == "Shared")
        .expect("a shared project-stage route event");
    project.route = "Representation".into();
    assert!(
        std::panic::catch_unwind(|| verify(unshared)).is_err(),
        "a project-stage head claimed as refused must fail coverage"
    );
    // A project stage that never walks its source is not shared.
    unwalked
        .dispatches
        .retain(|dispatch| dispatch.phase != "CrossFile");
    assert!(
        std::panic::catch_unwind(|| verify(unwalked)).is_err(),
        "a project stage with no walk dispatch must fail coverage"
    );
    Ok(())
}
fn verify(observed: Observation) -> Result<Value, Failure> {
    // The engine-independent expectation: each instruction's route is one its operation kind
    // allows, every instruction of the census is traced, and a class holds only selections.
    assert_eq!(
        observed.routes.len() as u64,
        observed.census.instructions,
        "one route event for each census instruction"
    );
    let mut counts = std::collections::BTreeMap::<&str, u64>::new();
    for event in &observed.routes {
        let allowed = expected_routes(&event.kind, event.head);
        assert!(
            allowed.contains(&event.route.as_str()),
            "instruction {} ({}) bound to {}, expected one of {allowed:?}",
            event.logical,
            event.kind,
            event.route
        );
        assert_eq!(
            event.class.is_some(),
            event.route == "Shared",
            "a class is held by shared routes only"
        );
        *counts.entry(event.route.as_str()).or_default() += 1;
    }
    assert_eq!(
        counts.get("Shared").copied().unwrap_or(0),
        observed.census.shared,
        "the traced shared routes are the census shared heads"
    );
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
    // Project-stage heads that select a relation join the walk like file heads: they are
    // shared, in classes of their own partition, and the barriers beside them are the other
    // operation kinds.
    assert!(
        census.instructions > census.shared,
        "barriers remain beside the shared heads"
    );
    let project_shared = observed
        .routes
        .iter()
        .filter(|route| route.phase == "CrossFile" && route.head && route.route == "Shared")
        .count();
    assert!(
        project_shared > 0,
        "a project-stage selection of a relation is a shared head, never refused by construction"
    );
    assert!(
        observed
            .ready
            .iter()
            .all(|head| observed.heads.contains_key(&head.logical)),
        "no admitted head is outside the compiled shared heads"
    );
    // Two files: each file generation owns fresh populations; none is reused or shared.
    let generations = observed
        .ready
        .iter()
        .map(|head| head.generation.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        generations.len(),
        2,
        "one file generation for each of the two files"
    );
    // A population id names a canonical producer population of the program, so it may
    // repeat across files; the walk keys its batches by (generation, population). Each
    // generation therefore admits the same heads on its own, and no key collapses across
    // generations.
    let heads_of = |generation: &str| {
        observed
            .ready
            .iter()
            .filter(|head| head.generation == generation)
            .map(|head| head.logical)
            .collect::<BTreeSet<_>>()
    };
    let per_generation = generations
        .iter()
        .map(|generation| heads_of(generation))
        .collect::<Vec<_>>();
    assert!(!per_generation[0].is_empty());
    assert_eq!(
        per_generation[0], per_generation[1],
        "each file generation admits the same heads on its own"
    );
    let keys = observed
        .ready
        .iter()
        .map(|head| {
            (
                head.generation.as_str(),
                head.population.as_str(),
                head.logical,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys.len(),
        observed
            .ready
            .iter()
            .map(|head| (head.generation.as_str(), head.logical))
            .collect::<BTreeSet<_>>()
            .len(),
        "one population per head within a generation; no key is shared across generations"
    );
    for dispatch in observed
        .dispatches
        .iter()
        .filter(|dispatch| dispatch.phase == "PerFile")
    {
        assert!(
            generations.contains(dispatch.generation.as_str()),
            "every file dispatch belongs to an admitted file generation"
        );
    }
    // The project stage has one walk generation of its own, never a file's.
    let project_generations = observed
        .dispatches
        .iter()
        .filter(|dispatch| dispatch.phase == "CrossFile")
        .map(|dispatch| dispatch.generation.as_str())
        .collect::<BTreeSet<_>>();
    assert!(
        project_generations.is_disjoint(&generations),
        "a project dispatch is never attributed to a file generation"
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
    let (project, file): (Vec<_>, Vec<_>) = observed
        .dispatches
        .into_iter()
        .partition(|dispatch| dispatch.phase == "CrossFile");
    let mut dispatched = BTreeSet::new();
    let (mut passes, mut rows) = (0, 0);
    let mut shared_cohort = false;
    for event in file {
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
    // Project stage: each (population, class) is read by one pass for all the rules of its
    // cohort, and no population is read twice.
    assert!(
        !project.is_empty(),
        "the project stage walks its source population through the shared walk"
    );
    let mut project_read = BTreeSet::new();
    for event in &project {
        assert_eq!(event.classes.len() as u64, event.result_slots);
        assert_eq!(event.passes, 1, "one source pass for this project population");
        for class in &event.classes {
            assert_eq!(
                observed.classes.get(class),
                Some(&event.relation),
                "actual bound project class"
            );
            assert!(
                project_read.insert((event.population.clone(), *class)),
                "a project population is read once for all the rules that share its class"
            );
        }
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
    assert_eq!(
        observed.class_rules.keys().collect::<Vec<_>>(),
        observed.classes.keys().collect::<Vec<_>>(),
        "every compiled class names the rules that share it"
    );
    assert!(
        observed.class_rules.values().any(|rules| rules.len() > 1),
        "at least one compiled class is shared by two or more rules"
    );
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
