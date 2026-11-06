//! A shared source batch lives until its last consumer is delivered, on every exit.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{BuildPolicy, Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand, observed_demand};
use crate::application::file_dag::program::native_walk::RetainedBatches;
use crate::application::file_dag::program::protocol::types::{CaptureContext, RowEmission};

fn captured_program() -> Program {
    Program::new(
        Authored::captured(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new(BuildPolicy::Normal)),
    )
    .unwrap()
}
fn relations_program() -> Program {
    Program::new(
        Authored::relations(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new(BuildPolicy::Normal)),
    )
    .unwrap()
}
fn capture(
    file: &mut FileRun<'_>,
    program: &Program,
    rule: RuleIndex,
    symbols: &mut Symbols,
) -> Result<usize, OperatingError> {
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = ryo_relations::domain::relations::types::Words::default();
    file.capture_frontiers(
        rule,
        ryo_values::domain::stored_facts::types::Bucket(0),
        &|relation| program.read(relation),
        symbols,
        CaptureContext {
            file: FileId(0),
            rule_digest: Digest([0; 16]),
            worker: ryo_relations::domain::relations::types::WorkerNumber(0),
            retained: crate::domain::finish::types::Retained {
                arena: crate::domain::finish::types::Arena(&mut arena),
                places: &mut places,
            },
            words: &mut words,
        },
    )
    .map(|captures| captures.len())
}
fn rows(
    file: &mut FileRun<'_>,
    program: &Program,
    rule: RuleIndex,
    symbols: &mut Symbols,
    each: &mut dyn FnMut(&[Cell]) -> Result<(), OperatingError>,
) -> Result<RowEmission, OperatingError> {
    file.emit_rows(rule, &|relation| program.read(relation), symbols, each)
}

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
        RetainedBatches(0),
        "nothing is walked before a delivery"
    );
    assert!(matches!(
        deliver(&mut file, &program, RuleIndex(1), &mut symbols),
        RootOutcome::Complete(_)
    ));
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "three consumers of the class still owe"
    );
    deliver(&mut file, &program, RuleIndex(3), &mut symbols);
    deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "the forwarded consumer still owes"
    );
    forward(&mut file, &program, &mut symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
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
        RetainedBatches(1),
        "the failing rule still owes its delivery"
    );
    let failed = deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert!(matches!(
        failed,
        RootOutcome::Failed(OperatingError::WrongOperation)
    ));
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
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
    assert_eq!(file.walk.retained(), RetainedBatches(1));
    let refused = forward(&mut file, &program, &mut symbols, &mut |_| {
        Err(OperatingError::NotRead)
    });
    assert!(matches!(refused, Err(OperatingError::NotRead)));
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
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
        RetainedBatches(1),
        "rules 0 and 3 still owe the batch the cancelled alias shared"
    );
    let failed = deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert!(matches!(
        failed,
        RootOutcome::Failed(OperatingError::WrongOperation)
    ));
    assert_eq!(file.walk.retained(), RetainedBatches(1), "rule 3 still owes");
    assert!(matches!(
        deliver(&mut file, &program, RuleIndex(3), &mut symbols),
        RootOutcome::Complete(_)
    ));
    assert_eq!(file.walk.retained(), RetainedBatches(0));
    let RootOutcome::Complete(repeated) = deliver(&mut file, &program, RuleIndex(1), &mut symbols)
    else {
        panic!("the committed answer survives retirement of its batch")
    };
    assert_eq!(repeated, first);
    file.finish().unwrap();
}

/// A `KernelPermit` cannot be built outside the runtime, so this control leases through the
/// test-only `lease_any` on the batch map. It proves a held lease stays readable across
/// retirement and that a later demand finds no batch; it does not exercise the permit path
/// of a real nested kernel demand.
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
    assert_eq!(file.walk.retained(), RetainedBatches(0));
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
    assert_eq!(first.walk.retained(), RetainedBatches(1));
    assert_eq!(
        second.walk.retained(),
        RetainedBatches(0),
        "another generation never sees the batch"
    );
    deliver(&mut second, &program, RuleIndex(1), &mut second_symbols);
    for rule in [RuleIndex(3), RuleIndex(0)] {
        deliver(&mut first, &program, rule, &mut first_symbols);
    }
    forward(&mut first, &program, &mut first_symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(first.walk.retained(), RetainedBatches(0));
    assert_eq!(
        second.walk.retained(),
        RetainedBatches(1),
        "the second file still owes its consumers"
    );
    for rule in [RuleIndex(3), RuleIndex(0)] {
        deliver(&mut second, &program, rule, &mut second_symbols);
    }
    forward(&mut second, &program, &mut second_symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(second.walk.retained(), RetainedBatches(0));
    assert_eq!(first.walk_counts(), second.walk_counts());
    first.finish().unwrap();
    second.finish().unwrap();
}

#[test]
fn a_rule_owing_rows_and_capture_keeps_the_batch_until_its_capture_is_delivered() {
    let program = captured_program();
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    for rule in [RuleIndex(0), RuleIndex(1)] {
        let delivered = file
            .rule(
                rule,
                observed_demand(),
                &|relation| program.read(relation),
                &mut symbols,
            )
            .unwrap();
        assert!(matches!(delivered.emit, RootOutcome::Complete(_)));
    }
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "both rules still owe their capture after their rows"
    );
    capture(&mut file, &program, RuleIndex(0), &mut symbols).unwrap();
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "the second rule still owes its capture"
    );
    capture(&mut file, &program, RuleIndex(1), &mut symbols).unwrap();
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
        "the last capture released the batch"
    );
    assert_eq!(file.walk_counts().passes.0, 1, "one pass served both deliveries");
    file.finish().unwrap();
}

#[test]
fn a_refused_entry_point_call_does_not_consume_the_delivery_it_would_have_been() {
    let program = program(Residency::Stable);
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    // Rule 2 is forwarded: rule() is the wrong entry point for it.
    let wrong = file.rule(
        RuleIndex(2),
        demand(),
        &|relation| program.read(relation),
        &mut symbols,
    );
    assert!(matches!(wrong, Err(OperatingError::NotRun)));
    // Rule 3 is collected without capture: rows and capture are the wrong entry points.
    let rows_wrong = rows(&mut file, &program, RuleIndex(3), &mut symbols, &mut |_| Ok(()));
    assert!(matches!(rows_wrong, Err(OperatingError::NotRun)));
    let capture_wrong = capture(&mut file, &program, RuleIndex(3), &mut symbols);
    assert!(matches!(capture_wrong, Err(OperatingError::NotRun)));
    // A delivery whose demand differs from the declared intent is refused too.
    let demanded = file.rule(
        RuleIndex(3),
        observed_demand(),
        &|relation| program.read(relation),
        &mut symbols,
    );
    assert!(matches!(demanded, Err(OperatingError::NotRun)));
    for rule in [RuleIndex(1), RuleIndex(0)] {
        deliver(&mut file, &program, rule, &mut symbols);
    }
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "rules 2 and 3 still owe: their refused calls were not deliveries"
    );
    forward(&mut file, &program, &mut symbols, &mut |_| Ok(())).unwrap();
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "rule 3 still owes its one delivery"
    );
    deliver(&mut file, &program, RuleIndex(3), &mut symbols);
    assert_eq!(file.walk.retained(), RetainedBatches(0));
    file.finish().unwrap();
}

#[test]
fn retrying_a_cancelled_forwarded_delivery_after_retirement_finds_no_batch_and_walks_nothing() {
    let program = program(Residency::Stable);
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    for rule in [RuleIndex(1), RuleIndex(3), RuleIndex(0)] {
        deliver(&mut file, &program, rule, &mut symbols);
    }
    let cancelled = forward(&mut file, &program, &mut symbols, &mut |_| {
        Err(OperatingError::NotRead)
    });
    assert!(matches!(cancelled, Err(OperatingError::NotRead)));
    assert_eq!(file.walk.retained(), RetainedBatches(0));
    let passes = file.walk_counts().passes.0;
    assert_eq!(passes, 1);
    let retried = forward(&mut file, &program, &mut symbols, &mut |_| Ok(()));
    assert!(
        matches!(retried, Ok(RowEmission::Complete)),
        "the committed publication replays after the batch retired"
    );
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
        "the retry rebuilt no batch"
    );
    assert_eq!(
        file.walk_counts().passes.0,
        passes,
        "the retry walked no source again"
    );
    file.finish().unwrap();
}

#[test]
fn a_relation_is_released_while_another_relation_is_still_owed() {
    let program = relations_program();
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    assert_ne!(
        program.head_class(RuleIndex(0)).unwrap(),
        program.head_class(RuleIndex(1)).unwrap(),
        "two classes on the first relation"
    );
    assert_eq!(
        program.head_class(RuleIndex(2)).unwrap(),
        program.head_class(RuleIndex(3)).unwrap(),
        "one class on the second relation consumed by two rules"
    );
    deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    deliver(&mut file, &program, RuleIndex(2), &mut symbols);
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(2),
        "rule 1 still owes the first relation and rule 3 the second"
    );
    deliver(&mut file, &program, RuleIndex(1), &mut symbols);
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "nobody owes the first relation any more; the second is still owed"
    );
    deliver(&mut file, &program, RuleIndex(3), &mut symbols);
    assert_eq!(file.walk.retained(), RetainedBatches(0));
    file.finish().unwrap();
}

#[test]
fn a_batch_stays_while_a_rule_of_another_class_on_its_relation_still_owes() {
    let program = relations_program();
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    // Class of rule 0 has no consumer left after its delivery, but rule 1 reads
    // the same relation through another class.
    deliver(&mut file, &program, RuleIndex(0), &mut symbols);
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(1),
        "the relation is still owed by the other class"
    );
    deliver(&mut file, &program, RuleIndex(1), &mut symbols);
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
        "the last consumer of the relation released it"
    );
    // The same relation with the classes in the other order.
    let mut symbols = Symbols::default();
    let mut reversed = program.open(&mut symbols).unwrap();
    deliver(&mut reversed, &program, RuleIndex(1), &mut symbols);
    assert_eq!(reversed.walk.retained(), RetainedBatches(1));
    deliver(&mut reversed, &program, RuleIndex(0), &mut symbols);
    assert_eq!(reversed.walk.retained(), RetainedBatches(0));
    // A shared class stays until both of its rules are delivered.
    let mut symbols = Symbols::default();
    let mut shared = program.open(&mut symbols).unwrap();
    deliver(&mut shared, &program, RuleIndex(2), &mut symbols);
    assert_eq!(shared.walk.retained(), RetainedBatches(1));
    deliver(&mut shared, &program, RuleIndex(3), &mut symbols);
    assert_eq!(shared.walk.retained(), RetainedBatches(0));
    file.finish().unwrap();
    reversed.finish().unwrap();
    shared.finish().unwrap();
}
