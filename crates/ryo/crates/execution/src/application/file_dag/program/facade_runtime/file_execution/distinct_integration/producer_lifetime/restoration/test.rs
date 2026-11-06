//! Real restored-token consumers keep backing and terminal results through outer delivery exits.
use super::super::super::*;
use super::fixtures::{BoundSupply, CapturedProject, Exit};
use crate::application::file_dag::program::facade_runtime::{
    ProjectRootDeclaration, evaluate_roots,
};
use crate::application::file_dag::program::protocol::types::ProjectBatch;
use crate::domain::coverage::types::Partials;
use ryo_values::domain::rows::errors::TableModelError;

fn exercise(exit: Exit) -> Result<(), OperatingError> {
    let mut symbols = Symbols::default();
    let captured = CapturedProject::new(exit, &mut symbols)?;
    let partials = Partials::default();
    let owner = Rc::clone(captured.phase.owner());
    assert!(!owner.derived.index_observation().available);
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::Required,
    };
    let declarations = (1..5)
        .map(|at| ProjectRootDeclaration {
            rule: RuleIndex(at),
            demand,
        })
        .collect::<Vec<_>>();
    let runs = (1..5)
        .map(|at| {
            captured.open(
                RuleIndex(at),
                &partials,
                &mut symbols,
                if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
                    BoundSupply::Refused
                } else {
                    BoundSupply::Restored
                },
            )
        })
        .collect::<Vec<_>>();
    let guards = std::cell::Cell::new(0);
    let batch = evaluate_roots(runs, &declarations, &mut symbols, |rule| {
        guards.set(guards.get() + 1);
        if rule == RuleIndex(4) && matches!(exit, Exit::CancelledDelivery) {
            Err(OperatingError::NotRead)
        } else {
            Ok(())
        }
    });
    let outcomes = match batch {
        ProjectBatch::Complete(outcomes) => {
            assert!(
                !matches!(exit, Exit::CancelledDelivery),
                "cancellation must refuse its actual delivery"
            );
            outcomes
        }
        ProjectBatch::Refused { completed, error } => {
            assert!(matches!(exit, Exit::CancelledDelivery));
            assert!(matches!(error, OperatingError::NotRead));
            assert_eq!(
                completed.len(),
                3,
                "earlier delivered aliases survive later refusal"
            );
            completed
        }
    };
    let counts = captured.phase.readiness_counts();
    let builds = owner.derived.counts();
    let working = owner.derived.working_bytes();
    assert!(counts.evaluations.0 > 0);
    assert!(
        builds.0 > 0,
        "the actual restored population acquires required equality backing"
    );
    assert!(
        working > 0,
        "external obligations retain the required restored producer lease"
    );
    assert_eq!(owner.derived.retained_bytes(), 0);
    let backing = owner
        .derived
        .view_weak(ryo_derived::domain::contract::types::DerivedKind::Equality)
        .expect("actual project equality backing");
    assert!(backing.upgrade().is_some());
    let mut transferred = vec![];
    for (at, outcome) in outcomes.into_iter().enumerate() {
        let outcome = outcome?;
        match outcome.emit {
            RootOutcome::Complete(table) => transferred.push(table),
            RootOutcome::Failed(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 99,
            })) => {
                assert_eq!(at, 3);
                assert!(matches!(exit, Exit::SemanticFailure));
            }
            RootOutcome::Failed(OperatingError::WrongBinding) => {
                assert_eq!(at, 3);
                assert!(matches!(exit, Exit::RefusedBinding));
            }
            _ => panic!("{exit:?}: actual restored delivery lost its terminal result"),
        }
    }
    assert_eq!(
        transferred[0], transferred[1],
        "duplicate named obligations deliver the same canonical rows"
    );
    assert_eq!(transferred[0].count, RowCount(2));
    for at in [1, 2, 4] {
        if at == 4 && matches!(exit, Exit::CancelledDelivery) {
            continue;
        }
        let mut run = captured.open(
            RuleIndex(at),
            &partials,
            &mut symbols,
            if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
                BoundSupply::Refused
            } else {
                BoundSupply::Restored
            },
        )?;
        for _ in 0..2 {
            let replay = run.rule(RuleIndex(at), demand, &mut symbols)?;
            match (at, exit, replay.emit) {
                (
                    4,
                    Exit::SemanticFailure,
                    RootOutcome::Failed(OperatingError::Table(TableModelError::NoSuchColumn {
                        index: 99,
                    })),
                )
                | (4, Exit::RefusedBinding, RootOutcome::Failed(OperatingError::WrongBinding)) => {}
                (_, _, RootOutcome::Complete(table)) => assert!(table.count.0 > 0),
                _ => panic!(
                    "{exit:?}: repeated project delivery must preserve its exact terminal outcome"
                ),
            }
            assert_eq!(
                captured.phase.readiness_counts(),
                counts,
                "replay cannot schedule a terminal restored root"
            );
            assert_eq!(
                owner.derived.counts(),
                builds,
                "replay cannot reacquire a restored view"
            );
            assert_eq!(owner.derived.working_bytes(), working);
            assert!(
                backing.upgrade().is_some(),
                "another duplicate obligation must retain its actual backing"
            );
        }
    }
    captured.phase.finish()?;
    assert_eq!(owner.derived.working_bytes(), 0);
    assert_eq!(owner.derived.owned_bytes(), 0);
    assert!(
        backing.upgrade().is_none(),
        "last actual phase release drops restored producer backing"
    );
    assert_eq!(
        owner.derived.counts(),
        builds,
        "finish cannot dispatch or acquire work"
    );
    assert_eq!(
        transferred[0].count,
        RowCount(2),
        "transferred rows outlive phase retirement"
    );
    Ok(())
}

#[test]
fn restored_named_duplicate_delivery_retains_real_leases_through_replay_and_failure() {
    for exit in [Exit::Complete, Exit::SemanticFailure, Exit::RefusedBinding] {
        exercise(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}

#[test]
fn restored_named_cancellation_preserves_earlier_duplicate_deliveries_and_releases_phase() {
    exercise(Exit::CancelledDelivery).expect("actual restored delivery cancellation");
}
