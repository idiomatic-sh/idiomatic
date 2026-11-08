//! Named captures keep their identity; fused and held programs agree on rows, order and faults.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::{Combine, Feed, Fusion};
use super::super::fixtures::semantics::{Derived, MeaningSnapshot, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_execution_domain::domain::executable_program::fusion::types::{Barrier, Fusibility, Ineligible};

#[test]
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

/// What one run of a combining program decided: rows or error, and the faults it reached.
#[derive(Debug, PartialEq)]
struct Decision {
    answer: String,
    reached: MeaningSnapshot,
}
fn decide(
    combine: Combine,
    left: Feed,
    right: Feed,
    fusion: Fusion,
) -> Result<Decision, OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let program = Program::new(
        Authored::combined(combine, left, right, fusion),
        Population::Three,
        Residency::Stable,
        meanings.clone(),
        Arc::new(Derived::new()),
    )?;
    // The projection after each head is the only thing the second reader keeps apart.
    let compiled = program.prepared.program();
    for head in [0usize, 3] {
        let projection = compiled.rules()[0].original_steps[head + 1]
            .as_ref()
            .map_err(Clone::clone)?
            .logical;
        let class = compiled.fusibility(projection);
        match fusion {
            Fusion::Fused => assert_eq!(class, Some(Fusibility::Fusable), "{combine:?}"),
            Fusion::Held => assert_eq!(
                class,
                Some(Fusibility::Barrier(Barrier::SharedConsumer)),
                "{combine:?}"
            ),
        }
    }
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let answer = match file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit {
        RootOutcome::Complete(table) => format!("complete {:?} {}", table.cells, table.count.0),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        _ => "other".to_owned(),
    };
    file.finish()?;
    let mut reached = meanings.snapshot();
    // Projection calls are the continuation itself; every other call is observable order.
    reached.projects = super::super::fixtures::semantics::Calls(0);
    Ok(Decision { answer, reached })
}

#[test]
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

#[test]
fn an_empty_left_does_not_hide_a_failing_right_differently_when_fused() {
    for combine in Combine::ALL {
        let fused = decide(combine, Feed::Empty, Feed::Failing, Fusion::Fused).unwrap();
        let held = decide(combine, Feed::Empty, Feed::Failing, Fusion::Held).unwrap();
        assert_eq!(fused.answer, held.answer, "{combine:?}");
        assert_eq!(fused.reached.failed, held.reached.failed, "{combine:?}");
    }
}

#[test]
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
