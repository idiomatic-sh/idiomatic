//! A shared source batch lives until its last consumer is delivered, on every exit.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{BuildPolicy, Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::native_walk::Retained;

fn program(residency: Residency) -> Program {
    Program::new(
        Authored::continuations(),
        Population::Three,
        residency,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new(BuildPolicy::Normal)),
    )
    .unwrap()
}
fn deliver(
    file: &mut FileRun<'_>,
    program: &Program,
    rule: RuleIndex,
    symbols: &mut Symbols,
) -> RootOutcome {
    file.rule(rule, demand(), &|relation| program.read(relation), symbols)
        .unwrap()
        .emit
}
fn forward(
    file: &mut FileRun<'_>,
    program: &Program,
    symbols: &mut Symbols,
    each: &mut dyn FnMut(&[Cell]) -> Result<(), OperatingError>,
) -> Result<crate::application::file_dag::program::protocol::types::RowEmission, OperatingError> {
    file.emit_rows(
        RuleIndex(2),
        &|relation| program.read(relation),
        symbols,
        each,
    )
}

#[test]
fn success_of_the_last_consumer_retires_the_shared_batch_and_keeps_its_accounting() {
    let program = program(Residency::Stable);
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    assert_eq!(
        file.walk.retained(),
        Retained(0),
        "nothing is walked before a delivery"
    );
    assert!(matches!(
        deliver(&mut file, &program, RuleIndex(1), &mut symbols),
        RootOutcome::Complete(_)
    ));
    assert_eq!(
        file.walk.retained(),
        Retained(1),
        "three consumers of the class still owe"
    );
    deliver(&mut file, &program, RuleIndex(3), &mut symbols);
    deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert_eq!(
        file.walk.retained(),
        Retained(1),
        "the forwarded consumer still owes"
    );
    forward(&mut file, &program, &mut symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(
        file.walk.retained(),
        Retained(0),
        "the last consumer released the batch"
    );
    let work = file.walk_counts();
    assert_eq!(
        work.passes.0, 1,
        "retirement keeps the one pass in the totals"
    );
    assert_eq!(work.source_rows.0, 3);
    file.finish().unwrap();
}

#[test]
fn error_of_the_last_consumer_retires_the_shared_batch() {
    let program = program(Residency::Stable);
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    for rule in [RuleIndex(1), RuleIndex(3)] {
        assert!(matches!(
            deliver(&mut file, &program, rule, &mut symbols),
            RootOutcome::Complete(_)
        ));
    }
    forward(&mut file, &program, &mut symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(
        file.walk.retained(),
        Retained(1),
        "the failing rule still owes its delivery"
    );
    let failed = deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert!(matches!(
        failed,
        RootOutcome::Failed(OperatingError::WrongOperation)
    ));
    assert_eq!(
        file.walk.retained(),
        Retained(0),
        "a failed last consumer still releases"
    );
    file.finish().unwrap();
}

#[test]
fn cancellation_of_the_last_consumer_retires_the_shared_batch() {
    let program = program(Residency::Stable);
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    for rule in [RuleIndex(0), RuleIndex(1), RuleIndex(3)] {
        deliver(&mut file, &program, rule, &mut symbols);
    }
    assert_eq!(file.walk.retained(), Retained(1));
    let refused = forward(&mut file, &program, &mut symbols, &mut |_| {
        Err(OperatingError::NotRead)
    });
    assert!(matches!(refused, Err(OperatingError::NotRead)));
    assert_eq!(
        file.walk.retained(),
        Retained(0),
        "a cancelled last consumer still releases"
    );
    file.finish().unwrap();
}

#[test]
fn an_aliased_consumer_cancelled_early_does_not_retire_a_batch_other_rules_still_owe() {
    let program = program(Residency::Stable);
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    let RootOutcome::Complete(first) = deliver(&mut file, &program, RuleIndex(1), &mut symbols)
    else {
        panic!("the alias source rule must complete")
    };
    // Rule 2 is the same authored plan as rule 1, delivered through a callback.
    let refused = forward(&mut file, &program, &mut symbols, &mut |_| {
        Err(OperatingError::NotRead)
    });
    assert!(matches!(refused, Err(OperatingError::NotRead)));
    assert_eq!(
        file.walk.retained(),
        Retained(1),
        "rules 0 and 3 still owe the batch the cancelled alias shared"
    );
    let failed = deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert!(matches!(
        failed,
        RootOutcome::Failed(OperatingError::WrongOperation)
    ));
    assert_eq!(file.walk.retained(), Retained(1), "rule 3 still owes");
    assert!(matches!(
        deliver(&mut file, &program, RuleIndex(3), &mut symbols),
        RootOutcome::Complete(_)
    ));
    assert_eq!(file.walk.retained(), Retained(0));
    let RootOutcome::Complete(repeated) = deliver(&mut file, &program, RuleIndex(1), &mut symbols)
    else {
        panic!("the committed answer survives retirement of its batch")
    };
    assert_eq!(repeated, first);
    file.finish().unwrap();
}

#[test]
fn a_held_lease_stays_readable_across_reentrant_demands_and_retirement() {
    let program = program(Residency::Unstable);
    let class = program.head_class(RuleIndex(1)).unwrap();
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    deliver(&mut file, &program, RuleIndex(1), &mut symbols);
    let held = file.walk.lease_any(class).expect("the class is admitted");
    let positions = held
        .result()
        .positions
        .0
        .iter()
        .copied()
        .collect::<Vec<_>>();
    assert!(!positions.is_empty());
    // Other populations advance and the batches retire while the lease is held.
    deliver(&mut file, &program, RuleIndex(3), &mut symbols);
    deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    forward(&mut file, &program, &mut symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(file.walk.retained(), Retained(0));
    assert!(
        file.walk.lease_any(class).is_none(),
        "a demand after retirement finds no batch and refuses"
    );
    assert_eq!(
        held.result()
            .positions
            .0
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        positions,
        "the held lease owns its answer independent of the retired batch"
    );
    drop(held);
    file.finish().unwrap();
}

#[test]
fn files_retire_their_own_batches_independently() {
    let program = program(Residency::Stable);
    let (mut first_symbols, mut second_symbols) = (Symbols::default(), Symbols::default());
    let mut first = program.open(&mut first_symbols).unwrap();
    let mut second = program.open(&mut second_symbols).unwrap();
    deliver(&mut first, &program, RuleIndex(1), &mut first_symbols);
    assert_eq!(first.walk.retained(), Retained(1));
    assert_eq!(
        second.walk.retained(),
        Retained(0),
        "another generation never sees the batch"
    );
    deliver(&mut second, &program, RuleIndex(1), &mut second_symbols);
    for rule in [RuleIndex(3), RuleIndex(0)] {
        deliver(&mut first, &program, rule, &mut first_symbols);
    }
    forward(&mut first, &program, &mut first_symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(first.walk.retained(), Retained(0));
    assert_eq!(
        second.walk.retained(),
        Retained(1),
        "the second file still owes its consumers"
    );
    for rule in [RuleIndex(3), RuleIndex(0)] {
        deliver(&mut second, &program, rule, &mut second_symbols);
    }
    forward(&mut second, &program, &mut second_symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(second.walk.retained(), Retained(0));
    assert_eq!(first.walk_counts(), second.walk_counts());
    first.finish().unwrap();
    second.finish().unwrap();
}
