//! Refused calls are not deliveries; capture is owed; release is keyed by relation.
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::fixtures::{Residency, demand, observed_demand};
use super::fixtures::{
    capture, captured_program, deliver, forward, program, relations_program, rows,
};
use crate::application::file_dag::program::native_walk::types::RetainedBatches;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_language::domain::symbols::types::Symbols;

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
    assert_eq!(
        file.walk_counts().passes.0,
        1,
        "one pass served both deliveries"
    );
    file.finish().unwrap();
}

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
    let rows_wrong = rows(&mut file, &program, RuleIndex(3), &mut symbols, &mut |_| {
        Ok(())
    });
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

fn a_batch_stays_while_a_rule_of_another_class_on_its_relation_still_owes() {
    let program = relations_program();
    // The first delivery walks every relation the file reads: two batches. Class
    // `> 1` of the first relation has no consumer left after rule 0, but rule 1
    // reads the same relation through class `> 2`, so only the second relation's
    // owners and rule 1 decide what stays.
    for order in [[RuleIndex(0), RuleIndex(1)], [RuleIndex(1), RuleIndex(0)]] {
        let mut symbols = Symbols::default();
        let mut file = program.open(&mut symbols).unwrap();
        deliver(&mut file, &program, order[0], &mut symbols);
        assert_eq!(
            file.walk.retained(),
            RetainedBatches(2),
            "the other class of the first relation and the second relation are still owed"
        );
        deliver(&mut file, &program, order[1], &mut symbols);
        assert_eq!(
            file.walk.retained(),
            RetainedBatches(1),
            "the last consumer of the first relation released it; the second is still owed"
        );
        deliver(&mut file, &program, RuleIndex(2), &mut symbols);
        assert_eq!(
            file.walk.retained(),
            RetainedBatches(1),
            "rule 3 shares the class of rule 2 and still owes"
        );
        deliver(&mut file, &program, RuleIndex(3), &mut symbols);
        assert_eq!(file.walk.retained(), RetainedBatches(0));
        file.finish().unwrap();
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ARuleOwingRowsAndCaptureKeepsTheBatchUntilItsCaptureIsDelivered,
    ARefusedEntryPointCallDoesNotConsumeTheDeliveryItWouldHaveBeen,
    RetryingACancelledForwardedDeliveryAfterRetirementFindsNoBatchAndWalksNothing,
    ARelationIsReleasedWhileAnotherRelationIsStillOwed,
    ABatchStaysWhileARuleOfAnotherClassOnItsRelationStillOwes,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_rule_owing_rows_and_capture_keeps_the_batch_until_its_capture_is_delivered",
        ),
        word: Word::Succeeds,
        input: || Scenario::ARuleOwingRowsAndCaptureKeepsTheBatchUntilItsCaptureIsDelivered,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a_refused_entry_point_call_does_not_consume_the_delivery_it_would_have_been",
        ),
        word: Word::Succeeds,
        input: || Scenario::ARefusedEntryPointCallDoesNotConsumeTheDeliveryItWouldHaveBeen,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "retrying_a_cancelled_forwarded_delivery_after_retirement_finds_no_batch_and_walks_nothing",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::RetryingACancelledForwardedDeliveryAfterRetirementFindsNoBatchAndWalksNothing
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_relation_is_released_while_another_relation_is_still_owed"),
        word: Word::Succeeds,
        input: || Scenario::ARelationIsReleasedWhileAnotherRelationIsStillOwed,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_batch_stays_while_a_rule_of_another_class_on_its_relation_still_owes"),
        word: Word::Succeeds,
        input: || Scenario::ABatchStaysWhileARuleOfAnotherClassOnItsRelationStillOwes,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ARuleOwingRowsAndCaptureKeepsTheBatchUntilItsCaptureIsDelivered => {
            a_rule_owing_rows_and_capture_keeps_the_batch_until_its_capture_is_delivered();
            Ok(())
        }
        Scenario::ARefusedEntryPointCallDoesNotConsumeTheDeliveryItWouldHaveBeen => {
            a_refused_entry_point_call_does_not_consume_the_delivery_it_would_have_been();
            Ok(())
        }
        Scenario::RetryingACancelledForwardedDeliveryAfterRetirementFindsNoBatchAndWalksNothing => {
            retrying_a_cancelled_forwarded_delivery_after_retirement_finds_no_batch_and_walks_nothing();
            Ok(())
        }
        Scenario::ARelationIsReleasedWhileAnotherRelationIsStillOwed => {
            a_relation_is_released_while_another_relation_is_still_owed();
            Ok(())
        }
        Scenario::ABatchStaysWhileARuleOfAnotherClassOnItsRelationStillOwes => {
            a_batch_stays_while_a_rule_of_another_class_on_its_relation_still_owes();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
