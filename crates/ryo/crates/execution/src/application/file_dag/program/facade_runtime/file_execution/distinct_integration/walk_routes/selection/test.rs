//! A lawful presemantic decline differs from a reached acquisition failure.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{BuildPolicy, Calls, Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, cells, demand};

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Acquired,
    Fault,
    Declined,
}
fn exercise(scenario: Scenario) -> Result<(), OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let derived = Arc::new(Derived::new(match scenario {
        Scenario::Acquired => BuildPolicy::Normal,
        Scenario::Fault | Scenario::Declined => BuildPolicy::RefuseDistinct,
    }));
    let residency = match scenario {
        Scenario::Declined => Residency::Unstable,
        _ => Residency::Stable,
    };
    let program = Program::new(
        Authored::distinct(),
        Population::Duplicate,
        residency,
        meanings.clone(),
        derived.clone(),
    )?;
    let root = *program.prepared.program().rules()[0].emit.as_ref().unwrap();
    let region = &program.prepared.program().regions()[root.0];
    assert!(
        region.selection.is_some(),
        "actual compiler must install the distinct selected alternative"
    );
    assert!(
        program
            .prepared
            .walk_class(&region.instructions[0])
            .is_none(),
        "distinct row semantics are not a pure source-walk head"
    );
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let first = file.rule(RuleIndex(0), demand(), &read, &mut symbols)?;
    match (scenario, first.emit) {
        (Scenario::Fault, RootOutcome::Failed(OperatingError::DuplicateOrder)) => {}
        (Scenario::Acquired | Scenario::Declined, RootOutcome::Complete(table)) => {
            assert_eq!(table.cells, cells(&[2, 3]));
            assert_eq!(table.count, RowCount(2));
        }
        _ => panic!(
            "{scenario:?}: selected acquisition fault must never become ordinary evaluator success"
        ),
    }
    let semantic = meanings.snapshot();
    match scenario {
        Scenario::Acquired | Scenario::Fault => {
            assert_eq!(
                derived.distinct_calls(),
                Calls(1),
                "the actual required distinct acquisition was reached"
            );
            assert_eq!(
                semantic.projects,
                Calls(0),
                "a reached owner result or fault cannot fall back to registered Project"
            );
        }
        Scenario::Declined => {
            assert_eq!(
                derived.distinct_calls(),
                Calls(0),
                "unstable identity declines before any acquisition"
            );
            assert!(
                semantic.projects.0 > 0,
                "the lawful decline reaches the same registered semantic contract"
            );
        }
    }
    let work = file.walk_counts();
    assert_eq!(work.passes.0, 0);
    let reads = program.reads();
    let builds = derived.snapshot();
    for _ in 0..2 {
        let replay = file.rule(RuleIndex(0), demand(), &read, &mut symbols)?;
        match (scenario, replay.emit) {
            (Scenario::Fault, RootOutcome::Failed(OperatingError::DuplicateOrder)) => {}
            (Scenario::Acquired | Scenario::Declined, RootOutcome::Complete(table)) => {
                assert_eq!(table.cells, cells(&[2, 3]))
            }
            _ => panic!("{scenario:?}: terminal replay changed its exact acquisition result"),
        }
    }
    assert_eq!(meanings.snapshot(), semantic);
    assert_eq!(file.walk_counts(), work);
    if !matches!(scenario, Scenario::Acquired) {
        assert_eq!(program.reads(), reads);
    }
    // A successful selected publication may reborrow stable provider facts;
    // its semantic/acquisition counters above must still remain unchanged.
    assert_eq!(derived.snapshot(), builds);
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(owner.derived.working_bytes(), 0);
    Ok(())
}
#[test]
fn selected_distinct_success_and_owner_fault_are_terminal_without_registered_fallback() {
    for scenario in [Scenario::Acquired, Scenario::Fault] {
        exercise(scenario).unwrap_or_else(|error| panic!("{scenario:?}: {error:?}"));
    }
}
#[test]
fn unstable_selected_source_declines_before_semantics_without_hiding_a_reached_fault() {
    exercise(Scenario::Declined).expect("actual presemantic stability decline");
}

fn forward_refusal(scenario: Scenario) -> Result<(), OperatingError> {
    let derived = Arc::new(Derived::new(match scenario {
        Scenario::Acquired => BuildPolicy::Normal,
        Scenario::Fault | Scenario::Declined => BuildPolicy::RefuseDistinct,
    }));
    let program = Program::new(
        Authored::forwarded_distinct(),
        Population::Duplicate,
        Residency::Stable,
        Arc::new(Meanings::default()),
        derived.clone(),
    )?;
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let mut refused = vec![];
    let first = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
        refused.push(row.to_vec());
        Err(OperatingError::NotRead)
    });
    let builds = derived.snapshot();
    match scenario {
        Scenario::Acquired => {
            assert!(
                matches!(first, Err(OperatingError::NotRead)),
                "the sink refusal is the only failure of a completed selected publication"
            );
            assert_eq!(
                refused,
                vec![cells(&[2])],
                "delivery stops at the first refused row"
            );
            let mut replay = vec![];
            file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
                replay.push(row.to_vec());
                Ok(())
            })?;
            assert_eq!(
                replay,
                vec![cells(&[2]), cells(&[3])],
                "retry replays the whole committed publication"
            );
            assert_eq!(derived.snapshot(), builds, "retry builds nothing again");
        }
        Scenario::Fault | Scenario::Declined => {
            assert!(
                matches!(first, Err(OperatingError::DuplicateOrder)),
                "the reached acquisition fault is primary; the sink is never asked"
            );
            assert!(refused.is_empty());
            let again = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |_| Ok(()));
            assert!(matches!(again, Err(OperatingError::DuplicateOrder)));
            assert_eq!(derived.snapshot(), builds, "a failed root is never rebuilt");
        }
    }
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(owner.derived.working_bytes(), 0);
    assert_eq!(owner.derived.publication_ownership_bytes().0, 0);
    Ok(())
}
#[test]
fn selected_distinct_delivery_refusal_and_acquisition_fault_release_with_the_owner() {
    for scenario in [Scenario::Acquired, Scenario::Fault] {
        forward_refusal(scenario).unwrap_or_else(|error| panic!("{scenario:?}: {error:?}"));
    }
}
