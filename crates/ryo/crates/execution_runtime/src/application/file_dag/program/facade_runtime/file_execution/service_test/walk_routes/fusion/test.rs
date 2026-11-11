//! Named captures keep their identity; fused and held programs agree on rows, order and faults.
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::{Combine, Feed, Fusion};
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency};
use super::fixtures::decide;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::fusion::types::{
    Barrier, Fusibility, Ineligible,
};
use std::sync::Arc;

fn a_named_capture_root_is_not_fused_into() {
    let program = Program::new(
        Authored::named_continuation(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .expect("named producers with a projection of their captured head");
    let compiled = program.prepared.program();
    for rule in [RuleIndex(0), RuleIndex(1)] {
        let origins = &compiled.rules()[rule.0].original_steps;
        let projection = origins[1].as_ref().unwrap().logical;
        assert_eq!(
            compiled.fusibility(projection),
            Some(Fusibility::Barrier(Barrier::Capture)),
            "a named observation output keeps its owner-issued publication"
        );
        let region = compiled.logical_regions()[projection.0];
        assert_eq!(compiled.regions()[region.0].instructions().len(), 1);
    }
    // Its consumer reads a named observation: a source, never a continuation.
    let consumer = compiled.rules()[2].original_steps[0]
        .as_ref()
        .unwrap()
        .logical;
    assert_eq!(
        compiled.fusibility(consumer),
        Some(Fusibility::Excluded(Ineligible::Source))
    );
}

fn fused_and_held_combinations_agree_on_rows_order_and_error_precedence() {
    for combine in Combine::ALL {
        for left in Feed::ALL {
            for right in Feed::ALL {
                let fused = decide(combine, left, right, Fusion::Fused);
                let held = decide(combine, left, right, Fusion::Held);
                match (fused, held) {
                    (Ok(fused), Ok(held)) => assert_eq!(
                        fused, held,
                        "{combine:?} left {left:?} right {right:?}: fusion changed the answer or the faults reached"
                    ),
                    (fused, held) => panic!(
                        "{combine:?} left {left:?} right {right:?}: {:?} against {:?}",
                        fused.map(|_| ()),
                        held.map(|_| ())
                    ),
                }
            }
        }
    }
}

fn an_empty_left_does_not_hide_a_failing_right_differently_when_fused() {
    for combine in Combine::ALL {
        let fused = decide(combine, Feed::Empty, Feed::Failing, Fusion::Fused).unwrap();
        let held = decide(combine, Feed::Empty, Feed::Failing, Fusion::Held).unwrap();
        assert_eq!(fused.answer, held.answer, "{combine:?}");
        assert_eq!(fused.reached.failed, held.reached.failed, "{combine:?}");
    }
}

fn competing_faults_reach_the_same_side_first_when_fused() {
    for combine in Combine::ALL {
        let fused = decide(combine, Feed::Failing, Feed::Failing, Fusion::Fused).unwrap();
        let held = decide(combine, Feed::Failing, Feed::Failing, Fusion::Held).unwrap();
        assert_eq!(fused, held, "{combine:?}");
        assert!(
            fused.answer.starts_with("failed"),
            "{combine:?}: {}",
            fused.answer
        );
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ANamedCaptureRootIsNotFusedInto,
    FusedAndHeldCombinationsAgreeOnRowsOrderAndErrorPrecedence,
    AnEmptyLeftDoesNotHideAFailingRightDifferentlyWhenFused,
    CompetingFaultsReachTheSameSideFirstWhenFused,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_named_capture_root_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Scenario::ANamedCaptureRootIsNotFusedInto,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("fused_and_held_combinations_agree_on_rows_order_and_error_precedence"),
        word: Word::Succeeds,
        input: || Scenario::FusedAndHeldCombinationsAgreeOnRowsOrderAndErrorPrecedence,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_empty_left_does_not_hide_a_failing_right_differently_when_fused"),
        word: Word::Succeeds,
        input: || Scenario::AnEmptyLeftDoesNotHideAFailingRightDifferentlyWhenFused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("competing_faults_reach_the_same_side_first_when_fused"),
        word: Word::Succeeds,
        input: || Scenario::CompetingFaultsReachTheSameSideFirstWhenFused,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ANamedCaptureRootIsNotFusedInto => {
            a_named_capture_root_is_not_fused_into();
            Ok(())
        }
        Scenario::FusedAndHeldCombinationsAgreeOnRowsOrderAndErrorPrecedence => {
            fused_and_held_combinations_agree_on_rows_order_and_error_precedence();
            Ok(())
        }
        Scenario::AnEmptyLeftDoesNotHideAFailingRightDifferentlyWhenFused => {
            an_empty_left_does_not_hide_a_failing_right_differently_when_fused();
            Ok(())
        }
        Scenario::CompetingFaultsReachTheSameSideFirstWhenFused => {
            competing_faults_reach_the_same_side_first_when_fused();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
